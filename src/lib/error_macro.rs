//! Macro for generating standardized error types with custom and simple error variants.

/// # Usage
///
/// ```rust
/// define_error!(Error, ErrorKind);
/// ```
///
/// # Parameters
///
/// - `$error:ident` — name for the generated error struct.
/// - `$kind:ident` — name of a pre-defined error kind enum. Must implement `Copy`, `Debug`,
///   and `Display`. `Copy` is required because `kind()` returns the enum by value.
///
/// # Generates
///
/// - `$error` — error struct with transparent `Display` (delegates to payload or kind).
/// - `impl Display for $error` — displays payload message for custom errors, kind for simple ones.
/// - `impl std::error::Error for $error` — provides `source()` for chained error reporting.
/// - `impl From<$kind> for $error` — allows `?` from bare kind values.
/// - `$error::new(kind, error)` — constructs a custom error with kind and a boxed source.
/// - `$error::kind()` — returns the `$kind` discriminant by value (requires `Copy` on `$kind`).
/// - `$error::source()` — inherent method returning the underlying cause if present; used by
///   the `std::error::Error` impl to avoid needing to import the trait at call sites.
/// - `[<$error Repr>]` — uses `paste` crate to concatenate the error name with "Repr"
///   to allow multiple invocations of macro in the same scope.
///
/// # Note
///
/// Does not generate a `Result` alias. Declare it manually per error type:
/// ```rust
/// pub type Result<T> = std::result::Result<T, Error>;
/// ```
#[macro_export]
macro_rules! define_error {
  ($error:ident, $kind:ident) => {
    paste::paste! {
      #[derive(Debug)]
      pub struct $error([<$error Repr>]);

      impl std::fmt::Display for $error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          match &self.0 {
            [<$error Repr>]::Simple(kind) => write!(f, "{}", kind),
            [<$error Repr>]::Custom { payload, .. } => write!(f, "{}", payload),
          }
        }
      }

      impl std::error::Error for $error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
          $error::source(self)
        }
      }

      impl From<$kind> for $error {
        fn from(kind: $kind) -> Self {
          Self([<$error Repr>]::Simple(kind))
        }
      }

      #[allow(dead_code)]
      impl $error {
        pub(super) fn new<E>(kind: $kind, error: E) -> Self
        where
          E: Into<Box<dyn std::error::Error + Send + Sync>>,
        {
          Self([<$error Repr>]::Custom {
            kind,
            payload: error.into(),
          })
        }

        pub fn kind(&self) -> $kind {
          match &self.0 {
            [<$error Repr>]::Simple(kind) => *kind,
            [<$error Repr>]::Custom { kind, .. } => *kind,
          }
        }

        pub fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
          match &self.0 {
            [<$error Repr>]::Simple(_) => None,
            [<$error Repr>]::Custom { payload, .. } => Some(payload.as_ref()),
          }
        }
      }

      #[allow(dead_code)]
      #[derive(Debug)]
      enum [<$error Repr>] {
        Simple($kind),
        Custom {
          kind: $kind,
          payload: Box<dyn std::error::Error + Send + Sync>,
        },
      }
    }
  };
}
