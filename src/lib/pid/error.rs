//! PID file management errors.

define_error!();

// == Error kinds ==

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ErrorKind {
  // Logical errors:
  #[error("PID file not found")]
  PidFileNotFound,
  #[error("invalid PID file content")]
  InvalidPidFileContent,
  #[error("no or invalid parent directory for PID file")]
  ParentDirInvalid,

  // I/O errors:
  #[error("failed to create parent directory for PID file")]
  CreateParentDir,
  #[error("failed to create PID file")]
  CreatePidFile,
  #[error("failed to write to PID file")]
  WriteToPidFile,
  #[error("failed to read from PID file")]
  ReadFromPidFile,
  #[error("failed to remove PID file")]
  RemovePidFile,
  #[error("failed to remove parent directory of PID file")]
  RemoveParentDir,
}
