use tokio::net::UnixStream as TokioUnixStream;
use rustyline::error::ReadlineError as RstlnReadlineErr;

use crate::daemon;
use p2p_chat::{
  socket,
  protocol::*
};

enum ReplAction {
  Continue,
  Quit,
}

// ========================================================================
// Driver code
// ========================================================================

pub async fn run() {
  if let None = daemon::control::status() {
    println!(
      "Daemon is not running\n\
        Please start the daemon first with `daemon start` command"
    );
    return;
  }
  
  let mut repl_engine = rustyline::DefaultEditor::new().unwrap();

  let mut socket = socket::connect_from_cli(&daemon::control::socket_file_path()).await.unwrap(); // TODO: unwrap()?
  
  loop {
    let readline = repl_engine.readline("> ");
    match readline {
      Ok(raw_cmd) => {
        match process_cmd(&mut socket, raw_cmd).await { // TODO: Add real asynchrony
          ReplAction::Quit => return,
          ReplAction::Continue => continue,
        }
      }
      Err(RstlnReadlineErr::Interrupted) => {
        println!("Use 'quit' to exit");
      },
      
      Err(err) => {
        eprintln!("{:?}\nExiting repl mode...", err); // TODO: Debug representation of an error may not be user-friendly, consider implementing Display
        return;
      }
    }
  }
}

// ========================================================================
// REPL command processing
// ========================================================================

async fn process_cmd(socket: &mut TokioUnixStream, raw_cmd: String) -> ReplAction {
  use clap::Parser;
  let shlexed_cmd = match shlex::split(&raw_cmd) {
    Some(cmd) => cmd,
    None => {
      eprintln!("Invalid command syntax: check quotes and escaping");
      return ReplAction::Continue;
    }
  };
  let cmd = match InteractiveCommand::try_parse_from(shlexed_cmd) {
    Ok(c) => c,
    Err(e) => {
      let _ = e.print(); // TODO: Handle errors
      return ReplAction::Continue;
    },
  };

  match cmd {
    InteractiveCommand::Quit => {
      println!("Exiting repl mode...");
      ReplAction::Quit
    },
    InteractiveCommand::Peer(peer_cmd) => {
      let response = _test_send(socket, &peer_cmd).await;
      println!("Daemon received this command: {}", response);
      
      ReplAction::Continue
    }
  }
}





/* TEMP HELPER */
async fn _test_send(socket: &mut TokioUnixStream, cmd: &PeerCmd) -> String {
  let serded_cmd = serde_json::to_string(cmd)
    .expect("failed to serialize command");

  socket::write_data(socket, &serded_cmd).await
    .expect("failed to write message to socket");

  socket::read_data(socket).await
    .expect("failed to read message from socket")
}
