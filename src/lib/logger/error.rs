//! Logger errors.

define_error!();

// == Error kinds ==

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ErrorKind {
  #[error("failed to initialize logger")]
  LoggerInitFailed,
}