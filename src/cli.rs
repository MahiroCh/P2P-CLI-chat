mod repl;

use tokio::net::UnixStream as TokioUnixStream;

use crate::daemon;
use p2p_chat::{
  socket,
  protocol::*
};

// ========================================================================
// Driver code
// ========================================================================

pub fn parser() {
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
  if cmdline.init_internal {
    daemon::run();
    return;
  }

  let Some(cmd) = cmdline.command else {
    unreachable!(
      "Invalid command-line arguments: \
       no command provided and internal init flag is not set"
    )
  };

  match cmd {
    Command::Daemon { subcmd } => handle_daemon_control_cmd(subcmd),
    Command::Peer { subcmd } => {
      let tokio_rt = tokio::runtime::Runtime::new().unwrap();
      tokio_rt.block_on(handle_peer_cmd(subcmd));
    }
    Command::Interactive => {
      let tokio_rt = tokio::runtime::Runtime::new().unwrap();
      tokio_rt.block_on(repl::run());
    }
  }
}

// ========================================================================
// Command handlers
// ========================================================================

async fn handle_peer_cmd(cmd: PeerCmd) {
  if let None = daemon::control::status() {
    println!(
      "Daemon is not running\n\
        Please start the daemon first with `daemon start` command"
    );
    return;
  }

  let mut socket = socket::connect_from_cli(&daemon::control::socket_file_path()).await.unwrap(); // TODO: unwrap()?
  
  let response = _test_send(&mut socket, &cmd).await;
  println!("Daemon received this command: {}", response);
}

fn handle_daemon_control_cmd(cmd: DaemonCmd) {
  match cmd {
    DaemonCmd::Stop => daemon::control::destroy(),
    DaemonCmd::Start => daemon::control::create(),
    DaemonCmd::Status => match daemon::control::status() {
      Some(pid) => println!("Daemon is running with PID {}", pid),
      None => println!("Daemon is not running"),
    },
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
