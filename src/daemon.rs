//! Daemon process for p2p chat application.

mod error;
mod session;
mod command_processor;

use error::{Error, ErrorKind, Result};
use p2p_chat::{logger, paths, pid, socket};
use session::{Connection, ConnectionsSession};

use std::collections::HashSet;
use tokio::{
  signal::unix::{signal, Signal as SignalListener, SignalKind},
  sync::mpsc,
};

// == Run daemon ==

struct DaemonCleanupGuard;
impl Drop for DaemonCleanupGuard {
  fn drop(&mut self) {
    if let Err(err) = pid::cleanup(&paths::daemon_pidfile()) {
      log::warn!("Failed to clean up daemon PID file on daemon shutdown: {err}");
      log::debug!(
        "drop() of DaemonCleanupGuard failed to clean up daemon PID file: {err:?}"
      );
    }

    if let Err(err) = socket::cleanup(&paths::daemon_socket()) {
      log::warn!("Failed to clean up daemon socket on daemon shutdown: {err}");
      log::debug!(
        "drop() of DaemonCleanupGuard failed to clean up daemon socket: {err:?}"
      );
    }
  }
}

#[tokio::main]
pub(super) async fn run() -> std::result::Result<(), ()> {
  if let Err(err) = logger::init_daemon_logger() {
    eprint!("Failed to start daemon: ");
    // Logger is not initialized so fallback to printing error details to stderr.
    eprintln!("logger initialization error: {err}");
    return Err(());
  }

  let _guard = DaemonCleanupGuard;

  let (conn_session, mut signal_listener) = match daemon_init_components() {
    Ok((conn_session, signal_listener)) => {
      log::info!(
        "Daemon with PID {} initialized successfully and ready to accept connections",
        pid::this_proc_pid()
      );
      (conn_session, signal_listener)
    }
    Err(err)
      if matches!(
        err.kind(),
        ErrorKind::PidFileCreationFailed
          | ErrorKind::SocketCreationFailed
          | ErrorKind::SignalHandlerFailed
      ) =>
    {
      eprintln!(
        "Daemon failed to initialize crucial components to run. \
         See logs for more info"
      );
      log::error!(
        "Daemon failed to run because of socket, \
         pid file, or signal handler creation failure: {err}"
      );

      return Err(());
    }
    Err(_) => unreachable!(),
  };

  match tokio::select! {
    _ = signal_listener.recv() => Ok(()),
    out = crate::daemon::accept_connections(&conn_session) => out
  } {
    Ok(()) => {
      println!("Daemon received termination signal, shutting down...");
      log::info!("Daemon received termination signal, shutting down...");
    }
    Err(err) => {
      eprintln!("Daemon failed during business logic execution. See logs for more info");
      log::error!("Daemon failed during business logic execution: {err}");
      return Err(());
    }
  }

  Ok(())
}

// == Daemon business logic ==

async fn accept_connections(conn_session: &ConnectionsSession) -> Result<()> {
  let (conn_finished_tx, mut conn_finished_rx) =
    mpsc::channel::<u32>(conn_session.max_connections as usize);
  let mut active_connection_ids: HashSet<u32> = HashSet::new();

  loop {
    tokio::select! {
      accept_result = conn_session.accept_connection() => {
        let connection = match accept_result {
          Ok(conn) => conn,
          Err(err) if matches!(err.kind(), ErrorKind::ConnectionAtCapacity) => {
            log::warn!(
              "Rejected new connection: maximum concurrent connections \
               ({}) reached", conn_session.max_connections);
            continue;
          }
          Err(err) => {
            log::error!("Failed to accept new connection in daemon: {err}");
            return Err(err);
          }
        };

        let mut client_id = 1;
        while active_connection_ids.contains(&client_id)
              && client_id < conn_session.max_connections {
          client_id += 1;
        }

        active_connection_ids.insert(client_id);
        log::info!(
          "Client (ID {}) connected (total active: {})",
          client_id,
          active_connection_ids.len()
        );

        let tx = conn_finished_tx.clone();
        tokio::spawn(async move {
          if let Err(err) = handle_connection(client_id, connection).await {
            log::error!("Client (ID {client_id}) closed connection with error {err}");
          }
          let _ = tx.send(client_id).await;
        });
      }
      Some(finished_client_id) = conn_finished_rx.recv() => {
        active_connection_ids.remove(&finished_client_id);
        log::info!(
          "Client (ID {}) closed connection with daemon (total active left: {})",
          finished_client_id,
          active_connection_ids.len()
        );
      }
    }
  }

  #[allow(unreachable_code)]
  // This is unreachable because the loop is infinite and the only way to stop
  // it is by receiving a termination signal for daemon by client. Monitoring
  // the signal arrival is happening in the caller function.
  Ok(())
}

async fn handle_connection(
  client_id: u32,
  mut connection: Connection,
) -> Result<()> {
  loop {
    let cmd = match connection.read_command().await {
      Ok(cmd) => {
        log::info!("Received command from client (ID {client_id}): {cmd:?}");
        cmd
      }
      Err(err) if matches!(err.kind(), ErrorKind::ClientAbortedConnection) => {
        log::info!(
          "Client (ID {client_id}) closed connection while daemon was waiting \
           for action command (client process terminated): {err:?}"
        );
        break;
      }
      Err(err) => {
        log::error!(
          "Failed to read action command from client (ID {client_id}) in daemon: {err}"
        );
        return Err(err);
      }
    };

    // TODO: Make this call async.
    match command_processor::process(&mut connection, client_id, &cmd).await {
      Ok(()) => {}
      Err(err) if matches!(err.kind(), ErrorKind::ClientAbortedConnection) => {
        log::info!(
          "Client (ID {client_id}) closed connection while daemon was writing \
           response (client process terminated): {err:?}"
        );
        break;
      }
      Err(err) => {
        log::error!("Client (ID {client_id}) command processor failed: {err}");
        return Err(err);
      }
    }
  }

  Ok(())
}

// == Helpers ==

fn daemon_init_components() -> Result<(ConnectionsSession, SignalListener)> {
  pid::create(&paths::daemon_pidfile(), &pid::this_proc_pid())
    .inspect_err(|err| {
      log::debug!(
        "pid::create() in daemon_init_components() failed to create daemon PID file: {err:?}"
      );
    })
    .map_err(|err| Error::new(ErrorKind::PidFileCreationFailed, err))?;

  let session = ConnectionsSession::new(16).inspect_err(|err| {
    log::debug!(
      "ConnectionsSession::new() in daemon_init_components() failed to \
       create daemon connection session: {err:?}"
    );
  })?;

  let listener = signal(SignalKind::terminate())
    .inspect_err(|err| {
      log::debug!(
        "tokio::signal::unix::signal() in daemon_init_components() failed to \
         create daemon signal handler: {err:?}"
      );
    })
    .map_err(|err| Error::new(ErrorKind::SignalHandlerFailed, err))?;

  Ok((session, listener))
}