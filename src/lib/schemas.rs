//! Schemas of interactions between daemon and CLI, daemon and network.

use crate::cli_interface::{CommunicateCmd, InfoCmd, InteractiveCommand};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", content = "payload")]
#[non_exhaustive]
pub enum ClientRequest {
  /// Disconnect from the peer by its endpoint ID.
  #[serde(rename = "disconnect")]
  Disconnect {
    /// Peer endpoint ID.
    #[serde(rename = "peer_id")]
    peer_id: String,
  },

  /// Connect to a peer by its shareable ticket.
  #[serde(rename = "connect")]
  Connect {
    /// Peer ticket string.
    #[serde(rename = "peer_id")]
    peer_id: String,
  },

  /// Send a text message to a peer we are connected to.
  #[serde(rename = "send")]
  Send {
    /// Peer endpoint ID.
    #[serde(rename = "peer_id")]
    peer_id: String,

    /// Message content (free-form UTF-8).
    #[serde(rename = "message")]
    message: String,
  },

  /// List peers currently connected to this daemon.
  #[serde(rename = "list_peers")]
  List,

  /// Show this daemon's own ticket so the user can share it with peers.
  #[serde(rename = "my_id")]
  MyID,

  /// REPL is about to exit — daemon should release any per-client state.
  /// The daemon keeps running (it's a persistent background service).
  #[serde(rename = "bye")]
  Bye,
}

impl From<InteractiveCommand> for ClientRequest {
  fn from(command: InteractiveCommand) -> Self {
    match command {
      InteractiveCommand::Communicate(CommunicateCmd::Disconnect { peer_id }) => {
        Self::Disconnect { peer_id }
      }
      InteractiveCommand::Communicate(CommunicateCmd::Connect { peer_id }) => {
        Self::Connect { peer_id }
      }
      InteractiveCommand::Communicate(CommunicateCmd::Send { peer_id, message }) => {
        Self::Send { peer_id, message }
      }
      InteractiveCommand::Info(InfoCmd::List) => Self::List,
      InteractiveCommand::Info(InfoCmd::Myid) => Self::MyID,
      InteractiveCommand::Quit => Self::Bye,
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", content = "payload")]
#[non_exhaustive]
pub enum DaemonEvent {
  // --- Responses to requests ---
  /// Answer to `ActionCmd::MyID`: daemon reports its own endpoint ID.
  #[serde(rename = "my_id")]
  MyId { endpoint_id: String },

  /// Answer to `ActionCmd::List`: list of peer IDs currently connected.
  #[serde(rename = "peer_list")]
  PeerList { peers: Vec<String> },

  /// Acknowledges that a command was accepted by the daemon. `info` holds a
  /// human-readable message the REPL can print verbatim.
  #[serde(rename = "ok")]
  Ok { info: String },

  /// Reports a non-fatal error with a user-facing message. The REPL is
  /// expected to display it and keep running.
  #[serde(rename = "error")]
  Error { message: String },

  // --- Unsolicited notifications ---
  /// A new peer has connected (incoming or outgoing) and is ready to chat.
  #[serde(rename = "peer_connected")]
  PeerConnected { peer_id: String },

  /// A peer has disconnected.
  #[serde(rename = "peer_disconnected")]
  PeerDisconnected { peer_id: String },

  /// A chat message has been received from a peer. This is THE event that
  /// must appear in the REPL "live" — that is why we need `rustyline-async`
  /// on the client sIDe.
  #[serde(rename = "peer_message")]
  PeerMessage { peer_id: String, message: String, timestamp_secs: i64 },
}

impl From<NetEvent> for DaemonEvent {
  fn from(event: NetEvent) -> Self {
    match event {
      NetEvent::PeerConnected { peer_id } => DaemonEvent::PeerConnected { peer_id },
      NetEvent::PeerDisconnected { peer_id } => {
        DaemonEvent::PeerDisconnected { peer_id }
      }
      NetEvent::PeerMessage { peer_id, message, timestamp_secs } => {
        DaemonEvent::PeerMessage { peer_id, message, timestamp_secs }
      }
    }
  }
}

#[derive(Debug, Clone)]
pub enum NetEvent {
  /// A new peer is connected and ready to exchange messages.
  PeerConnected { peer_id: String },

  /// A previously-connected peer went away (gracefully or not).
  PeerDisconnected { peer_id: String },

  /// A chat message arrived from `peer_id`.
  PeerMessage { peer_id: String, message: String, timestamp_secs: i64 },
}
