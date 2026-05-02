//! Client process for p2p chat application.

mod daemon_control;
mod error;
mod repl;

use daemon_control as daemon;
use error::Result;
pub use error::{Error, ErrorKind};
use p2p_chat::{cli_schema::*, logger};

// == Run client ==

pub fn run(cmd: Command) -> std::result::Result<(), ()> {
  if let Err(err) = logger::init_client_logger() {
    eprintln!("Failed to start client");
    // Logger is not initialized so fallback to printing error details to stderr.
    eprintln!("Logger initialization error: {err}");
    return Err(());
  }

  if match cmd {
    Command::Daemon { subcmd } => handle_daemon_cmd(&subcmd),
    Command::Peer { subcmd } => handle_peer_cmd(&subcmd),
    Command::Interactive => handle_repl_cmd(),
    _ => unreachable!(),
  }
  .is_err()
  {
    return Err(());
  }

  Ok(())
}

// == Command handlers ==

fn handle_daemon_cmd(cmd: &DaemonCmd) -> Result<()> {
  match cmd {
    DaemonCmd::Start => match daemon::create() {
      Ok(daemon::CreateRes::Started { pid }) => {
        println!("Daemon started");
        log::info!("Daemon started with PID: {pid}");
      }
      Ok(daemon::CreateRes::Running { pid }) => {
        println!("Daemon is already running");
        log::info!("Daemon already running with PID: {pid}");
      }
      Err(err) => {
        eprintln!("Failed to create daemon. See logs for more info");
        log::error!("Failed to create daemon process: {err}");
        return Err(Error::new(ErrorKind::DaemonStartFailed, err));
      }
    },
    DaemonCmd::Stop => match daemon::destroy() {
      Ok(daemon::DestroyRes::Destroyed { pid }) => {
        println!("Daemon stopped");
        log::info!("Daemon stopped, PID was: {pid}");
      }
      Ok(daemon::DestroyRes::NotRunning) => {
        println!("Daemon is already not running");
        log::info!("Daemon stop requested but daemon is already not running");
      }
      Err(err) => {
        eprintln!("Failed to stop daemon. See logs for more info");
        log::error!("Failed to stop daemon process: {err}");
        return Err(Error::new(ErrorKind::DaemonStopFailed, err));
      }
    },
    DaemonCmd::Status => match daemon::status() {
      Ok(daemon::Status::Running { pid }) => {
        println!("Daemon is running");
        log::info!("Daemon is running with PID: {pid}");
      }
      Ok(daemon::Status::NotRunning) => {
        println!("Daemon is not running");
        log::info!("Daemon is not running");
      }
      Err(err) if matches!(err.kind(), ErrorKind::DaemonStateUnknown) => {
        eprintln!("Failed to obtain daemon state. See logs for more info");
        log::error!("Failed to obtain daemon state: {err}");
        return Err(err);
      }
      Err(err) if matches!(err.kind(), ErrorKind::DaemonCorrupted) => {
        eprintln!("Daemon is corrupted. See logs for more info");
        log::error!("Daemon is corrupted: {err}");
        return Err(err);
      }
      Err(_) => unreachable!(),
    },
    _ => unreachable!(),
  }

  Ok(())
}

#[tokio::main]
async fn handle_peer_cmd(cmd: &PeerCmd) -> Result<()> {
  ensure_daemon_ready()?;

  let mut daemon_client = daemon::Session::connect()
    .await
    .inspect_err(|err| {
      eprintln!("Failed to connect to daemon. See logs for more info");
      log::error!("Failed to connect to daemon socket/session: {err}");
    })
    .map_err(|err| Error::new(ErrorKind::DaemonConnectionFailed, err))?;

  // Temp behavior for testing.
  let response = daemon_client
    ._test_handle_peer_command(cmd)
    .await
    .inspect_err(|err| {
      eprintln!("Peer command failed. See logs for more info");
      log::error!("Peer command failed: {err}");
    })
    .map_err(|err| Error::new(ErrorKind::PeerCommandFailed, err))?;

  println!("Response from daemon: {response}");

  Ok(())
}

#[tokio::main]
async fn handle_repl_cmd() -> Result<()> {
  ensure_daemon_ready()?;

  match repl::run().await {
    Ok(()) => {}
    Err(err) if matches!(err.kind(), ErrorKind::DaemonConnectionFailed) => {
      eprintln!(
        "Interactive mode failed because failed to connect to daemon. \
         See logs for more info"
      );
      log::error!("Interactive mode failed: {err}");
      return Err(err);
    }
    Err(err) if matches!(err.kind(), ErrorKind::PeerCommandFailed) => {
      eprintln!(
        "Failed to communicate peer command with daemon in interactive mode"
      );
      log::error!("Failed to communicate peer command to daemon: {err}");
      return Err(err);
    }
    Err(err) => {
      eprintln!("Interactive mode failed. See logs for more info");
      log::error!("Interactive mode failed: {err}");
      return Err(err);
    }
  }

  Ok(())
}

// == Helpers ==

fn ensure_daemon_ready() -> Result<()> {
  match daemon::status() {
    Ok(daemon::Status::Running { .. }) => Ok(()),
    Ok(daemon::Status::NotRunning) => {
      eprintln!("Daemon is not running. Start daemon first");
      log::info!("Client command requires daemon but daemon is not running");
      Err(Error::from(ErrorKind::DaemonNotRunningButNeeded))
    }
    Err(err)
      if matches!(
        err.kind(),
        ErrorKind::DaemonStateUnknown | ErrorKind::DaemonCorrupted
      ) =>
    {
      eprintln!(
        "Cannot proceed: daemon state is unknown or corrupted. \
         See logs for more info"
      );
      log::error!(
        "Daemon state invalid, cannot procceed with client command: {err}"
      );
      Err(err)
    }
    Err(_) => unreachable!(),
  }
}
