//! Client process for p2p chat application.

mod daemon_control;
mod daemon_session;
mod error;
mod logger;
mod repl;

use daemon::State::*;
use daemon_control as daemon;
use daemon_session::Session as DaemonSession;
use error::{Error, ErrorKind};
use p2p_chat::cli_interface::*;
use p2p_chat::schemas::{ClientRequest, DaemonEvent};
use ErrorKind::*;

type Result<T> = std::result::Result<T, Error>;

pub const DEFAULT_LOG_LEVEL: &str = "info";

// == Run client ==

pub(super) fn run(cmd: Command, cli_log_level: String) -> Result<()> {
  let log_level = if cli_log_level.is_empty() {
    DEFAULT_LOG_LEVEL.to_owned()
  } else {
    cli_log_level
  };

  logger::init(log_level).inspect_err(|err| {
    // Logger is not initialized so fallback to printing error details to stderr.
    eprintln!("Failed to start client due to logger initialization error: {err}");
  })?;

  match cmd {
    Command::Daemon { subcmd } => daemon(&subcmd)?,
    Command::Client { subcmd } => client(&subcmd)?,
    Command::Info(cmd) => info(&cmd)?,
    _ => unreachable!(),
  }

  Ok(())
}

// == Command handlers ==

fn daemon(cmd: &DaemonCmd) -> Result<()> {
  match cmd {
    DaemonCmd::Start { daemon_log_level } => {
      match daemon::create(daemon_log_level) {
        Ok(ProcessCreated) => {
          println!("Daemon started");
        }
        Ok(Running { .. }) => {
          println!("Daemon is already running");
        }
        Err(err) => {
          eprintln!("Failed to create daemon: {err}");
          log::error!("Failed to spawn daemon process: {err}");
          return Err(err);
        }
        _ => unreachable!("other states are not expected from the callee"),
      }
    }
    DaemonCmd::Stop => match daemon::destroy() {
      Ok(StopRequested { .. }) => {
        println!(
          "Daemon stop requested. To verify that daemon is stopped, \
           check its status with the corresponding command"
        );
      }
      Ok(NotRunning) => {
        println!("Daemon is already not running");
      }
      Err(err) => {
        eprintln!("Failed to stop daemon: {err}");
        log::error!("Failed to stop daemon process: {err}");
        return Err(err);
      }
      _ => unreachable!("other states are not expected from the callee"),
    },
    DaemonCmd::Status => match daemon::status() {
      Running { .. } => {
        println!("Daemon is running");
      }
      NotRunning => {
        println!("Daemon is not running");
      }
      StateUnknown(err) => {
        eprintln!("Failed to obtain daemon state: {err}. Try stop the daemon");
        return Err(err);
      }
      Corrupted(err) => {
        eprintln!("Daemon is corrupted: {err}. Try stop the daemon");
        return Err(err);
      }
      _ => unreachable!("other states are not expected from the callee"),
    },
    _ => unreachable!("other states are not expected from the callee"),
  }

  Ok(())
}

fn client(cmd: &ClientCmd) -> Result<()> {
  match cmd {
    ClientCmd::Interactive => repl()?,
    _ => unreachable!("other commands are not expected from the callee"),
  }

  Ok(())
}

fn repl() -> Result<()> {
  match daemon::status() {
    Running { .. } => {}
    NotRunning => {
      eprintln!("Daemon is not running. Start daemon first");
      log::info!("REPL mode requires daemon but daemon isn't running");
      return Err(Error::other(std::io::Error::other("daemon is not running")));
    }
    Corrupted(err) => {
      eprintln!("Cannot proceed: daemon state is corrupted: {err}");
      log::error!(
        "Cannot procceed with REPL mode: daemon state is corrupted: {err}"
      );
      return Err(err);
    }
    StateUnknown(err) => {
      eprintln!("Cannot proceed: daemon state is unknown: {err}");
      log::error!("Cannot procceed with REPL mode: daemon state is unknown: {err}");
      return Err(err);
    }
    _ => unreachable!("other states are not expected from the callee"),
  }

  match repl::run() {
    Ok(()) => Ok(()),
    Err(err) => {
      if err.kind() == DaemonAbortedConnection {
        eprintln!("Daemon closed connection: {err}");
        log::error!("Daemon aborted connection during REPL mode: {err}");
      } else if err.kind() == DaemonConnectionFailed {
        eprintln!("Failed to connect to daemon: {err}");
        log::error!("REPL mode failed to connect to daemon: {err}");
      } else if err.kind() == ReplInitFailed {
        eprintln!("Interactive mode failed to initialize: {err}");
        log::error!("REPL failed to initialize: {err}");
      } else {
        eprintln!("Interactive mode failed: {err}");
        log::error!("REPL error: {err}");
      }

      Err(err)
    }
  }
}

fn info(cmd: &InfoCmd) -> Result<()> {
  match daemon::status() {
    Running { .. } => {}
    NotRunning => {
      eprintln!("Daemon is not running. Start daemon first");
      log::info!("Info commands require daemon but daemon isn't running");
      return Err(Error::other(std::io::Error::other("daemon is not running")));
    }
    Corrupted(err) => {
      eprintln!("Cannot proceed: daemon state is corrupted: {err}");
      log::error!(
        "Cannot proceed with info command: daemon state is corrupted: {err}"
      );
      return Err(err);
    }
    StateUnknown(err) => {
      eprintln!("Cannot proceed: daemon state is unknown: {err}");
      log::error!(
        "Cannot proceed with info command: daemon state is unknown: {err}"
      );
      return Err(err);
    }
    _ => unreachable!("other states are not expected from the callee"),
  }

  let request = match cmd {
    InfoCmd::List => ClientRequest::List,
    InfoCmd::Myid => ClientRequest::MyID,
    _ => unreachable!(),
  };

  info_async(request).inspect_err(|err| {
    log::error!("Info command failed: {err}");
  })?;

  Ok(())
}

#[tokio::main]
async fn info_async(request: ClientRequest) -> Result<()> {
  let mut daemon_session = DaemonSession::new()
    .await
    .inspect_err(|err| {
      log::debug!("info_async() failed to connect to daemon session: {err:?}");
    })
    .map_err(|err| {
      if err.kind() == DaemonAbortedConnection {
        Error::new(DaemonConnectionFailed, err)
      } else {
        Error::other(err)
      }
    })?;

  daemon_session
    .send_request_to_daemon(&request)
    .await
    .inspect_err(|err| {
      log::debug!("info_async() failed to send request to daemon: {err:?}");
    })
    .map_err(|err| {
      if err.kind() == DaemonAbortedConnection {
        Error::new(SendCommandFailed, err)
      } else {
        Error::other(err)
      }
    })?;

  let event = daemon_session
    .recv_daemon_event()
    .await
    .inspect_err(|err| {
      log::debug!("info_async() failed to receive daemon event: {err:?}");
    })
    .map_err(|err| {
      if err.kind() == DaemonAbortedConnection {
        Error::new(ReceiveResponseFailed, err)
      } else {
        Error::other(err)
      }
    })?;

  match event {
    DaemonEvent::MyId { endpoint_id } => {
      println!("{}", format_myid_output(&endpoint_id));
    }
    DaemonEvent::PeerList { peers } => {
      if peers.is_empty() {
        println!("no peers connected");
      } else {
        for peer in peers {
          println!("{peer}");
        }
      }
    }
    DaemonEvent::Error { message } => {
      eprintln!("Daemon error: {message}");
      return Err(Error::other(std::io::Error::other(message)));
    }
    other => {
      return Err(Error::other(format!(
        "unexpected daemon response for info command: {other:?}"
      )));
    }
  }

  Ok(())
}

fn format_myid_output(ticket: &str) -> String {
  let endpoint_id = serde_json::from_str::<iroh::EndpointAddr>(ticket)
    .map(|addr| addr.id.to_string())
    .unwrap_or_else(|_| "<failed-to-parse-endpoint-id>".to_owned());

  format!(
    "my endpoint id: {endpoint_id}\n\
     my ticket: {ticket}\n\
     tip: connect using either endpoint id or full ticket"
  )
}

// NOTE: Code for action commands not in interactive mode.
// #[tokio::main]
// async fn handle_action_cmd(cmd: &ActionCmd) -> Result<()> {
//   ensure_daemon_ready()?;

//   let mut daemon_client = match ConnectionSession::new().await {
//     Ok(session) => session,
//     Err(err) if matches!(err.kind(), ErrorKind::DaemonRefusedConnection) => {
//       eprintln!("Daemon refused connection. See logs for more info");
//       log::error!("Daemon refused connection while handling action command: {err}");
//       return Err(err);
//     }
//     Err(err) => {
//       eprintln!("Failed to connect to daemon. See logs for more info");
//       log::error!("Failed to connect to daemon socket/session: {err}");
//       return Err(err);
//     }
//   };

//   match daemon_client._send_cmd_to_daemon(cmd).await {
//     Ok(()) => {}
//     Err(err) if matches!(err.kind(), ErrorKind::DaemonAbortedConnection) => {
//       eprintln!(
//         "Daemon closed connection while sending action command. See logs for more info"
//       );
//       log::error!("Failed to send action command because daemon closed connection: {err}");
//       return Err(err);
//     }
//     Err(err) => {
//       eprintln!("Failed to send action command to daemon. See logs for more info");
//       log::error!("Failed to send action command: {err}");
//       return Err(err);
//     }
//   }

//   log::info!("Action command sent to daemon: {cmd:?}");

//   // NOTE: Temp behavior for testing.
//   let response = match daemon_client._recv_response_from_daemon().await {
//     Ok(response) => response,
//     Err(err) if matches!(err.kind(), ErrorKind::DaemonAbortedConnection) => {
//       eprintln!("Daemon closed connection while client was waiting for response. See logs for more info");
//       log::error!("Failed to receive response from daemon because it closed the connection: {err}");
//       return Err(err);
//     }
//     Err(err) => {
//       eprintln!("Failed to receive response from daemon. See logs for more info");
//       log::error!("Failed to receive response from daemon: {err}");
//       return Err(err);
//     }
//   };

//   println!("Response from daemon: {:?}", response);
//   log::info!("Received response from daemon for action command: {response:?}");

//   Ok(())
// }
