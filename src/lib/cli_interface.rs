//! Command-line interface for the peer-to-peer chat application.

mod validator;

use clap::{Parser, Subcommand};

// INTERNAL_DAEMON_INIT_FLAG is not intended to be seen and used by the user.
// It is used by daemon::control::create(), which spawns a new daemon process
// with this flag set. This approach allows reusing the same binary for both
// the daemon and the CLI.
pub const INTERNAL_DAEMON_INIT_FLAG: &str = "initializedaemoninternalcmd";

// == Main application ==

#[derive(Parser)]
#[command(
  about = "Simple peer-to-peer chat.\n\n\
           Before establishing connections, launch daemon first.\n\
           To start chatting, launch interactive session in a REPL-like environment.",
  arg_required_else_help = true
)]
pub struct Cli {
  /// Internal flag to actualy start daemon (not intended for user use)
  #[arg(long = INTERNAL_DAEMON_INIT_FLAG, hide = true)]
  pub init_daemon_internal: bool,

  /// Log level for client-side logs (`error|warn|info|debug`).
  #[arg(
    long = "cli-log-level",
    global = true,
    default_value = "info",
    value_parser = validator::parse_log_level,
  )]
  pub cli_log_level: String,

  #[command(subcommand)]
  pub command: Option<Command>,
}

#[derive(Subcommand)]
#[non_exhaustive]
pub enum Command {
  /// Daemon control commands
  #[command(flatten_help = true)]
  Daemon {
    #[command(subcommand)]
    subcmd: DaemonCmd,
  },

  /// Client commands
  #[command(flatten_help = true)]
  Client {
    #[command(subcommand)]
    subcmd: ClientCmd,
  },

  #[command(flatten)]
  Info(InfoCmd),
}

#[derive(Subcommand)]
#[non_exhaustive]
pub enum ClientCmd {
  /// PLACEHOLDER (not implemented yet)
  #[command(hide = true)]
  Placeholder,

  /// Start interactive terminal session
  Interactive,
}

#[derive(Subcommand)]
#[non_exhaustive]
pub enum DaemonCmd {
  /// Start the daemon
  Start {
    /// Log level for daemon logs (`error|warn|info|debug`).
    #[arg(
      long = "log-level",
      default_value = "info",
      value_parser = validator::parse_log_level,
    )]
    daemon_log_level: String,
  },

  /// Stop the daemon
  Stop,

  /// Get daemon status
  Status,
}

// == Interactive mode ==

#[derive(Parser, Debug)]
// Tells clap not to expect the first argument to be the program name.
#[command(no_binary_name = true)]
// Unsets the name used in help messages.
#[command(bin_name = "")]
#[non_exhaustive]
#[command(about = "Chat interactive session.")]
pub enum InteractiveCommand {
  #[command(flatten)]
  Communicate(CommunicateCmd),

  #[command(flatten)]
  Info(InfoCmd),

  /// Quit interactive terminal session
  Quit,
}

// == Action commands for daemon ==

#[derive(Subcommand, Debug)]
#[non_exhaustive]
pub enum CommunicateCmd {
  /// Disconnect from the peer
  Disconnect {
    /// Peer ID
    peer_id: String,
  },

  /// Connect to the peer
  Connect {
    /// Peer ticket (shareable value from `myid`)
    peer_id: String,
  },

  /// Send a message to a peer
  Send {
    /// Peer ID
    peer_id: String,

    /// Message content
    message: String,
  },
}

#[derive(Subcommand, Debug)]
#[non_exhaustive]
pub enum InfoCmd {
  /// List connected peers
  List,

  /// Show my ticket for connection
  Myid,
}
