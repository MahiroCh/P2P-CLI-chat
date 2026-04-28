//! REPL mode for p2p chat.

use crate::client::{Error, ErrorKind, Result};
use p2p_chat::cli_schema::InteractiveCommand;
use crate::client::daemon_control as daemon; 

use rustyline::error::ReadlineError as RstlnReadlineErr;

// == REPL logic ==

pub async fn run() -> Result<()> {
  let mut repl_engine = rustyline::DefaultEditor::new()
    .inspect_err( |err| {
      log::debug!("During repl::run(): REPL engine initialization error details: {err:?}");
    })
    .map_err(|err| Error::new(ErrorKind::ReplFailed, err))?;
  
  let mut daemon_client = daemon::Session::connect().await
    .inspect_err(|err| {
      log::debug!("In repl::run(), daemon process connection details: {err:?}");
    })
    .map_err(|err| Error::new(ErrorKind::DaemonConnectionFailed, err))?;

  log::info!("Connected to daemon session from REPL");
  
  loop {
    let readline = repl_engine.readline("> ");

    let raw_cmd = match readline {
      Ok(raw_cmd) => raw_cmd,
      Err(RstlnReadlineErr::Interrupted) => {
        log::info!("REPL input interrupted (Ctrl-C); ignoring and continuing REPL loop");
        println!("Use 'quit' command to exit\n");
        continue;
      },
      Err(RstlnReadlineErr::Eof) => {
        log::info!("EOF received in REPL input; exiting REPL");
        println!("Exiting interactive mode...");
        break;
      },
      Err(err) => {
        log::debug!("repl_engine.readline() failed to read input in REPL mode: {err:?}");
        return Err(Error::new(ErrorKind::ReplReadCliFailed, err));
      }
    };
    
    // TODO: Consider improving error handling or e.print() statements.
    let cmd = match parse_raw_cmd(raw_cmd) {
      Ok(cmd) => cmd,
      Err(ParseCmdError::ShlexParseFailure) => {
        log::warn!("REPL command parsing failed: shlex split failed");
        use clap::CommandFactory;
        if let Err(print_err) = InteractiveCommand::command()
          .error(
            clap::error::ErrorKind::InvalidValue,
            "command of invalid format"
          )
          .print()
        {
          log::debug!("Failed to print REPL parse error (help message) to stdio: {print_err:?}");
          return Err(Error::new(ErrorKind::ReplFailed, print_err));
        }
        continue;
      }
      Err(ParseCmdError::ClapParseFailure(e)) => {
        log::warn!("REPL command parsing failed by clap engine");
        if let Err(print_err) = e.print() {
          log::debug!("Failed to print clap parse error (help message) to stdio: {print_err:?}");
          return Err(Error::new(ErrorKind::ReplFailed, print_err));
        }
        continue;
      }
    };

    match cmd {
      InteractiveCommand::Quit => {
        log::info!("REPL `quit` command received, exiting REPL...");
        println!("Exiting interactive mode...");
        break;
      },
      InteractiveCommand::Peer(peer_cmd) => {
        log::info!("Sending peer command to daemon from REPL");
        // Temp for testing/debugging purposes.
        let response = daemon_client._test_handle_peer_command(&peer_cmd).await
          .map_err(|err| {
            log::debug!("Communicating peer command to daemon error details: {err:?}");
            Error::new(ErrorKind::PeerCommandFailed, "_test_handle_peer_command failed in repl mode")
          })?;
        log::info!("Received response from daemon for peer command sent from REPL");
        println!("Response from daemon: {response}\n");
      },
      _ => unreachable!()
    } 
  }

  Ok(())
}

enum ParseCmdError {
  ShlexParseFailure,
  ClapParseFailure(clap::Error),
}

fn parse_raw_cmd(raw_cmd: String) -> std::result::Result<InteractiveCommand, ParseCmdError> {
  let shlexed_cmd = shlex::split(&raw_cmd)
    .ok_or(ParseCmdError::ShlexParseFailure)?;

  use clap::Parser;
  InteractiveCommand::try_parse_from(shlexed_cmd)
    .map_err(ParseCmdError::ClapParseFailure)
}
