//! Daemon process for p2p chat application.

mod client_session;
mod error;
mod logger;
mod network;

use client_session::Session as ClientSession;
use error::*;
use network::NetworkHandle;
use p2p_chat::{
  paths, pid, socket,
  schemas::{ClientRequest, DaemonEvent, NetEvent},
};

use tokio::signal::unix::{signal, Signal as SignalListener, SignalKind};
use tokio::sync::mpsc;

type Result<T> = std::result::Result<T, Error>;

pub const DEFAULT_LOG_LEVEL: &str = "info";

// == Daemon logic ==

struct DaemonCleanupGuard;
impl Drop for DaemonCleanupGuard {
  fn drop(&mut self) {
    if let Err(err) = pid::cleanup(&paths::daemon_pidfile()) {
      log::warn!("Failed to clean up daemon PID file on shutdown: {err}");
      log::debug!(
        "CleanupGuard failed to clean up daemon PID file on shutdown: {err:?}"
      );
    }
    if let Err(err) = socket::cleanup(&paths::daemon_socket()) {
      log::warn!("Failed to clean up daemon socket on shutdown: {err}");
      log::debug!(
        "CleanupGuard failed to clean up daemon socket on shutdown: {err:?}"
      );
    }
  }
}

#[tokio::main]
pub(super) async fn run() -> Result<()> {
  // Retrieve log level from environment variable.
  // TODO: Consider other methods of passing log level to daemon.
  let daemon_log_level = std::env::var("P2PCHAT_DAEMON_LOG_LEVEL")
    .unwrap_or_else(|_| DEFAULT_LOG_LEVEL.to_owned());

  logger::init(daemon_log_level).inspect_err(|err| {
    // Logger is not initialized so fallback to printing error details to stderr.
    eprint!("Failed to start daemon due to logger initialization error: {err}");
  })?;

  let _guard = DaemonCleanupGuard;

  let (client_session, network_session, network_events_rx, mut signal_listener) = init_daemon_components()
    .await
    .inspect_err(|err| {
      log::error!("Failed to initialize daemon components: {err}");
      eprintln!("Daemon failed to initialize crucial component to run: {err}");
    })?;

  let (client_requests_tx, client_requests_rx) = 
    mpsc::channel::<p2p_chat::schemas::ClientRequest>(256);
  let (daemon_events_tx, daemon_events_rx) = 
    mpsc::channel::<p2p_chat::schemas::DaemonEvent>(256);

  let client_gateway_task_handle = tokio::spawn(client_gateway(
    client_session,
    client_requests_tx,
    daemon_events_rx,
  ));
  let logic_task_handle = tokio::spawn(logic(
    network_session,
    network_events_rx,
    client_requests_rx,
    daemon_events_tx,
  ));

  log::info!(
    "Daemon with PID {} initialized and ready to accept connections",
    pid::this_proc_pid()
  );

  match tokio::select! {
    _ = signal_listener.recv() => {
      log::info!("Daemon received termination signal, shutting down...");
      Ok(())
    },
    out = logic_task_handle => match out {
      Ok(Ok(())) => Ok(()),
      Ok(Err(err)) => Err(err),
      Err(join_err) => Err(Error::other(format!("daemon event router task failed: {join_err}"))),
    },
    out = client_gateway_task_handle => match out {
      Ok(Ok(())) => Err(Error::other("client gateway stopped unexpectedly")),
      Ok(Err(err)) => Err(err),
      Err(join_err) => Err(Error::other(format!("client gateway task failed: {join_err}"))),
    }
  } {
    Ok(()) => {
      // TODO: print and log what does this mean
      Ok(())
    }
    Err(err) => {
      if err.kind() == ErrorKind::ClientAcceptFailed {
        eprintln!("Daemon failed to accept client connection: {err}");
        log::error!("Daemon failed to accept client connection: {err}");
      } else {
        eprintln!("Daemon failed: {err}");
        log::error!("Daemon failed: {err}");
      }

      Err(err)
    }
  }
}

async fn logic(
  network: NetworkHandle,
  mut network_events_rx: mpsc::Receiver<NetEvent>,
  mut client_requests_rx: mpsc::Receiver<ClientRequest>,
  daemon_events_tx: mpsc::Sender<DaemonEvent>,
) -> Result<()> {
  loop {
    tokio::select! {
      Some(request) = client_requests_rx.recv() => {
        // Client sent a request; process it and send response back.
        if let Some(response) = handle_request(&network, request).await? {
          if let Err(err) = daemon_events_tx.send(response).await {
            log::warn!("failed to send response to client (client disconnected?): {err}");
            // Continue processing network events even if client isn't listening.
          }
        }
      }
      Some(ev) = network_events_rx.recv() => {
        // Unsolicited event from network (e.g., peer connected/disconnected, message arrived).
        // Forward to client if still connected.
        if let Err(err) = daemon_events_tx.send(DaemonEvent::from(ev)).await {
          log::warn!("failed to send network event to client (client disconnected?): {err}");
          // Continue processing network events even if client isn't listening.
        }
      }
      else => {
        break;
      }
    }
  }

  Ok(())
}

async fn client_gateway(
  mut session: ClientSession,
  client_request_tx: mpsc::Sender<ClientRequest>,
  mut daemon_events_rx: mpsc::Receiver<DaemonEvent>,
) -> Result<()> {
  loop {
    session.accept_client().await.map_err(|err| {
      log::error!("Failed to accept client connection: {err}");
      Error::new(ErrorKind::ClientAcceptFailed, err)
    })?;

    log::info!("Client connected");

    loop {
      tokio::select! {
        req = session.recv_client_request() => {
          match req {
            Ok(ClientRequest::Bye) => {
              log::info!("client said 'Bye', ending session");
              break;
            }
            Ok(request) => {
              client_request_tx.send(request).await.map_err(|err| {
                Error::other(format!("failed to forward client request to daemon: {err}"))
              })?;
            }
            Err(err) => {
              if err.kind() == ErrorKind::ClientAbortedConnection {
                log::info!("client disconnected: {err}");
                break;
              }
              log::error!("recv_client_request error, ending session: {err}");
              return Err(Error::other(err));
            }
          }
        }
        event = daemon_events_rx.recv() => {
          match event {
            Some(event) => {
              // Logic task sent a response; forward it to the client.
              if let Err(err) = session.send_event_to_client(&event).await {
                if err.kind() == ErrorKind::ClientAbortedConnection {
                  log::info!("client disconnected while writing response: {err}");
                  break;
                }
                log::error!("send_event_to_client error, ending session: {err}");
                return Err(Error::other(err));
              }
            }
            None => {
              // Logic task dropped the sender (daemon is shutting down).
              return Err(Error::other("daemon event channel closed unexpectedly"));
            }
          }
        }
      }
    }

    log::info!("Client disconnected; daemon continues running");
  }
}

// == Helpers ==

async fn handle_request(
  network: &NetworkHandle,
  request: ClientRequest,
) -> Result<Option<DaemonEvent>> {
  match request {
    ClientRequest::Bye => Ok(None),
    ClientRequest::MyID => Ok(Some(DaemonEvent::MyId {
      endpoint_id: network.my_ticket()?,
    })),
    ClientRequest::List => {
      let peers = network.list_peers().await;
      Ok(Some(DaemonEvent::PeerList { peers }))
    }
    ClientRequest::Connect { peer_id } => match network.connect(&peer_id).await {
      Ok(connected_id) => Ok(Some(DaemonEvent::Ok {
        info: format!("connected to {connected_id}"),
      })),
      Err(err) => Ok(Some(DaemonEvent::Error {
        message: format!("failed to connect: {err}"),
      })),
    },
    ClientRequest::Disconnect { peer_id } => {
      match network.disconnect(&peer_id).await {
        Ok(true) => Ok(Some(DaemonEvent::Ok {
          info: format!("disconnected from {peer_id}"),
        })),
        Ok(false) => Ok(Some(DaemonEvent::Error {
          message: format!("peer {peer_id} is not connected"),
        })),
        Err(err) => Ok(Some(DaemonEvent::Error {
          message: format!("disconnect failed: {err}"),
        })),
      }
    }
    ClientRequest::Send { peer_id, message } => {
      match network.send_message(&peer_id, &message).await {
        Ok(()) => {
          let timestamp_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
          Ok(Some(DaemonEvent::Ok {
            info: format!("sent to {peer_id} at {}", format_timestamp(timestamp_secs)),
          }))
        }
        Err(err) => Ok(Some(DaemonEvent::Error {
          message: format!("send failed: {err}"),
        })),
      }
    }
    _ => Ok(Some(DaemonEvent::Error {
      message: "unsupported request in this daemon version".to_owned(),
    })),
  }
}

fn format_timestamp(secs: i64) -> String {
  use chrono::{TimeZone, Utc, Local};
  
  match Utc.timestamp_opt(secs, 0) {
    chrono::LocalResult::Single(dt) => {
      dt.with_timezone(&Local).format("%H:%M:%S").to_string()
    },
    chrono::LocalResult::Ambiguous(dt, _) => {
      dt.with_timezone(&Local).format("%H:%M:%S").to_string()
    },
    chrono::LocalResult::None => "<invalid-time>".to_owned(),
  }
}

async fn init_daemon_components() -> Result<(
  ClientSession,
  NetworkHandle,
  mpsc::Receiver<NetEvent>,
  SignalListener,
)> {
  pid::create(&paths::daemon_pidfile(), &pid::this_proc_pid())
    .inspect_err(|err| {
      log::debug!(
        "init_daemon_components() failed to create daemon PID file: {err:?}"
      );
    })
    .map_err(|err| {
      Error::other(format!("failed to create daemon PID file: {err}"))
    })?;

  log::debug!("Daemon PID file created at {}", paths::daemon_pidfile().display());

  let session = ClientSession::new()
    .inspect_err(|err| {
      log::debug!("init_daemon_components() failed to create client connection listener: {err:?}");
    })
    .map_err(|err| Error::other(
      format!("failed to create client connection listener: {err}"))
    )?;

  log::debug!("Daemon client connection listener created at {}", paths::daemon_socket().display());

  let (network, net_events_rx) = NetworkHandle::new()
    .await
    .inspect_err(|err| {
      log::debug!(
        "init_daemon_components() failed to initialize networking stack: {err:?}"
      );
    })
    .map_err(|err| {
      Error::other(format!("failed to initialize networking stack: {err}"))
    })?;

  log::debug!("Daemon networking stack initialized successfully");

  let listener = signal(SignalKind::terminate())
    .inspect_err(|err| {
      log::debug!(
        "init_daemon_components() failed to create daemon signal handler: {err:?}"
      );
    })
    .map_err(|err| {
      Error::other(format!("failed to create daemon signal handler: {err}"))
    })?;

  log::debug!("Daemon signal handler created successfully");

  Ok((session, network, net_events_rx, listener))
}
