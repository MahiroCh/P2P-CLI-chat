//! Command-line interface for the P2P chat application.
//! 
//! This module defines parsing of P2P-chat command-line arguments (including REPL mode), 
//! and controlling the daemon process lifecycle.

mod repl;

use tokio::net::UnixStream as TokioUnixStream;

use crate::daemon;
use p2p_chat::{
  socket,
  cli_schema::*
};

// Driver code.

pub fn parse_cmdline() {
  use clap::Parser;
  let cmdline = match Cli::try_parse() {
    Ok(cmd) => cmd,
    Err(err) => {
      if let Err(io_err) = err.print() {
        eprintln!("I/O error: {io_err}");
      }
      return;
    },
  };
  process_cmdline(cmdline);
}

fn process_cmdline(cmdline: Cli) {
  // Runs the daemon itself if INTERNAL_DAEMON_INIT_FLAG hidden flag is set by 
  // daemon::control::create() function.
  if cmdline.init_internal {
    daemon::run();
    return;
  }

  // If the flag is not set, command is required to be present by the cli_schema,
  // so we can safely unwrap it here.
  let cmd = cmdline.command.unwrap();
  if let Command::Daemon {subcmd} = cmd {
    handle_daemon_control_cmd(subcmd);
  } else {
    if let None = daemon::control::status() {
      println!(
        "Daemon is not running\n\
         Please start the daemon first with `daemon start` command"
      );
      return;
    }
    let tokio_rt = tokio::runtime::Runtime::new().unwrap();
    match cmd {
      // NOTE: Maybe I will change the way I use tokio runtime later.
      Command::Peer { subcmd } => tokio_rt.block_on(handle_peer_cmd(subcmd)),
      Command::Interactive => tokio_rt.block_on(handle_repl_cmd()),
      _ => unreachable!()
    }
  }
}

// Command handlers.

fn handle_daemon_control_cmd(cmd: DaemonCmd) {
  match cmd {
    DaemonCmd::Start => daemon::control::create(),
    DaemonCmd::Stop => daemon::control::destroy(),
    DaemonCmd::Status => match daemon::control::status() {
      Some(pid) => println!("Daemon is running with PID {}", pid),
      None => println!("Daemon is not running"),
    },
  }
}

async fn handle_peer_cmd(cmd: PeerCmd) {
  // TODO: unwrap()?
  let mut socket = socket::connect_from_cli(&daemon::control::socket_file_path()).await.unwrap();
  
  // NOTE: Temporary code for testing the socket communication, to be replaced 
  // NOTE: with real command processing logic later.
  let response = _test_send(&mut socket, &cmd).await;
  println!("Daemon response: {response}");
}

async fn handle_repl_cmd() {
  repl::run().await;
}

// NOTE: Temporary function for testing the socket communication,
// NOTE: to be replaced with real command processing logic later.
async fn _test_send(socket: &mut TokioUnixStream, cmd: &PeerCmd) -> String {
  let serded_cmd = serde_json::to_string(cmd)
    .expect("failed to serialize command");

  socket::write_data(socket, &serded_cmd).await
    .expect("failed to write message to socket");

  socket::read_data(socket).await
    .expect("failed to read message from socket")
}
