//! Client top-level errors.

use p2p_chat::define_error;

define_error!();

// == Error kinds ==

#[allow(dead_code)]
#[derive(thiserror::Error, Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ErrorKind {
  // Daemon control errors.
  #[error("failed to spawn separate daemon process")]
  DaemonStartFailed,
  #[error("failed to stop daemon for an unknown reason")]
  DaemonStopFailed,
  #[error("failed to obtain daemon state")]
  DaemonStateUnknown,
  #[error("daemon is corrupted")]
  DaemonCorrupted,
  #[error("daemon is not running but it is required")]
  DaemonNotRunningButNeeded,

  // Communication with daemon errors.
  #[error("failed to create session with daemon")]
  DaemonConnectionFailed,
  #[error("peer command error")]
  PeerCommandFailed,
  #[error("serde failed for peer command")]
  SerdeFailed,
  #[error("failed to write peer command to daemon")]
  WriteCommandFailed,
  #[error("failed to read daemon response")]
  ReadResponseFailed,

  // REPL errors.
  #[error("repl engine initialization error")]
  ReplFailed,
  #[error("failed to read input in REPL mode")]
  ReplReadCliFailed,
}
