//! Socket file management errors.

define_error!();

// == Error kinds ==

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ErrorKind {
  // Logical errors.
  #[error("no or invalid parent directory for socket file")]
  ParentDirInvalid,
  #[error("socket connection aborted by counterparty")]
  ConnectionAborted,
  #[error("socket frame too large")]
  FrameTooLarge,
  #[error("socket payload was not valid UTF-8")]
  InvalidUtf8,

  // I/O errors.
  #[error("failed to create parent directory for socket file")]
  CreateParentDir,
  #[error("failed to bind socket listener")]
  BindListener,
  #[error("failed to remove existing socket file")]
  RemoveSocketFile,
  #[error("failed to remove parent directory of socket file")]
  RemoveParentDir,
  #[error("failed to read from socket")]
  ReadFromSocket,
  #[error("failed to write to socket")]
  WriteToSocket,
}

#[derive(thiserror::Error, Debug)]
#[error("this frame is {given} bytes but the maximum is {max} bytes")]
pub(super) struct FrameTooLargeData {
  pub given: u32,
  pub max: u32,
}
