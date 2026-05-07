//! Daemon control module: responsible for managing the lifecycle of the daemon
//! process, including starting, stopping, and checking its status.

use crate::client::{Error, Result};
use p2p_chat::{paths, pid, socket};
use State::*;

use nix::{sys::signal, unistd::Pid};

pub(super) enum State {
  Running { pid: i32 },
  NotRunning,
  Corrupted(Error),
  StateUnknown(Error),

  ProcessCreated,
  StopRequested,
}

// == Control functions for managing the daemon process ==

pub(super) fn create(daemon_log_level: &str) -> Result<State> {
  match status() {
    NotRunning => {}
    Running { pid } => return Ok(Running { pid }),
    Corrupted(err) | StateUnknown(err) => {
      log::debug!(
        "daemon::create() failed: status() reported a problem with daemon: {err:?}"
      );
      return Err(Error::other(err));
    }
    _ => unreachable!("other states are not expected from the callee"),
  };

  // Get current binary path to spawn the same binary with a hidden flag
  // that triggers the real daemon initialization code.
  let exe = std::env::current_exe()
    .inspect_err(|err| {
      log::debug!("daemon::create() failed: current_exe() couldn't to get executable path: {err:?}");
    })
    .map_err(|err| Error::other(err))?;

  // Configure command to run the binary with the hidden flag. Redirect stdio to null
  // for daemon.
  let mut command = std::process::Command::new(exe);
  command
    // Trigger hidden flag to call real daemon.
    .arg(format!(
      "--{}",
      p2p_chat::cli_interface::INTERNAL_DAEMON_INIT_FLAG
    ))
    .env("P2PCHAT_DAEMON_LOG_LEVEL", daemon_log_level)
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());

  // Detach from controlling terminal using standard new session strategy.
  // std::process's implementation of setsid() is still nightly-only feature,
  // so I use nix crate's alternative.
  unsafe {
    use std::os::unix::process::CommandExt;
    command.pre_exec(|| {
      nix::unistd::setsid().map_err(|e: nix::errno::Errno| {
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
      })?;

      Ok(())
    });
  }

  // Run the spawn daemon process command.
  let child = command
    .spawn()
    .inspect_err(|err| {
      log::debug!("daemon::create() failed: command.spawn() failed: {err:?}");
    })
    .map_err(|err| Error::other(err))?;

  log::info!(
    "Daemon proccess spawned with PID {}. See daemon logs for details",
    child.id()
  );

  Ok(ProcessCreated)
}

pub(super) fn destroy() -> Result<State> {
  let pid = match status() {
    Running { pid } => pid,
    NotRunning => return Ok(NotRunning),
    Corrupted(err) | StateUnknown(err) => {
      log::debug!(
        "daemon::destroy() failed: status() reported a problem with daemon: {err:?}"
      );
      return Err(Error::other(err));
    }
    _ => unreachable!("other states are not expected from the callee"),
  };

  signal::kill(Pid::from_raw(pid), signal::Signal::SIGTERM)
    .inspect(|_| {
      log::debug!("daemon::destroy() sent SIGTERM to daemon process with PID {pid}")
    })
    .inspect_err(|err| {
      log::debug!("daemon::destroy() failed to stop daemon with PID {pid}: {err:?}");
    })
    .map_err(|err| Error::other(err))?;

  log::info!("Daemon with PID {pid} was sent stop signal");

  Ok(StopRequested)
}

pub(super) fn status() -> State {
  let pid_fp = paths::daemon_pidfile();
  let mut pid: Option<i32> = None;

  match pid::read(&pid_fp) {
    Ok(p) => {
      if is_process_alive(p) {
        pid = Some(p.into());
        log::debug!("status() found live daemon process with PID {p}");
      } else {
        log::debug!(
          "status() found daemon PID file with PID {p}, but process \
           is not alive, so starting cleanup procedure"
        );
        match pid::cleanup(&pid_fp) {
          Ok(()) => {}
          Err(err) => {
            if err.kind() == pid::ErrorKind::RemovePidFile {
              log::warn!(
                "During cleanup found stale daemon PID-file (PID {p}), but \
                 failed to remove it. This may cause problems with future daemon \
                 creation attempts until the file is removed manually"
              );
            } else if err.kind() == pid::ErrorKind::RemoveParentDir {
              log::warn!(
                "During cleanup found stale daemon PID-file (PID {p}) and \
                 removed it, but failed to remove parent directory. This may cause \
                 problems with future daemon creation attempts until the file is \
                 removed manually"
              );
            } else {
              unreachable!("other errors are not expected from the callee");
            }
          }
        }
      }
    }
    Err(err) => {
      if err.kind() == pid::ErrorKind::PidFileNotFound {
        log::debug!(
          "status() didn't find PID file, so assume daemon is not running and \
           continue with the rest of status() function"
        );
      } else if err.kind() == pid::ErrorKind::ReadFromPidFile
        || err.kind() == pid::ErrorKind::InvalidPidFileContent
      {
        log::error!("Daemon status check could not determine state: {err}");
        log::debug!(
          "status() run pid::read() reported problem with daemon PID file: {err:?}"
        );
        return StateUnknown(Error::other(err));
      } else {
        unreachable!("other errors are not expected from the callee");
      }
    }
  }

  let socket_path = paths::daemon_socket();
  match socket_path.exists() {
    true => {
      if let None = pid {
        log::debug!(
          "status() found daemon socket file at expected path \
           but no live daemon process found, so starting cleanup procedure"
        );
        match socket::cleanup(&socket_path) {
          Ok(()) => {
            log::info!(
              "During cleanup found stale daemon socket file and removed it"
            );
          }
          Err(err) => {
            if err.kind() == socket::ErrorKind::RemoveSocketFile {
              log::warn!(
                "During cleanup found stale daemon socket file, but \
                 failed to remove it. This may cause problems with future daemon \
                 creation attempts until the file is removed manually"
              );
            } else if err.kind() == socket::ErrorKind::RemoveParentDir {
              log::warn!(
                "During cleanup found stale daemon socket file and removed it, but \
                 failed to remove parent directory. This may cause problems with \
                 future daemon creation attempts until the file is removed manually"
              );
            } else {
              unreachable!("other errors are not expected from the callee");
            }
          }
        }
      }
    }
    false => {
      if let Some(_) = pid {
        log::error!(
          "Daemon status check reported corrupted state: found live daemon \
           process but no socket file at expected path"
        );
        log::debug!(
          "status() found daemon PID file with live process, but \
           no socket file at expected path {:?}, so assume daemon is corrupted",
          paths::daemon_socket()
        );
        return Corrupted(Error::other(
          "daemon process is alive but socket file is missing",
        ));
      }
    }
  }

  match pid {
    Some(pid) => Running { pid },
    None => NotRunning,
  }
}

// == Helpers ==

fn is_process_alive(pid: nix::unistd::Pid) -> bool {
  match signal::kill(pid, None) {
    Ok(()) => true,
    // Process exists, but we don’t have permission to send a signal to it.
    Err(nix::errno::Errno::EPERM) => todo!(
      "Daemon process with PID {pid} exists but we don't have permission to 
       signal it. Consider implementing some strategy for this case, e.g. 
       checking if it's actually our daemon process or not"
    ),
    // Process doesn't exist.
    Err(nix::errno::Errno::ESRCH) => false,
    // Some other error.
    Err(_) => todo!(
      "Failed to check if process with PID {pid} is alive; reason unknown. \
       Consider implementing some strategy for this case."
    ),
  }
}
