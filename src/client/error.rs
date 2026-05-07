//! Client top-level errors.

use p2p_chat::define_error;

define_error!(Error, ErrorKind);

impl Error {
  pub fn other<E>(error: E) -> Self
  where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
  {
    Self::new(ErrorKind::Other, error)
  }
}

#[allow(dead_code)]
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
  // Connection errors.
  #[error("daemon aborted connection")]
  DaemonAbortedConnection,
  #[error("failed to create session with daemon")]
  DaemonConnectionFailed,

  // Communication with daemon errors.
  #[error("failed to send command to daemon")]
  SendCommandFailed,
  #[error("failed to receive daemon response")]
  ReceiveResponseFailed,

  // REPL errors.
  #[error("repl initialization failed")]
  ReplInitFailed,
  #[error("failed to parse input: shlex split error")]
  ShlexFailed,
  #[error("failed to parse input: clap parsing error")]
  ClapFailed,

  // For specific custom errors.
  #[error("error")]
  Other,
}
