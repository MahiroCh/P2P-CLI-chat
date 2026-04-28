//! Daemon control module: responsible for managing the lifecycle of the daemon 
//! process, including starting, stopping, and checking its status.

// NOTE: Consider implementing some way for daemon to kill itself if it 
// NOTE: idles without CLI connection for too long to avoid dangling process.
// NOTE: (which is not really bad as I know, but still...)

mod session;

use crate::client::{Error, ErrorKind, Result};
use p2p_chat::{socket, pid, paths, cli_schema::INTERNAL_DAEMON_INIT_FLAG};
pub use session::Session;

use nix::{sys::signal, unistd::Pid};

pub enum CreateRes {
	Started { pid: i32 },
	Running { pid: i32 },
}

// == Control functions for managing the daemon process ==

pub fn create() -> Result<CreateRes> {
	// TODO: If PID exists, it doesn't necessarily mean the process is 
	// TODO: our p2p-chat daemon. Some other proces might have taken the PID.
	// TODO: Implement this check.
	// NOTE: Stronger design for the future: pair this with a lockfile or Unix socket.
	// Cannot throw error in the current implementation.
	match status() {
		Ok(Status::NotRunning) => { /* Continue running create() */ },
		Ok(Status::Running { pid }) => return Ok(CreateRes::Running { pid }),
		Err(err) if matches!(err.kind(), ErrorKind::DaemonStateUnknown) => {
			log::debug!("status() inside create() failed to obtain daemon state: {err:?}");
			return Err(err);
		},
		Err(_) => unreachable!("other errors are not expected from the callee"),
	};

	// Get current binary path to spawn the same binary with a hidden flag 
	// that triggers the real daemon initialization code.
	let exe = std::env::current_exe().map_err(|err| {
		log::debug!("current_exe() failed to get executable path: {err:?}");
		Error::new(ErrorKind::DaemonStartFailed, err)
	})?;

	// Configure command to run the binary with the hidden flag. Redirect stdio to null 
	// for daemon.
	// NOTE: Consider later adding an option to redirect daemon logs to a file or
	// NOTE: another terminal window.
	let mut command = std::process::Command::new(exe);
	command
	   // Trigger hidden flag to call real daemon.
		.arg(format!("--{}", INTERNAL_DAEMON_INIT_FLAG))
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());
	
	// Detach from controlling terminal using standard new session strategy. 
	// std::process's implementation of setsid() is still nightly-only feature, 
	// so I use nix crate's alternative.
	unsafe {
		use std::os::unix::process::CommandExt;
		command.pre_exec(|| {
			nix::unistd::setsid().map_err(
				|e: nix::errno::Errno| 
				std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
			)?;
			
			Ok(())
		});
	}

	// Run the spawn command.
	let child = command.spawn()
		.map_err(|err| {
			log::debug!("command.spawn() failed: {err:?}");
			Error::new(ErrorKind::DaemonStartFailed, err)
		})?;

	// TODO: Consider implementing some waiting strategy for the daemon to be 
	// TODO: fully initialized, e.g. by waiting for the PID file or socket file to 
	// TODO: be created, or something else.

	Ok(CreateRes::Started { pid: child.id() as i32 })
}

pub enum DestroyRes {
	Destroyed { pid: i32 },
	NotRunning,
}

pub fn destroy() -> Result<DestroyRes> {
	let pid = match status() {
		Ok(Status::Running { pid }) => pid,
		Ok(Status::NotRunning) => return Ok(DestroyRes::NotRunning),
		Err(err) if matches!(err.kind(), ErrorKind::DaemonStateUnknown) => {
			log::debug!("status() inside destroy() failed to obtain daemon state");
			return Err(err);
		},
		Err(_) => unreachable!("other errors are not expected from the callee"),
	};

	match signal::kill(Pid::from_raw(pid), signal::Signal::SIGTERM) {
		Ok(()) => log::debug!("signal::kill() sent SIGTERM to daemon process with PID: {pid}"),
		Err(err) if err == nix::errno::Errno::EPERM => {
			log::debug!("signal::kill() permission denied to stop daemon with PID: {pid}");
			return Err(Error::new(ErrorKind::DaemonStopFailed, err));
		},
		Err(err) => {
			log::debug!("signal::kill() failed to stop daemon with PID: {pid}: {err}");
			return Err(Error::new(ErrorKind::DaemonStopFailed, err));
		},
	}

	Ok(DestroyRes::Destroyed { pid })
}

pub enum Status {
	Running { pid: i32 },
	NotRunning,
}

pub fn status() -> Result<Status> {
	let pid_fp = paths::daemon_pidfile();
	let mut pid: Option<i32> = None;

	match pid::read(&pid_fp) {
		Ok(p) => { 
			if is_process_alive(p) {
				pid = Some(p.into());
				log::debug!("During status() run found live daemon process with PID: {p}");
			} else {
				log::debug!(
					"During status() run found daemon PID file with PID {p}, but process \
					 is not alive, so starting cleanup procedure"
				);
				match pid::cleanup(&pid_fp) {
					Ok(()) => {},
					Err(err) if matches!(err.kind(), pid::ErrorKind::RemovePidFile) => {
						log::warn!(
							"During cleanup found stale daemon PID-file (PID {p}), but \
							 failed to remove it. This may cause problems with future daemon \
							 creation attempts until the file is removed manually"
						);
					},
					Err(err) if matches!(err.kind(), pid::ErrorKind::RemoveParentDir) => {
						log::warn!(
							"During cleanup found stale daemon PID-file (PID {p}) and \
							 removed it, but failed to remove parent directory. This may cause \
							 problems with future daemon creation attempts until the file is \
							 removed manually"
						);
					},
					Err(_) => unreachable!("other errors are not expected from the callee"),
				}
			}
		},
		Err(err) if matches!(err.kind(), pid::ErrorKind::PidFileNotFound) => { 
			log::debug!(
				"During status() run pid::read() returned PidFileNotFound error,so \
			   assume daemon is not running and continue with the rest of status() function"
			);
		},
		Err(err) if matches!(err.kind(), pid::ErrorKind::ReadFromPidFile) => {
			log::error!("Failed to read daemon PID file, daemon state is unknown");
			log::debug!("During status() run pid::read() failed to read from PID file: {err:?}");
			return Err(Error::new(ErrorKind::DaemonStateUnknown, err));
		},
		Err(err) if matches!(err.kind(), pid::ErrorKind::InvalidPidFileContent) => {
			log::error!("Content of the daemon PID file is of invalid format, daemon state is unknown");
			log::debug!("During status() run pid::read() returned InvalidPidFileContent error: {err:?}");
			return Err(Error::new(ErrorKind::DaemonStateUnknown, err));
		},
		Err(_) => unreachable!("other errors are not expected from the callee"),
	}

	let socket_path = paths::daemon_socket();
	match socket_path.exists() {
		true => {
			if let None = pid {
				log::debug!(
					"During status() run found daemon socket file at expected path \
					 but no live daemon process found, so starting cleanup procedure"
				);
				match socket::cleanup(&socket_path) {
					Ok(()) => {
						log::info!("During cleanup found stale daemon socket file and removed it");
					},
					Err(err) if matches!(err.kind(), socket::ErrorKind::RemoveSocketFile) => {
						log::warn!(
							"During cleanup found stale daemon socket file, but \
							 failed to remove it. This may cause problems with future daemon \
							 creation attempts until the file is removed manually"
						);
					},
					Err(err) if matches!(err.kind(), socket::ErrorKind::RemoveParentDir) => {
						log::warn!(
							"During cleanup found stale daemon socket file and removed it, but \
							 failed to remove parent directory. This may cause problems with \
							 future daemon creation attempts until the file is removed manually"
						);
					},
					Err(_) => unreachable!("other errors are not expected from the callee"),
				}
			}
		},
		false => {
			if let Some(_) = pid {
				log::error!(
					"Found live daemon process but no socket file at expected \
				   path which means that daemon is in corrupted state"
				);
				log::debug!(
					"During status() run found daemon PID file with live process, but \
					 no socket file at expected path, so assume daemon is corrupted and \
					 return DaemonCorrupted error"
				);
				// TODO: Consinder sending SIGTERM to the daemon.
				return Err(Error::from(ErrorKind::DaemonCorrupted));
			}
		}
	}

	match pid {
		Some(pid) => Ok(Status::Running { pid }),
		None => Ok(Status::NotRunning),
	}
}

// == Helpers ==

// TODO: This should be taken in account: if sending a null signal fails with 
// TODO: the error ESRCH, then we know the process doesn’t exist. If the call fails 
// TODO: with the error EPERM (meaning the process exists, but we don’t have permission 
// TODO: to send a signal to it) or succeeds (meaning we do have permission to send
// TODO: a signal to the process), then we know that the process exists.
fn is_process_alive(pid: nix::unistd::Pid) -> bool {
	match signal::kill(pid, None) {
		Ok(()) => true,
		// Process exists, but we don’t have permission to send a signal to it.
		Err(nix::errno::Errno::EPERM) => 
			todo!(
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
