mod daemon;
mod ipc;
mod message;

use anyhow::Result;
use daemon::DaemonState;
use ipc::socket_path;
use message::{Command, IpcMessage, Response};
use std::fs;
use std::os::unix::net::UnixListener;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
  // Initialize logging
  tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

  info!("Starting P2P Chat Daemon");

  // Clean up old socket file if it exists
  let socket_path = socket_path();
  let _ = fs::remove_file(socket_path);

  // Create Unix socket listener
  let listener = UnixListener::bind(socket_path)?;
  info!("Socket listener bound to {}", socket_path);

  // Initialize daemon state
  let daemon_state = Arc::new(DaemonState::new().await?);
  let node_id = daemon_state.node_id();
  info!("Iroh node started: {}", node_id);
  let addr = daemon_state.endpoint_addr();
  info!("Endpoint address: {:?}", addr);
  info!("Local endpoints: {:?}", addr.ip_addrs().collect::<Vec<_>>());

  // Spawn task to accept incoming Iroh connections
  let daemon_state_accept = Arc::clone(&daemon_state);
  tokio::spawn(async move {
    loop {
      match daemon_state_accept.endpoint.accept().await {
        Some(incoming) => {
          let daemon_state = Arc::clone(&daemon_state_accept);
          tokio::spawn(async move {
            match incoming.await {
              Ok(conn) => {
                let peer_id = conn.remote_id().to_string();
                info!("Accepted connection from peer: {}", peer_id);
                
                // Handle the connection
                if let Err(e) = daemon_state.handle_incoming_connection(conn).await {
                  error!("Error handling incoming connection: {}", e);
                }
              }
              Err(e) => {
                error!("Failed to accept connection: {}", e);
              }
            }
          });
        }
        None => {
          info!("Endpoint closed, stopping accept loop");
          break;
        }
      }
    }
  });

  // Handle client connections
  info!("Listening for CLI connections...");
  for stream in listener.incoming() {
    match stream {
      Ok(stream) => {
        let daemon_state = Arc::clone(&daemon_state);
        let msg_rx = daemon_state.msg_broadcast.subscribe();
        task::spawn_blocking(move || {
          if let Err(e) = handle_client(stream, daemon_state, msg_rx) {
            error!("Client handler error: {}", e);
          }
        });
      }
      Err(e) => {
        error!("Connection error: {}", e);
      }
    }
  }

  Ok(())
}

/// Handle a single client connection
fn handle_client(
  stream: std::os::unix::net::UnixStream,
  daemon_state: Arc<DaemonState>,
  mut msg_rx: broadcast::Receiver<message::Message>,
) -> Result<()> {
  use std::io::Read;

  let runtime = tokio::runtime::Runtime::new()?;

  info!("Client connected");
  let stream_clone = stream.try_clone()?;
  let mut reader = stream_clone;
  let mut writer = stream.try_clone()?;

  // Spawn thread to forward incoming messages to this CLI client
  let writer_for_msgs = stream.try_clone()?;
  std::thread::spawn(move || {
    let mut writer = writer_for_msgs;
    loop {
      match msg_rx.blocking_recv() {
        Ok(msg) => {
          let response = IpcMessage::Response(Response::Message(msg));
          if let Err(e) = ipc::send_message(&mut writer, &response) {
            error!("Failed to send message to CLI: {}", e);
            break;
          }
        }
        Err(broadcast::error::RecvError::Lagged(n)) => {
          error!("Message receiver lagged by {}", n);
        }
        Err(broadcast::error::RecvError::Closed) => {
          break;
        }
      }
    }
  });

  loop {
    // Read message from client
    let mut buffer = [0u8; 8192];
    match reader.read(&mut buffer) {
      Ok(0) => {
        info!("Client disconnected");
        break;
      }
      Ok(n) => {
        let data = String::from_utf8_lossy(&buffer[..n]);
        let trimmed = data.trim();

        if trimmed.is_empty() {
          continue;
        }

        // Parse command
        match serde_json::from_str::<IpcMessage>(trimmed) {
          Ok(IpcMessage::Command(cmd)) => {
            let response = runtime.block_on(async {
              match cmd {
                Command::Status => daemon_state.get_status().await,
                Command::ListPeers => Response::Peers(daemon_state.list_peers().await),
                Command::Connect { endpoint } => {
                  match daemon_state.connect_peer(&endpoint).await {
                    Ok(peer_id) => Response::Result {
                      success: true,
                      message: format!("Connected to peer: {}", peer_id),
                    },
                    Err(e) => Response::Error(format!("Connection failed: {}", e)),
                  }
                }
                Command::Send { peer_id, content } => {
                  let msg = message::Message::new(daemon_state.node_id(), content);
                  match daemon_state.send_message_to_peer(&peer_id, &msg).await {
                    Ok(_) => Response::Result {
                      success: true,
                      message: format!("Message sent to {}", peer_id),
                    },
                    Err(e) => Response::Error(format!("Failed to send message: {}", e)),
                  }
                }
                Command::Quit => {
                  return Response::Result {
                    success: true,
                    message: "Goodbye".to_string(),
                  };
                }
              }
            });

            let ipc_response = IpcMessage::Response(response.clone());
            if let Err(e) = ipc::send_message(&mut writer, &ipc_response) {
              error!("Failed to send response: {}", e);
              break;
            }

            if let Response::Result {
              message,
              success: true,
            } = response
            {
              if message == "Goodbye" {
                break;
              }
            }
          }
          Ok(IpcMessage::Response(_)) => {
            error!("Daemon received Response from CLI, ignoring");
          }
          Err(e) => {
            error!("Failed to parse command: {}", e);
            let error_response = IpcMessage::Response(Response::Error(format!("Parse error: {}", e)));
            let _ = ipc::send_message(&mut writer, &error_response);
          }
        }
      }
      Err(e) => {
        error!("Read error: {}", e);
        break;
      }
    }
  }

  Ok(())
}
