use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};

pub const INTERNAL_DAEMON_INIT_FLAG: &str = "initializedaemoninternalcmd";
pub const DAEMON_NAME: &str = "daemon";

/* Command-line arguments for the main application: */

#[derive(Parser)]
#[command(
  about = "Simple peer‑to‑peer chat.\n\n\
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

/* Command-line arguments for the interactive terminal: */

#[derive(Parser)]
#[command(no_binary_name=true)] // tells clap not to expect the first argument to be the program name
#[command(bin_name="")] // unsets the name used in help messages
#[command(
  about = "Interactive mode of the peer-to-peer chat.\n\n\
          Use commands like `connect`, `send`, and `peers` to manage peers and messages.\n\
          Type `quit` to exit the interactive session."
)]
pub enum InteractiveCommand {
  #[command(flatten)]
  Peer(PeerCmd),
  
  /// Quit interactive terminal session
  Quit,
}


#[derive(Subcommand)]
pub enum DaemonCmd {
  /// Start the daemon
  Start,
  /// Stop the daemon
  Stop,
  /// Get daemon status
  Status,
}

/* Common commands for both interactive and non-interactive modes: messaging commands: */

#[derive(Subcommand)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "peer_cmd", content = "data")]
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