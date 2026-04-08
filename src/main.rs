// TODO: Implement normal error handling

#![allow(unused)] // TODO: Remove this when the code is fully implemented

mod cli;
mod daemon;

use std::process::ExitCode;

// ========================================================================
// Custom ExitCode
// ========================================================================

#[derive(Debug)]
enum AppExitCode {
  SUCCESS,
  FAILURE,
  PARSEFAILURE, // exit code for clap-related errors
}

impl From<AppExitCode> for ExitCode {
  fn from(code: AppExitCode) -> Self {
    match code {
      AppExitCode::SUCCESS      => ExitCode::SUCCESS,
      AppExitCode::FAILURE      => ExitCode::FAILURE,
      AppExitCode::PARSEFAILURE => ExitCode::from(2),
    }
  }
}

// ========================================================================
// Driver code
// ========================================================================

fn main() -> ExitCode {
  use clap::Parser;
  let cmd = match cli::Cmdline::try_parse() {
    Ok(parsed) => parsed,
    Err(e) => {
      if let Err(io_err) = e.print() {
        eprintln!("I/O error: {io_err}");
        return AppExitCode::FAILURE.into();
      }
      return AppExitCode::PARSEFAILURE.into();
    },
  };

  cli::process_cmdline(cmd);

  AppExitCode::SUCCESS.into()
}
