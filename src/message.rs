use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A message sent between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
  /// Sender's peer ID as string
  pub sender: String,
  /// Unix timestamp of message creation
  pub timestamp: u64,
  /// Message content
  pub content: String,
}

impl Message {
  /// Create a new message with current timestamp
  pub fn new(sender: String, content: String) -> Self {
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|d| d.as_secs())
      .unwrap_or(0);

    Message {
      sender,
      timestamp,
      content,
    }
  }

  /// Format message for display
  pub fn display(&self) -> String {
    let datetime = chrono::DateTime::<chrono::Utc>::from(
      std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp),
    );
    format!(
      "[{}] {}: {}",
      datetime.format("%H:%M:%S"),
      self.sender,
      self.content
    )
  }
}

/// Messages sent over IPC socket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
  /// Command from CLI to daemon
  Command(Command),
  /// Response/notification from daemon to CLI
  Response(Response),
}

/// Commands CLI sends to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
  /// Get current node status
  Status,
  /// List all connected peers
  ListPeers,
  /// Connect to a peer by endpoint
  Connect { endpoint: String },
  /// Send a message to a peer
  Send { peer_id: String, content: String },
  /// Disconnect from daemon
  Quit,
}

/// Responses daemon sends back to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
  /// Status response
  Status {
    node_id: String,
    local_endpoints: Vec<String>,
    peers: Vec<PeerInfo>,
  },
  /// List of peers
  Peers(Vec<PeerInfo>),
  /// Incoming message from a peer
  Message(Message),
  /// Command result (success/error)
  Result { success: bool, message: String },
  /// Error message
  Error(String),
}

/// Information about a connected peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
  pub peer_id: String,
  pub endpoint: String,
  pub connected: bool,
}
