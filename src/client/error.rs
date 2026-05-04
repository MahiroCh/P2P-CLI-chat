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
  #[error("daemon aborted connection")]
  DaemonAbortedConnection,
  #[error("failed to create session with daemon")]
  DaemonConnectionFailed,
  #[error("daemon refused connection from client")]
  DaemonRefusedConnection,
  #[error("serde failed for action command")]
  SerdeFailed,
  #[error("failed to write action command to daemon")]
  WriteCommandFailed,
  #[error("failed to read daemon response")]
  ReadResponseFailed,

  // REPL errors.
  #[error("repl input cannot be read or parsed into a valid command")]
  ReplReadOrParseFailed,
  #[error("failed to initialize REPL mode")]
  ReplInitFailed,
}
