use std::process::ExitCode;
use clap::Parser;

// ========================================================================
// Driver code
// ========================================================================

fn main() -> ExitCode {

  /* Parse command-line. */
  let cli = match CliInput::try_parse() {
    Ok(parsed) => parsed,
    Err(e) => {
      if let Err(io_err) = e.print() {
        eprintln!("I/O error: {io_err}");
        return AppExitCode::FAILURE.into();
      }
      return AppExitCode::PARSEFAILURE.into();
    },
  };

  match cli.command {
    Command::Connect => {},
    Command::Send{peer_id, message} => {},
  }

  AppExitCode::SUCCESS.into()
}

// ========================================================================
// Command-line parsing
// // Implemented with clap library
// ========================================================================

#[derive(Parser)]
struct CliInput {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Connect to the service provider
    Connect,

    /// Send a message to a peer
    Send {
        /// Peer ID
        peer_id: String,
        /// Message content
        message: String,
    }
}

// ========================================================================
// Custom ExitCode
// ========================================================================

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
// Helpers
// ========================================================================



// ========================================================================
// Tests
// ========================================================================

