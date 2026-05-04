//! Daemon top-level errors.

use p2p_chat::define_error;

define_error!();

// == Error kinds ==

#[allow(dead_code)]
#[derive(thiserror::Error, Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ErrorKind {
  // Initialization errors.
  #[error("failed to create PID file")]
  PidFileCreationFailed,
  #[error("failed to create socket file")]
  SocketCreationFailed,
  #[error("daemon signal handler failed to initialize")]
  SignalHandlerFailed,

  // Connection errors.
  #[error("client aborted connection")]
  ClientAbortedConnection,
  #[error("daemon failed to accept a connection")]
  ConnectionAcceptFailed,
  #[error("connection rejected: maximum concurrent connections reached")]
  ConnectionAtCapacity,

  // Communication with client errors.
  #[error("serde failed for action command")]
  SerdeFailed,
  #[error("daemon failed to read a command from client connection")]
  ReadCommandFailed,
  #[error("daemon failed to write a response to client connection")]
  WriteResponseFailed,
}
