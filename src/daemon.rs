//! Daemon process for p2p chat application.

mod error;

use error::Result;
pub use error::{Error, ErrorKind};
use p2p_chat::{logger, paths, pid, socket};

use tokio::{
  net::UnixListener as TokioUnixListener,
  signal::unix::{signal as tokio_signal, Signal as TokioSignal, SignalKind},
};

// == Run daemon ==

struct DaemonCleanupGuard;
impl Drop for DaemonCleanupGuard {
  fn drop(&mut self) {
    if let Err(err) = pid::cleanup(&paths::daemon_pidfile()) {
      log::warn!("Failed to clean up daemon PID file on daemon shutdown: {err}");
      log::debug!("drop() of DaemonCleanupGuard failed to clean up daemon PID file: {err:?}");
    }

    if let Err(err) = socket::cleanup(&paths::daemon_socket()) {
      log::warn!("Failed to clean up daemon socket on daemon shutdown: {err}");
      log::debug!("drop() of DaemonCleanupGuard failed to clean up daemon socket: {err:?}");
    }
  }
}

pub fn run() -> std::result::Result<(), ()> {
  if let Err(err) = logger::init_daemon_logger() {
    eprint!("Failed to start daemon: ");
    // Logger is not initialized so fallback to printing error details to stderr.
    eprintln!("logger initialization error: {err}");
    return Err(());
  }

  match handle_daemon_init() {
    Ok(()) => {}
    Err(err)
      if matches!(
        err.kind(),
        ErrorKind::PidFileCreationFailed
          | ErrorKind::SocketCreationFailed
          | ErrorKind::SignalHandlerFailed
      ) =>
    {
      eprintln!(
        "Daemon is created but failed to initialize crucial components to run. \
         See logs for more info"
      );
      log::error!(
        "Daemon is created but failed to run because of socket of \
         pid file creation failure: {err}"
      );
    }
    Err(err) => {
      eprintln!("Failed to run daemon process. See logs for more info");
      log::error!("Daemon startup or runtime failed: {err}");
      return Err(());
    }
  }

  Ok(())
}

#[tokio::main]
async fn handle_daemon_init() -> Result<()> {
  let _guard = DaemonCleanupGuard;

  pid::create(&paths::daemon_pidfile(), &pid::this_proc_pid())
    .inspect_err(|err| {
      log::debug!(
        "pid::create() in daemon::run() failed to create daemon PID file: {err:?}"
      );
    })
    .map_err(|err| Error::new(ErrorKind::PidFileCreationFailed, err))?;

  let conn_listener = socket::create(&paths::daemon_socket())
    .inspect_err(|err| {
      log::debug!(
        "socket::create() in daemon::run() failed to create daemon socket: {err:?}"
      );
    })
    .map_err(|err| Error::new(ErrorKind::SocketCreationFailed, err))?;

  let mut sigterm = tokio_signal(SignalKind::terminate())
    .inspect_err(|err| {
      log::debug!(
        "tokio::signal::unix::signal() in daemon::run() failed to \
         create daemon signal handler: {err:?}"
      );
    })
    .map_err(|err| Error::new(ErrorKind::SignalHandlerFailed, err))?;

  crate::daemon::logic(&conn_listener, &mut sigterm)
    .await
    .inspect_err(|err| {
      log::debug!(
        "daemon::logic() in daemon::run() failed during daemon main loop: {err:?}"
      );
    })?;

  Ok(())
}

// == Daemon business logic ==

// TODO: This one is kinda messy right now, needs to be refactored.
async fn logic(
  conn_listener: &TokioUnixListener,
  sigterm: &mut TokioSignal,
) -> Result<()> {
  // TODO: Make this truly asynchronous, e.g. by spawning a task and so on.
  'daemon_loop: loop {
    tokio::select! {
      _ = sigterm.recv() => {
        log::info!("Received SIGTERM, shutting down daemon");
        break 'daemon_loop;
      },

      accepted = conn_listener.accept() => {
        let (mut stream, _) = accepted.map_err(|err| {
          log::debug!("Daemon listener failed to accept connection: {err}");
          Error::new(ErrorKind::ConnectionAcceptFailed, err)
        })?;

        loop {
          tokio::select! {
            _ = sigterm.recv() => {
              log::info!("Received SIGTERM, shutting down daemon");
              break 'daemon_loop;
            },

            cmd = socket::read_data(&mut stream) => {
              let cmd = match cmd {
                Ok(value) => {
                  log::info!("Received command from peer: {value}");
                  value
                },
                Err(err) if matches!(err.kind(), socket::ErrorKind::ConnectionAborted) => {
                  log::debug!(
                    "Daemon listener lost connection while reading peer command \
                     (most likely because client process terminated): {err:?}");
                  break;
                },
                Err(err) => {
                  log::debug!("Daemon listener failed to read peer command: {err:?}");
                  break;
                },
              };

              let answer = String::from(format!("{}", cmd));
              match socket::write_data(&mut stream, &answer).await {
                Ok(_) => {
                  log::info!("Sent response to peer command back to client: {answer}");
                },
                Err(err) if matches!(err.kind(), socket::ErrorKind::ConnectionAborted) => {
                  log::debug!("Daemon listener lost connection while writing response: {err:?}");
                  break;
                },
                Err(err) => {
                  log::debug!("Daemon listener failed to write response: {err:?}");
                  break;
                }
              }
            }
          }
        }
      }
    }
  }

  Ok(())
}
