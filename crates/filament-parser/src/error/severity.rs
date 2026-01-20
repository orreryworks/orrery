//! Severity levels for diagnostics.
//!
//! This module defines the severity of diagnostic messages,
//! distinguishing between fatal errors and advisory warnings.

use std::fmt;

/// The severity level of a diagnostic.
///
/// Severity determines how the diagnostic should be handled:
/// - [`Severity::Error`] indicates a fatal issue that must be fixed
/// - [`Severity::Warning`] indicates an advisory issue that should be addressed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// A fatal error that prevents successful compilation.
    ///
    /// Errors must be fixed before the diagram can be processed.
    Error,

    /// A non-fatal warning about potential issues.
    ///
    /// Warnings indicate code that may be problematic but doesn't
    /// prevent compilation from succeeding.
    Warning,
}

impl Severity {
    /// Returns `true` if this is an error severity.
    pub fn is_error(&self) -> bool {
        matches!(self, Severity::Error)
    }

    /// Returns `true` if this is a warning severity.
    pub fn is_warning(&self) -> bool {
        matches!(self, Severity::Warning)
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}
