//! Client process for p2p chat application.

mod daemon_control;
mod error;
mod repl;

use daemon_control::{self as daemon, ConnectionSession};
use error::{Error, ErrorKind, Result};
use p2p_chat::{logger, schemas::*};

// == Run client ==

pub(super) fn run(cmd: Command) -> std::result::Result<(), ()> {
  if let Err(err) = logger::init_client_logger() {
    eprintln!("Failed to start client");
    // Logger is not initialized so fallback to printing error details to stderr.
    eprintln!("Logger initialization error: {err}");
    return Err(());
  }

  if match cmd {
    Command::Daemon { subcmd } => handle_daemon_cmd(&subcmd),
    Command::Action(action_cmd) => handle_action_cmd(&action_cmd),
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
      Ok(daemon::CreateRes::Started { .. }) => {
        println!("Daemon started");
      }
      Ok(daemon::CreateRes::Running { .. }) => {
        println!("Daemon is already running");
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
        log::info!("Daemon status checked: it is running with PID: {pid}");
      }
      Ok(daemon::Status::NotRunning) => {
        println!("Daemon is not running");
        log::info!("Daemon status checked: it is not running");
      }
      Err(err) if matches!(err.kind(), ErrorKind::DaemonStateUnknown) => {
        eprintln!("Failed to obtain daemon state. See logs for more info");
        log::error!(
          "Daemon status check requested: failed to obtain its state: {err}"
        );
        return Err(err);
      }
      Err(err) if matches!(err.kind(), ErrorKind::DaemonCorrupted) => {
        eprintln!("Daemon is corrupted. See logs for more info");
        log::error!("Daemon status checked: it is corrupted: {err}");
        return Err(err);
      }
      Err(_) => unreachable!(),
    },
    _ => unreachable!(),
  }

  Ok(())
}

#[tokio::main]
async fn handle_action_cmd(cmd: &ActionCmd) -> Result<()> {
  ensure_daemon_ready()?;

  let mut daemon_client = match ConnectionSession::new().await {
    Ok(session) => session,
    Err(err) if matches!(err.kind(), ErrorKind::DaemonRefusedConnection) => {
      eprintln!("Daemon refused connection. See logs for more info");
      log::error!("Daemon refused connection while handling action command: {err}");
      return Err(err);
    }
    Err(err) => {
      eprintln!("Failed to connect to daemon. See logs for more info");
      log::error!("Failed to connect to daemon socket/session: {err}");
      return Err(err);
    }
  };

  match daemon_client._send_cmd_to_daemon(cmd).await {
    Ok(()) => {}
    Err(err) if matches!(err.kind(), ErrorKind::DaemonAbortedConnection) => {
      eprintln!(
        "Daemon closed connection while sending action command. See logs for more info"
      );
      log::error!("Failed to send action command because daemon closed connection: {err}");
      return Err(err);
    }
    Err(err) => {
      eprintln!("Failed to send action command to daemon. See logs for more info");
      log::error!("Failed to send action command: {err}");
      return Err(err);
    }
  }

  log::info!("Action command sent to daemon: {cmd:?}");

  // NOTE: Temp behavior for testing.
  let response = match daemon_client._recv_response_from_daemon().await {
    Ok(response) => response,
    Err(err) if matches!(err.kind(), ErrorKind::DaemonAbortedConnection) => {
      eprintln!("Daemon closed connection while client was waiting for response. See logs for more info");
      log::error!("Failed to receive response from daemon because it closed the connection: {err}");
      return Err(err);
    }
    Err(err) => {
      eprintln!("Failed to receive response from daemon. See logs for more info");
      log::error!("Failed to receive response from daemon: {err}");
      return Err(err);
    }
  };

  println!("Response from daemon: {:?}", response);
  log::info!("Received response from daemon for action command: {response:?}");

  Ok(())
}

#[tokio::main]
async fn handle_repl_cmd() -> Result<()> {
  ensure_daemon_ready()?;

  match repl::run().await {
    Ok(()) => {}
    Err(err) if matches!(err.kind(), ErrorKind::DaemonRefusedConnection) => {
      eprintln!(
        "Daemon refused connection while entering interactive mode. See logs for more info"
      );
      log::error!("Daemon refused connection in REPL mode: {err}");
      return Err(err);
    }
    Err(err) if matches!(err.kind(), ErrorKind::DaemonAbortedConnection) => {
      eprintln!(
        "Daemon closed connection while communicating with it in interactive mode. \
         See logs for more info"
      );
      log::error!("Daemon aborted connection during REPL mode: {err}");
      return Err(err);
    }
    Err(err) if matches!(err.kind(), ErrorKind::DaemonConnectionFailed) => {
      eprintln!(
        "Interactive mode failed because failed to connect to daemon. \
         See logs for more info"
      );
      log::error!("REPL mode failed: {err}");
      return Err(err);
    }
    Err(err)
      if matches!(
        err.kind(),
        ErrorKind::WriteCommandFailed
          | ErrorKind::ReadResponseFailed
          | ErrorKind::SerdeFailed
      ) =>
    {
      eprintln!(
        "Failed to communicate action command with daemon in interactive mode"
      );
      log::error!(
        "Failed to communicate action command with daemon in REPL mode: {err}"
      );
      return Err(err);
    }
    Err(err) => {
      eprintln!("Interactive mode failed. See logs for more info");
      log::error!("REPL mode failed: {err}");
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
