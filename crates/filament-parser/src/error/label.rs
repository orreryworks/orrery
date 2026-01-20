//! Labeled source spans for diagnostic messages.
//!
//! A label associates a message with a span in the source code,
//! providing context for where an error or warning occurred.

use crate::span::Span;

/// A labeled span in source code.
///
/// Labels attach messages to specific locations in the source,
/// helping users understand where problems occurred and why.
///
/// # Primary vs Secondary Labels
///
/// - **Primary labels** mark the main location of an error or warning.
///   There should typically be one primary label per diagnostic.
/// - **Secondary labels** provide additional context, such as "first defined here"
///   or "also referenced here".
///
/// # Example
///
/// ```text
/// error[E301]: type `User` is defined multiple times
///   --> src/main.fil:10:1
///    |
/// 10 | type User = Rectangle;
///    | ^^^^^^^^^^^^^^^^^^^^^^ duplicate definition
///    |
///   --> src/main.fil:5:1
///    |
///  5 | type User = Circle;
///    | ------------------- first defined here
/// ```
#[derive(Debug, Clone)]
pub struct Label {
    span: Span,
    message: String,
    is_primary: bool,
}

impl Label {
    /// Create a new primary label.
    ///
    /// Primary labels mark the main location of an error or warning.
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: true,
        }
    }

    /// Create a new secondary label.
    ///
    /// Secondary labels provide additional context for the diagnostic.
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: false,
        }
    }

    /// Get the span this label applies to.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Get the label message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Check if this is a primary label.
    pub fn is_primary(&self) -> bool {
        self.is_primary
    }

    /// Check if this is a secondary label.
    pub fn is_secondary(&self) -> bool {
        !self.is_primary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_label() {
        let span = Span::new(10..20);
        let label = Label::primary(span, "error here");

        assert_eq!(label.span().start(), 10);
        assert_eq!(label.span().end(), 20);
        assert_eq!(label.message(), "error here");
        assert!(label.is_primary());
        assert!(!label.is_secondary());
    }

    #[test]
    fn test_secondary_label() {
        let span = Span::new(5..15);
        let label = Label::secondary(span, "first defined here");

        assert_eq!(label.span().start(), 5);
        assert_eq!(label.span().end(), 15);
        assert_eq!(label.message(), "first defined here");
        assert!(!label.is_primary());
        assert!(label.is_secondary());
    }
}
