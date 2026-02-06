mod ipc;
mod message;

use anyhow::Result;
use clap::{Parser, Subcommand};
use ipc::socket_path;
use message::{Command, IpcMessage, Response};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::Duration;
use tracing::info;

#[derive(Parser)]
#[command(name = "chat-cli")]
#[command(about = "P2P Chat CLI client", long_about = None)]
struct Args {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Connect to the daemon and enter interactive mode
  Connect,
  /// Get daemon status
  Status,
  /// List connected peers
  Peers,
  /// Connect to a remote peer
  #[command(arg_required_else_help = true)]
  Join {
    /// Remote peer endpoint
    endpoint: String,
  },
  /// Send a message to a peer
  #[command(arg_required_else_help = true)]
  Send {
    /// Peer ID
    peer_id: String,
    /// Message content
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    message: Vec<String>,
  },
  /// Disconnect from daemon
  Quit,
}

#[tokio::main]
async fn main() -> Result<()> {
  // Initialize logging
  tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

  let args = Args::parse();

  match args.command {
    Commands::Connect => {
      interactive_mode().await?;
    }
    Commands::Status => {
      send_command(Command::Status).await?;
    }
    Commands::Peers => {
      send_command(Command::ListPeers).await?;
    }
    Commands::Join { endpoint } => {
      send_command(Command::Connect { endpoint }).await?;
    }
    Commands::Send { peer_id, message } => {
      let content = message.join(" ");
      send_command(Command::Send { peer_id, content }).await?;
    }
    Commands::Quit => {
      send_command(Command::Quit).await?;
    }
  }

  Ok(())
}

/// Send a single command to the daemon and print response
async fn send_command(cmd: Command) -> Result<()> {
  let mut stream = connect_with_retry()?;

  // Send command
  let msg = IpcMessage::Command(cmd);
  ipc::send_message(&mut stream, &msg)?;

  // Read response
  let mut buffer = [0u8; 8192];
  match stream.read(&mut buffer) {
    Ok(0) => {
      println!("Connection closed by daemon");
    }
    Ok(n) => {
      let data = String::from_utf8_lossy(&buffer[..n]);
      match serde_json::from_str::<IpcMessage>(data.trim()) {
        Ok(IpcMessage::Response(resp)) => {
          print_response(resp);
        }
        Ok(IpcMessage::Command(_)) => {
          eprintln!("Unexpected Command message from daemon");
        }
        Err(e) => {
          eprintln!("Failed to parse response: {}", e);
        }
      }
    }
    Err(e) => {
      eprintln!("Failed to read response: {}", e);
    }
  }

  Ok(())
}

/// Interactive mode for continuous chat
async fn interactive_mode() -> Result<()> {
  println!("Connecting to daemon...");
  let stream = connect_with_retry()?;

  println!("Connected to daemon. Commands: help, join <endpoint>, send <peer_id> <message>, peers, status, quit");

  // Spawn thread to listen for incoming messages
  let stream_clone = stream.try_clone()?;
  let _listener_thread = thread::spawn(move || {
    let mut stream = stream_clone;
    let mut buffer = [0u8; 8192];
    loop {
      match stream.read(&mut buffer) {
        Ok(0) => break,
        Ok(n) => {
          let data = String::from_utf8_lossy(&buffer[..n]);
          if let Ok(IpcMessage::Response(resp)) = serde_json::from_str(data.trim()) {
            print_response(resp);
          }
        }
        Err(_) => break,
      }
    }
  });

  // Read user input and send commands
  let stdin = std::io::stdin();
  let mut stream = stream;
  let mut should_quit = false;

  loop {
    print!("> ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    stdin.read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
      continue;
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
      continue;
    }

    let cmd = match parts[0] {
      "help" => {
        println!("Commands:");
        println!("  help                          - Show this help");
        println!("  status                        - Show daemon status");
        println!("  peers                         - List connected peers");
        println!("  join <endpoint>               - Connect to a peer");
        println!("  send <peer_id> <message>      - Send message to peer");
        println!("  quit                          - Disconnect and exit");
        continue;
      }
      "status" => Command::Status,
      "peers" => Command::ListPeers,
      "join" => {
        if parts.len() < 2 {
          println!("Usage: join <endpoint>");
          continue;
        }
        Command::Connect {
          endpoint: parts[1].to_string(),
        }
      }
      "send" => {
        if parts.len() < 3 {
          println!("Usage: send <peer_id> <message>");
          continue;
        }
        let peer_id = parts[1].to_string();
        let content = parts[2..].join(" ");
        Command::Send { peer_id, content }
      }
      "quit" => {
        should_quit = true;
        Command::Quit
      }
      _ => {
        println!("Unknown command: {}. Type 'help' for available commands.", parts[0]);
        continue;
      }
    };

    let msg = IpcMessage::Command(cmd);
    if let Err(e) = ipc::send_message(&mut stream, &msg) {
      eprintln!("Failed to send command: {}", e);
      break;
    }

    // For quit command, wait for response then exit
    if should_quit {
      // Give a moment for response
      std::thread::sleep(std::time::Duration::from_millis(100));
      break;
    }
  }

  Ok(())
}

/// Connect to daemon with retry
fn connect_with_retry() -> Result<UnixStream> {
  let socket_path = socket_path();
  for attempt in 1..=5 {
    match UnixStream::connect(socket_path) {
      Ok(stream) => {
        info!("Connected to daemon");
        return Ok(stream);
      }
      Err(_) if attempt < 5 => {
        eprintln!("Connection attempt {} failed, retrying...", attempt);
        thread::sleep(Duration::from_millis(200));
      }
      Err(e) => {
        return Err(e.into());
      }
    }
  }
  Err(anyhow::anyhow!("Failed to connect to daemon"))
}

/// Print response in human-readable format
fn print_response(resp: Response) {
  match resp {
    Response::Status {
      node_id,
      local_endpoints,
      peers,
    } => {
      println!("=== Daemon Status ===");
      println!("Node ID: {}", node_id);
      println!("Local Endpoints:");
      for ep in local_endpoints {
        println!("  {}", ep);
      }
      println!("Connected Peers: {}", peers.len());
      for peer in peers {
        println!("  {} - {}", peer.peer_id, peer.endpoint);
      }
    }
    Response::Peers(peers) => {
      println!("=== Connected Peers ===");
      if peers.is_empty() {
        println!("No connected peers");
      } else {
        for peer in peers {
          println!("{} - {}", peer.peer_id, peer.endpoint);
        }
      }
    }
    Response::Message(msg) => {
      println!("{}", msg.display());
    }
    Response::Result { success, message } => {
      if success {
        println!("✓ {}", message);
      } else {
        eprintln!("✗ {}", message);
      }
    }
    Response::Error(e) => {
      eprintln!("Error: {}", e);
    }
  }
}
