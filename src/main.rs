mod cli;
mod daemon;

// ========================================================================
// Custom ExitCode
// ========================================================================

#[derive(Debug)]
enum AppExitCode {
  SUCCESS,
  FAILURE,
  PARSEFAILURE, // exit code for clap-related errors
}

use std::process::ExitCode;
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
  cli::parser();
  AppExitCode::SUCCESS.into()
}
