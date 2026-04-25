mod cli;
mod daemon;

// Custom ExitCode.

#[derive(Debug)]
enum AppExitCode {
  SUCCESS,
  FAILURE,
  // Exit code for clap-related errors.
  PARSEFAILURE,
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

// Driver code.

fn main() -> ExitCode {
  cli::parse_cmdline();
  AppExitCode::SUCCESS.into()
}
