//! Macro for generating standardized error types with custom and simple error variants.
//!
//! This macro eliminates boilerplate by generating:
//! - Result<T> type alias
//! - Error struct with transparent display
//! - Error::new() and Error::from() methods
//! - Repr enum for Custom/Simple variants
//! - Error and Display trait impls
//! - CustomErr struct for payload handling

#[macro_export]
macro_rules! define_error {
  () => {
    #[allow(dead_code)]
    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(thiserror::Error, Debug)]
    #[error(transparent)]
    pub struct Error(#[from] Repr);

    #[allow(dead_code)]
    impl Error {
      pub(super) fn new<E>(kind: ErrorKind, error: E) -> Self
      where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
      {
        Repr::Custom(Box::new(CustomErr {
          kind,
          payload: error.into(),
        }))
        .into()
      }

      pub(super) fn from(kind: ErrorKind) -> Self {
        Repr::Simple(kind).into()
      }

      pub fn kind(&self) -> ErrorKind {
        match &self.0 {
          Repr::Custom(custom) => custom.kind,
          Repr::Simple(kind) => *kind,
        }
      }
    }

    #[allow(dead_code)]
    #[derive(Debug)]
    enum Repr {
      Simple(ErrorKind),
      Custom(Box<CustomErr>),
    }

    impl std::error::Error for Repr {}
    impl std::fmt::Display for Repr {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
          Repr::Custom(custom) => write!(f, "{}: {}", custom.kind, custom.payload),
          Repr::Simple(kind) => write!(f, "{kind}"),
        }
      }
    }

    #[derive(Debug)]
    struct CustomErr {
      kind: ErrorKind,
      payload: Box<dyn std::error::Error + Send + Sync>,
    }
  };
}
