#![allow(unused)] // TODO: Remove this when the code is fully implemented

use std::process::ExitCode;
use rustyline::error::ReadlineError as RstlnReadlineErr;
use clap::Parser;

// ========================================================================
// Driver code
// ========================================================================

fn main() -> ExitCode {
  let cmd = match CmdLine::try_parse() {
    Ok(parsed) => parsed,
    Err(e) => {
      if let Err(io_err) = e.print() {
        eprintln!("I/O error: {io_err}");
        return AppExitCode::FAILURE.into();
      }
      return AppExitCode::PARSEFAILURE.into();
    },
  };

  match cmd.command {
    CmdLineCommand::Interactive => { interactive_mode(); }, // TDOO: Handle the exit code and errors more gracefully
    CmdLineCommand::Daemon { command } => { }, // TODO: Implement daemon commands
    CmdLineCommand::PeerCommand(peer_cmd) => { handle_peer_command(&peer_cmd); }, // TODO: Handle the exit code and errors more gracefully
  }

  AppExitCode::SUCCESS.into()
}

// ========================================================================
// Command-line parsing
// // Implemented with clap library
// ========================================================================

/* Common commands for both interactive and non-interactive modes: messaging commands: */

#[derive(clap::Subcommand)]
enum PeerCommand {
  /// Connect to the peer
  Connect {
    /// Peer ID
    peer_id: String,
  },
  /// List connected peers
  Peers,
  /// Send a message to a peer
  Send {
    /// Peer ID
    peer_id: String,
    /// Message content
    message: String,
  },
}

/* Command-line arguments for the main application: */

#[derive(Parser)]
#[command(
  about = "Simple peer‑to‑peer chat.\n\n\
          Before establishing connections, launch daemon first with `daemon` set of commands.\n\
          Use subcommands like `connect`, `send`, and `peers` to manage peers and messages.\n\
          You can also start an interactive terminal session with the `interactive` command, \
          which allows you to enter commands in a REPL-like environment."
)]
struct CmdLine {
  #[command(subcommand)]
  command: CmdLineCommand,
}

#[derive(clap::Subcommand)]
enum CmdLineCommand {
  #[command(flatten)]
  PeerCommand(PeerCommand),
  /// Daemon actions
  Daemon {
    #[command(subcommand)]
    command: DaemonCommand,
  },
  /// Start interactive terminal session
  Interactive,
}

#[derive(clap::Subcommand)]
enum DaemonCommand {
  /// Start the daemon
  Start,
  /// Stop the daemon
  Stop,
  /// Restart the daemon
  Restart,
  /// Get daemon status
  Status,
}

/* Command-line arguments for the interactive terminal: */

#[derive(Parser)]
#[command(no_binary_name=true)] // tells clap not to expect the first argument to be the program name
#[command(bin_name="")] // unsets the name used in help messages
#[command(
  about = "Interactive mode of the peer-to-peer chat.\n\n\
          Use commands like `connect`, `send`, and `peers` to manage peers and messages.\n\
          Type `quit` to exit the interactive session."
)]
struct InteractiveCmdLine {
  #[command(subcommand)]
  command: InteractiveCmdLineCommand,
}

#[derive(clap::Subcommand)]
enum InteractiveCmdLineCommand {
  #[command(flatten)]
  PeerCommand(PeerCommand),
  /// Quit interactive terminal session
  Quit,
}

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
// Interactive terminal
// ========================================================================

fn interactive_mode() -> AppExitCode{
  let mut rl = rustyline::DefaultEditor::new().unwrap();

  loop {
    let readline = rl.readline("> ");
    match readline {
      Ok(cmdline) => {
        let cmdline_args = shlex::split(&cmdline).expect("shlex parsing failed in interactive mode");
        let intrct_cli = match InteractiveCmdLine::try_parse_from(cmdline_args) {
          Ok(parsed) => parsed,
          Err(e) => {
            let _ = e.print(); // TODO: Handle errors from print() gracefully
            continue;
          },
        };

        match intrct_cli.command {
          InteractiveCmdLineCommand::Quit => {
            println!("Exited interactive mode!");
            return AppExitCode::SUCCESS;
          },
          InteractiveCmdLineCommand::PeerCommand(peer_cmd) => {
            handle_peer_command(&peer_cmd);
          }
        }
      },
      Err(RstlnReadlineErr::Interrupted) => {
        println!("\nUse 'quit' to exit");
        continue;
      },
      Err(err) => {
        eprintln!("Error: {:?}\nDying", err); // TODO: Debug representation of an error may not be user-friendly, consider implementing Display for better error messages
        return AppExitCode::FAILURE;
      }
    }
  }
}

// ========================================================================
// Helpers
// ========================================================================

fn handle_peer_command(cmd: &PeerCommand) { // TODO: Impelement error handling
  match cmd {
    PeerCommand::Connect { peer_id } => {
      println!("Connecting to peer: {}", peer_id);
      // TODO: Implement actual connection logic
    }
    PeerCommand::Peers => {
      println!("Connected peers: **TODO**");
      // TODO: Implement peers list command
    }
    PeerCommand::Send { peer_id, message } => {
      println!("Sending to {}: {}", peer_id, message);
      // TODO: Implement actual send logic
    }
  }
}

// ========================================================================
// Tests
// ========================================================================

// TODO: Planned
