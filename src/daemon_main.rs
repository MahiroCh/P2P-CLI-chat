mod daemon;
mod ipc;
mod types;

use daemon::DaemonState;
use ipc::socket_path;
use types::{Command, IpcMessage, Response};
use std::fs;
use std::os::unix::net::UnixListener;
use std::sync::Arc;

#[tokio::main]
async fn main() {
  println!("Starting P2P Chat Daemon");

  let socket_path = socket_path();
  let _ = fs::remove_file(socket_path);

  let listener = UnixListener::bind(socket_path).expect("Failed to bind socket");
  println!("Socket: {}", socket_path);

  let daemon_state = Arc::new(DaemonState::new().await.expect("Failed to initialize daemon"));
  let node_id = daemon_state.node_id();
  println!("Node ID: {}", node_id);

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
                println!("Accepted connection from: {}", peer_id);
                let _ = daemon_state.handle_incoming_connection(conn).await;
              }
              Err(e) => eprintln!("Failed to accept connection: {}", e),
            }
          });
        }
        None => break,
      }
    }
  });

  println!("Listening for CLI connections...");
  for stream in listener.incoming() {
    match stream {
      Ok(stream) => {
        let daemon_state = Arc::clone(&daemon_state);
        tokio::task::spawn_blocking(move || {
          let _ = handle_client(stream, daemon_state);
        });
      }
      Err(e) => eprintln!("Connection error: {}", e),
    }
  }
}

fn handle_client(
  stream: std::os::unix::net::UnixStream,
  daemon_state: Arc<DaemonState>,
) -> Result<(), Box<dyn std::error::Error>> {
  use std::io::Read;

  let runtime = tokio::runtime::Runtime::new()?;
  let mut reader = stream.try_clone()?;
  let mut writer = stream.try_clone()?;

  loop {
    let mut buffer = [0u8; 8192];
    match reader.read(&mut buffer) {
      Ok(0) => break,
      Ok(n) => {
        let data = String::from_utf8_lossy(&buffer[..n]);
        let trimmed = data.trim();

        if trimmed.is_empty() {
          continue;
        }

        match serde_json::from_str::<IpcMessage>(trimmed) {
          Ok(IpcMessage::Command(cmd)) => {
            let response = runtime.block_on(async {
              match cmd {
                Command::Connect { endpoint } => {
                  match daemon_state.connect_peer(&endpoint).await {
                    Ok(peer_id) => Response::Ok(format!("Connected to: {}", peer_id)),
                    Err(e) => Response::Error(format!("Connection failed: {}", e)),
                  }
                }
                Command::Send { peer_id, content } => {
                  let msg = types::Message::new(daemon_state.node_id(), content);
                  match daemon_state.send_message_to_peer(&peer_id, &msg).await {
                    Ok(_) => Response::Ok("Message sent".to_string()),
                    Err(e) => Response::Error(format!("Failed: {}", e)),
                  }
                }
              }
            });

            let ipc_response = IpcMessage::Response(response);
            ipc::send_message(&mut writer, &ipc_response)?;
          }
          Ok(IpcMessage::Response(_)) => {}
          Err(e) => eprintln!("Parse error: {}", e),
        }
      }
      Err(e) => {
        eprintln!("Read error: {}", e);
        break;
      }
    }
  }

  Ok(())
}
