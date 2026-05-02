//! Command-line interface and schemas for the peer-to-peer chat application.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

// INTERNAL_DAEMON_INIT_FLAG is not intended to be seen and used by the user.
// It is used by daemon::control::create(), which spawns a new daemon process
// with this flag set. This approach allows reusing the same binary for both
// the daemon and the CLI.
pub const INTERNAL_DAEMON_INIT_FLAG: &str = "initializedaemoninternalcmd";

// == Main application command-line arguments ==

#[derive(Parser)]
#[command(
  about = "Simple peer-to-peer chat.\n\n\
           Before establishing connections, launch daemon first with `daemon` set of commands.\n\
           Use subcommands like `connect`, `send`, and `peers` to manage peers and messages.\n\
           You can also start an interactive terminal session with the `interactive` command, \
           which allows you to enter commands in a REPL-like environment.",
  args_conflicts_with_subcommands = true,
  arg_required_else_help = true,
  override_usage = "p2pchat <COMMAND>"
)]
pub struct Cli {
  #[arg(long = INTERNAL_DAEMON_INIT_FLAG, hide = true)]
  pub init_internal: bool,
  #[command(subcommand)]
  pub command: Option<Command>,
}

#[derive(Subcommand)]
#[non_exhaustive]
pub enum Command {
  /// Peers-related actions
  Peer {
    #[command(subcommand)]
    subcmd: PeerCmd,
  },
  /// Daemon actions
  Daemon {
    #[command(subcommand)]
    subcmd: DaemonCmd,
  },
  /// Start interactive terminal session
  Interactive,
}

// == Interactive mode command-line arguments ==

#[derive(Parser)]
// Tells clap not to expect the first argument to be the program name.
#[command(no_binary_name = true)]
// Unsets the name used in help messages.
#[command(bin_name = "")]
#[non_exhaustive]
#[command(about = "Interactive mode of the peer-to-peer chat.\n\n\
           Use commands like `connect`, `send`, and `peers` to manage peers and messages.\n\
           Type `quit` to exit the interactive session.")]
pub enum InteractiveCommand {
  #[command(flatten)]
  Peer(PeerCmd),
  /// Quit interactive terminal session
  Quit,
}

#[derive(Subcommand)]
#[non_exhaustive]
pub enum DaemonCmd {
  /// Start the daemon
  Start,
  /// Stop the daemon
  Stop,
  /// Get daemon status
  Status,
}

// == Peer-related commands (common for both main application and REPL mode) ==

// NOTE: Consider bringing out all the `rename`s into a single place as consts,
// NOTE: to avoid inconsistencies and make it easier to change command names in the future.
#[derive(Subcommand, Serialize, Deserialize)]
#[serde(tag = "peer_cmd", content = "data")]
#[non_exhaustive]
pub enum PeerCmd {
  /// Connect to the peer
  #[serde(rename = "connect")]
  Connect {
    /// Peer ID
    #[serde(rename = "peer_id")]
    peer_id: String,
  },
  /// List connected peers
  #[serde(rename = "list_peers")]
  List,
  /// Send a message to a peer
  #[serde(rename = "send")]
  Send {
    /// Peer ID
    #[serde(rename = "peer_id")]
    peer_id: String,
    /// Message content
    #[serde(rename = "message")]
    message: String,
  },
}
