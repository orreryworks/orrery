//! Error type for [`SourceProvider`](crate::source_provider::SourceProvider) operations.
//!
//! [`SourceError`] is a lightweight, `Clone`-able error representing failures
//! in file resolution or reading. It is intentionally decoupled from the
//! diagnostic system so that the resolver can convert it into a
//! [`Diagnostic`](super::Diagnostic) with proper span and error-code context.

use std::{
    fmt,
    path::{Path, PathBuf},
};

/// Error returned by [`SourceProvider`](crate::source_provider::SourceProvider) operations.
///
/// `SourceError` is intentionally lightweight and `Clone`-able so it can be
/// stored, duplicated, and later converted into a [`Diagnostic`](super::Diagnostic)
/// by the resolver with proper span and error-code context.
///
/// # Example
///
/// ```
/// # use orrery_parser::error::SourceError;
/// let err = SourceError::new("shared/styles.orr", "file not found");
/// assert_eq!(err.path().to_str().unwrap(), "shared/styles.orr");
/// assert_eq!(err.message(), "file not found");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceError {
    /// The path that caused the error.
    path: PathBuf,
    /// Human-readable description of what went wrong.
    message: String,
}

impl SourceError {
    /// Creates a new source error.
    ///
    /// # Arguments
    ///
    /// * `path` - The path that caused the error.
    /// * `message` - Human-readable description of what went wrong.
    pub fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Returns the path associated with this error.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the human-readable error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for SourceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_error_accessors() {
        let err = SourceError::new("test.orr", "something went wrong");
        assert_eq!(err.path(), Path::new("test.orr"));
        assert_eq!(err.message(), "something went wrong");
    }

    #[test]
    fn source_error_display() {
        let err = SourceError::new("shared/styles.orr", "file not found");
        assert_eq!(err.to_string(), "shared/styles.orr: file not found");
    }
}
