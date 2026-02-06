use serde::{Deserialize, Serialize};

/// A message sent between peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
  pub sender: String,
  pub content: String,
}

impl Message {
  pub fn new(sender: String, content: String) -> Self {
    Message { sender, content }
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
  Connect { endpoint: String },
  Send { peer_id: String, content: String },
}

/// Responses daemon sends back to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
  Message(Message),
  Ok(String),
  Error(String),
}
