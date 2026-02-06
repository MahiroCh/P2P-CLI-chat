mod ipc;
mod types;

use ipc::socket_path;
use types::{Command, IpcMessage, Response};
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::time::Duration;

fn main() {
  let args: Vec<String> = std::env::args().collect();

  if args.len() < 2 {
    println!("Usage:");
    println!("  {} connect <peer_id>", args[0]);
    println!("  {} send <peer_id> <message>", args[0]);
    std::process::exit(1);
  }

  let result = match args[1].as_str() {
    "connect" => {
      if args.len() < 3 {
        println!("Usage: {} connect <peer_id>", args[0]);
        std::process::exit(1);
      }
      send_command(Command::Connect {
        endpoint: args[2].clone(),
      })
    }
    "send" => {
      if args.len() < 4 {
        println!("Usage: {} send <peer_id> <message>", args[0]);
        std::process::exit(1);
      }
      let content = args[3..].join(" ");
      send_command(Command::Send {
        peer_id: args[2].clone(),
        content,
      })
    }
    _ => {
      println!("Unknown command: {}", args[1]);
      println!("Available: connect, send");
      std::process::exit(1);
    }
  };

  if let Err(e) = result {
    eprintln!("Error: {}", e);
    std::process::exit(1);
  }
}

fn send_command(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
  let mut stream = connect_with_retry()?;

  let msg = IpcMessage::Command(cmd);
  ipc::send_message(&mut stream, &msg)?;

  let mut buffer = [0u8; 8192];
  match stream.read(&mut buffer) {
    Ok(0) => println!("Connection closed"),
    Ok(n) => {
      let data = String::from_utf8_lossy(&buffer[..n]);
      match serde_json::from_str::<IpcMessage>(data.trim()) {
        Ok(IpcMessage::Response(resp)) => print_response(resp),
        Ok(IpcMessage::Command(_)) => eprintln!("Unexpected command from daemon"),
        Err(e) => eprintln!("Parse error: {}", e),
      }
    }
    Err(e) => eprintln!("Read error: {}", e),
  }

  Ok(())
}

fn connect_with_retry() -> Result<UnixStream, Box<dyn std::error::Error>> {
  for _ in 0..5 {
    match UnixStream::connect(socket_path()) {
      Ok(stream) => return Ok(stream),
      Err(_) => std::thread::sleep(Duration::from_millis(100)),
    }
  }
  Err("Failed to connect to daemon".into())
}

fn print_response(resp: Response) {
  match resp {
    Response::Ok(msg) => println!("{}", msg),
    Response::Error(msg) => eprintln!("Error: {}", msg),
    Response::Message(msg) => println!("[{}] {}", msg.sender, msg.content),
  }
}
