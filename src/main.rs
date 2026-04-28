//! Entry point of the application.

mod daemon;
mod client;

use p2p_chat::cli_schema::Cli;

use std::process::{ExitCode, Termination};

// == Custom application exit code ==

#[derive(Debug)]
enum AppExitCode {
  Success,
  Failure,
}

impl Termination for AppExitCode {
  fn report(self) -> ExitCode {
    match self {
      AppExitCode::Success => ExitCode::SUCCESS,
      AppExitCode::Failure => ExitCode::FAILURE,
      // NOTE: Probably will add more different exit codes later, so this custom thing exists.
    }
  }
}

// == Entry point ==

fn main() -> AppExitCode {
  use clap::Parser;
  let input = match Cli::try_parse() {
    Ok(input) => input,
    Err(err) => {
      if err.print().is_err() { return AppExitCode::Failure; }
      return AppExitCode::Success;
    },
  };

  // Runs the daemon (in case INTERNAL_DAEMON_INIT_FLAG hidden flag is set by 
  // daemon::control::create() function). See cli schema for more info.
  if input.init_internal {
    match daemon::run() {
      Ok(_) => AppExitCode::Success,
      Err(_) => AppExitCode::Failure
    }
  }
  // Run the client. If the internal flag is not set, command is required to be 
  // present by the cli schema, so we can safely unwrap.
  else {
    match client::run(input.command.unwrap()) { 
      Ok(_) => AppExitCode::Success,
      Err(_) => AppExitCode::Failure
    }
  }
}
