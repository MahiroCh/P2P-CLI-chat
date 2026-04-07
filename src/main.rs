// TODO: Implement normal error handling

#![allow(unused)] // TODO: Remove this when the code is fully implemented

mod service;

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

  match cmd.command { // TDOO: Handle the exit code and errors more gracefully
    CmdLineCommand::Interactive => { interactive_mode(); }, // TODO: Implement protection against entering REPL without a running daemon --- IGNORE ---

    CmdLineCommand::PeerCommand(peer_cmd) => { handle_peer_command(&peer_cmd); }, 

    CmdLineCommand::Daemon { command } => {
      match command {
        DaemonCommand::Start { initialize } => {
          if initialize { service::daemon(); } 
          else { service::start_daemon(); }
        },  
        
        DaemonCommand::Connect => { service::connect_daemon(); },

        DaemonCommand::Stop => { service::stop_daemon(); },

        DaemonCommand::Restart => { service::restart_daemon(); },
  
        DaemonCommand::Status => { service::daemon_status(); }
      }
    }
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
  /// Start the daemon and connect to it
  Start {
    #[arg(long = "initialize", hide = true)]
    initialize: bool,
  },
  /// Connect to an already running daemon
  Connect,
  /// Stop the daemon
  Stop,
  /// Restart the daemon and connect to it
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
            let _ = e.print(); // TODO: Handle errors
            continue;
          },
        };

        match intrct_cli.command {
          InteractiveCmdLineCommand::Quit => {
            println!("Exited interactive mode");
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

fn handle_peer_command(cmd: &PeerCommand) {
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
