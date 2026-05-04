//! REPL mode for p2p chat.

mod command_processor;

use crate::client::{Error, ErrorKind, Result, daemon_control::ConnectionSession};
use p2p_chat::schemas::{ActionCmd, InteractiveCommand};

use rustyline::error::ReadlineError as RstlnReadlineErr;

// == REPL logic ==

pub(super) async fn run() -> Result<()> {
  let mut repl_engine = rustyline::DefaultEditor::new()
    .inspect_err(|err| {
      log::debug!(
        "During repl::run() rustyline::DefaultEditor::new() failed: {err:?}"
      );
    })
    .map_err(|err| Error::new(ErrorKind::ReplInitFailed, err))?;

  let mut daemon_client = match ConnectionSession::new().await {
    Ok(session) => session,
    Err(err) if matches!(err.kind(), ErrorKind::DaemonRefusedConnection) => {
      log::debug!(
        "During repl::run() daemon refused connection while ConnectionSession::new(): {err:?}"
      );
      return Err(err);
    }
    Err(err) => {
      log::debug!(
        "During repl::run() ConnectionSession::new() failed: {err:?}"
      );
      return Err(err);
    }
  };

  log::info!("Connected to daemon session from REPL");

  loop {
    let cmd = match read_input(&mut repl_engine) {
      Ok(ReadInputRes::Action(action_cmd)) => action_cmd,
      Ok(ReadInputRes::Quit) => {
        println!("Exiting interactive mode...\n");
        log::info!("REPL `quit` command received, exiting REPL...");
        break;
      }
      Ok(ReadInputRes::InterruptedCtrlC) => {
        println!("Use `quit` command to exit interactive mode\n");
        continue;
      }
      Ok(ReadInputRes::ShlexParseFailure) => {
        log::warn!("REPL command parsing failed: shlex split failed");
        use clap::CommandFactory;
        if let Err(print_err) = InteractiveCommand::command()
          .error(
            clap::error::ErrorKind::InvalidValue,
            "command of invalid format",
          )
          .print()
        {
          log::debug!(
            "During repl::run() failed to print REPL parse error \
            (help message) to stdio: {print_err:?}"
          );
          return Err(Error::new(ErrorKind::ReplReadOrParseFailed, print_err));
        }

        println!();
        continue;
      }
      Ok(ReadInputRes::ClapParseFailure(err)) => {
        log::warn!("REPL command parsing failed by clap engine");
        if let Err(print_err) = err.print() {
          log::debug!(
            "During repl::run() failed to print clap parse error \
            (help message) to stdio: {print_err:?}"
          );
          return Err(Error::new(ErrorKind::ReplReadOrParseFailed, print_err));
        }

        println!();
        continue;
      }
      Err(err) => {
        log::error!("Failed to read and parse input in REPL: {err}");
        return Err(err);
      }
    };

    // TODO: Make this call async.
    if let Err(err) = command_processor::process(&mut daemon_client, &cmd).await {
      log::error!("Failed to process action command read from REPL input: {err}");
      return Err(err);
    }
  }

  Ok(())
}

// == Read/parse input helper ==

enum ReadInputRes {
  Action(ActionCmd),
  Quit,
  InterruptedCtrlC,
  ShlexParseFailure,
  ClapParseFailure(clap::Error),
}

// TODO: Migrate to rustyline-async library.
fn read_input(repl_engine: &mut rustyline::DefaultEditor) -> Result<ReadInputRes> {
  let readline = repl_engine.readline("> ");

  let raw_cmd = match readline {
    Ok(raw_cmd) => raw_cmd,
    Err(RstlnReadlineErr::Interrupted) => {
      log::info!("REPL input interrupted (Ctrl-C); printing help message and continuing REPL loop");
      return Ok(ReadInputRes::InterruptedCtrlC);
    }
    Err(RstlnReadlineErr::Eof) => {
      log::info!("EOF received in REPL input; exiting REPL");
      return Ok(ReadInputRes::Quit);
    }
    Err(err) => {
      log::debug!("repl_engine.readline() failed to read input in repl::run()'s REPL loop: {err:?}");
      return Err(Error::new(ErrorKind::ReplReadOrParseFailed, err));
    }
  };

  let cmd = match shlex::split(&raw_cmd) {
    Some(shlexed_cmd) => {
      use clap::Parser;
      match InteractiveCommand::try_parse_from(shlexed_cmd) {
        Ok(cmd) => cmd,
        Err(err) => return Ok(ReadInputRes::ClapParseFailure(err)),
      }
    }
    None => return Ok(ReadInputRes::ShlexParseFailure),
  };

  match cmd {
    InteractiveCommand::Quit => Ok(ReadInputRes::Quit),
    InteractiveCommand::Action(action_cmd) => Ok(ReadInputRes::Action(action_cmd)),
    _ => unreachable!(),
  }
}
