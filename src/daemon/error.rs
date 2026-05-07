//! Daemon top-level errors.

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
  // Daemon initialization errors.
  #[error("failed to initialize daemon components")]
  DaemonInitFailed,

  // Connection errors.
  #[error("client aborted connection")]
  ClientAbortedConnection,
  #[error("failed to accept client connection")]
  ClientAcceptFailed,

  // For specific custom errors.
  #[error("error")]
  Other,
}
