use clap::Parser;

/* Command-line arguments for the main application: */

#[derive(Parser)]
#[command(
  about = "Simple peer‑to‑peer chat.\n\n\
          Before establishing connections, launch daemon first with `daemon` set of commands.\n\
          Use subcommands like `connect`, `send`, and `peers` to manage peers and messages.\n\
          You can also start an interactive terminal session with the `interactive` command, \
          which allows you to enter commands in a REPL-like environment."
)]
pub struct Cmdline {
  #[command(subcommand)]
  pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
  #[command(flatten)]
  PeerCommand(PeerCmd),
  /// Daemon actions
  Daemon {
    #[command(subcommand)]
    command: DaemonCmd,
  },
  /// Start interactive terminal session
  Interactive,
}

#[derive(clap::Subcommand)]
pub enum DaemonCmd {
  /// Start the daemon
  Start {
    #[arg(long = "initialize", hide = true)]
    initialize: bool,
  },
  /// Stop the daemon
  Stop,
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
pub struct InteractiveCmdline {
  #[command(subcommand)]
  pub command: InteractiveCommand,
}

#[derive(clap::Subcommand)]
pub enum InteractiveCommand {
  #[command(flatten)]
  PeerCommand(PeerCmd),
  /// Quit interactive terminal session
  Quit,
}

/* Common commands for both interactive and non-interactive modes: messaging commands: */

#[derive(clap::Subcommand)]
pub enum PeerCmd {
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