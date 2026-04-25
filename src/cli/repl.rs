//! REPL mode for p2p chat.

use tokio::net::UnixStream as TokioUnixStream;
use rustyline::error::ReadlineError as RstlnReadlineErr;

use crate::daemon;
use p2p_chat::{
  socket,
  cli_schema::*
};

// Driver code.

pub async fn run() {
  let mut repl_engine = rustyline::DefaultEditor::new().unwrap();

  // TODO: unwrap()?
  let mut socket = socket::connect_from_cli(&daemon::control::socket_file_path()).await.unwrap();
  
  loop {
    let readline = repl_engine.readline("> ");
    match readline {
      Ok(raw_cmdline) => {
        // TODO: Add real asynchrony.
        match parse_cmdline(&mut socket, raw_cmdline).await {
          ReplAction::Quit => return,
          ReplAction::Continue => continue,
        }
      }
      Err(RstlnReadlineErr::Interrupted) => {
        println!("Use 'quit' command to exit");
      },
      
      Err(err) => {
        // TODO: Debug representation of an error may not be user-friendly, consider implementing Display.
        eprintln!("{:?}\nExiting repl mode...", err);
        return;
      }
    }
  }
}

enum ReplAction {
  Continue,
  Quit,
}

async fn parse_cmdline(socket: &mut TokioUnixStream, raw_cmdline: String) -> ReplAction {
  use clap::Parser;
  let shlexed_cmd = match shlex::split(&raw_cmdline) {
    Some(cmd) => cmd,
    None => {
      // TODO: Consider maybe just printing clap's help message instead?
      eprintln!("shlex couldn't parse the command line: check quotes and escaping");
      return ReplAction::Continue;
    }
  };
  let cmdline = match InteractiveCommand::try_parse_from(shlexed_cmd) {
    Ok(c) => c,
    Err(e) => {
      // TODO: Handle errors.
      let _ = e.print();
      return ReplAction::Continue;
    },
  };

  process_cmdline(socket, cmdline).await
}

async fn process_cmdline(socket: &mut TokioUnixStream, cmdline: InteractiveCommand) -> ReplAction {
  match cmdline {
    InteractiveCommand::Quit => {
      println!("Exiting repl mode...");
      ReplAction::Quit
    },
    InteractiveCommand::Peer(peer_cmd) => {
      handle_peer_cmd(socket, peer_cmd).await;
      ReplAction::Continue
    }
  }
}

async fn handle_peer_cmd(socket: &mut TokioUnixStream, cmd: PeerCmd) {
  let response = super::_test_send(socket, &cmd).await;
  println!("Daemon response: {response}");
}
