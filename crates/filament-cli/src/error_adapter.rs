//! Error adapter for converting FilamentError to miette diagnostics.
//!
//! This module provides the bridge between the library's standard error types
//! and miette's rich diagnostic formatting used in the CLI.
//!
//! # Multi-Error Support
//!
//! When a [`filament_parser::error::ParseError`] contains multiple diagnostics, each
//! diagnostic is rendered independently.

use std::fmt;

use miette::{Diagnostic as MietteDiagnostic, LabeledSpan, SourceSpan};

use filament::FilamentError;
use filament_parser::error::Diagnostic;

/// Adapter for a single filament diagnostic.
///
/// This adapter wraps a single [`Diagnostic`] and implements
/// [`MietteDiagnostic`] to enable rich error formatting in the CLI.
pub struct DiagnosticAdapter<'a> {
    /// The wrapped diagnostic
    diag: &'a Diagnostic,
    /// Source code for displaying snippets
    src: &'a str,
}

impl<'a> DiagnosticAdapter<'a> {
    /// Create a new diagnostic adapter.
    pub fn new(diag: &'a Diagnostic, src: &'a str) -> Self {
        Self { diag, src }
    }
}

impl fmt::Debug for DiagnosticAdapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiagnosticAdapter")
            .field("diag", &self.diag)
            .finish()
    }
}

impl fmt::Display for DiagnosticAdapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.diag.message())
    }
}

impl std::error::Error for DiagnosticAdapter<'_> {}

impl MietteDiagnostic for DiagnosticAdapter<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.diag
            .code()
            .map(|c| Box::new(c) as Box<dyn fmt::Display>)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.diag
            .help()
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.src as &dyn miette::SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let labels = self.diag.labels();
        if labels.is_empty() {
            return None;
        }

        Some(Box::new(labels.iter().map(|label| {
            let span = span_to_miette(label.span());
            let message = Some(label.message().to_string());
            if label.is_primary() {
                LabeledSpan::new_primary_with_span(message, span)
            } else {
                LabeledSpan::new_with_span(message, span)
            }
        })))
    }
}

/// Adapter for non-diagnostic [`FilamentError`] variants.
///
/// This adapter handles errors that don't have rich diagnostic information,
/// such as I/O errors, graph errors, layout errors, and export errors.
pub struct ErrorAdapter<'a>(pub &'a FilamentError);

impl fmt::Debug for ErrorAdapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for ErrorAdapter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ErrorAdapter<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl MietteDiagnostic for ErrorAdapter<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        let code = match &self.0 {
            FilamentError::Io(_) => "filament::io",
            FilamentError::Parse { .. } => return None,
            FilamentError::Graph(_) => "filament::graph",
            FilamentError::Layout(_) => "filament::layout",
            FilamentError::Export(_) => "filament::export",
        };
        Some(Box::new(code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        None
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        None
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        None
    }
}

/// A reportable error that can be rendered by miette.
///
/// This enum wraps either a single diagnostic or a non-diagnostic error,
/// providing a uniform interface for error rendering.
#[derive(Debug)]
pub enum Reportable<'a> {
    /// A rich diagnostic with source location information.
    Diagnostic(DiagnosticAdapter<'a>),
    /// A simple error without source location.
    Error(ErrorAdapter<'a>),
}

impl fmt::Display for Reportable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reportable::Diagnostic(d) => fmt::Display::fmt(d, f),
            Reportable::Error(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Reportable<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Reportable::Diagnostic(_) => None,
            Reportable::Error(e) => e.source(),
        }
    }
}

impl MietteDiagnostic for Reportable<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match self {
            Reportable::Diagnostic(d) => d.code(),
            Reportable::Error(e) => e.code(),
        }
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match self {
            Reportable::Diagnostic(d) => d.help(),
            Reportable::Error(e) => e.help(),
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match self {
            Reportable::Diagnostic(d) => d.source_code(),
            Reportable::Error(e) => e.source_code(),
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        match self {
            Reportable::Diagnostic(d) => d.labels(),
            Reportable::Error(e) => e.labels(),
        }
    }
}

/// Convert a filament [`Span`](filament_parser::Span) to a miette [`SourceSpan`].
fn span_to_miette(span: filament_parser::Span) -> SourceSpan {
    SourceSpan::new(span.start().into(), span.len())
}

/// Convert a [`FilamentError`] into a list of reportable errors.
///
/// For [`FilamentError::Parse`], this returns one [`Reportable`] for
/// each diagnostic in the error. For other error variants, this returns a
/// single [`Reportable`].
pub fn to_reportables(err: &FilamentError) -> Vec<Reportable<'_>> {
    match err {
        FilamentError::Parse {
            err: parse_err,
            src,
        } => parse_err
            .diagnostics()
            .iter()
            .map(|d| Reportable::Diagnostic(DiagnosticAdapter::new(d, src)))
            .collect(),
        _ => vec![Reportable::Error(ErrorAdapter(err))],
    }
}

#[cfg(test)]
mod tests {
    use filament_parser::{
        Span,
        error::{ErrorCode, ParseError},
    };

    use super::*;

    #[test]
    fn test_single_diagnostic() {
        let diag = Diagnostic::error("test error")
            .with_code(ErrorCode::E300)
            .with_label(Span::new(0..5), "here")
            .with_help("try this");
        let parse_err = ParseError::from(diag);
        let err = FilamentError::new_parse_error(parse_err, "hello");

        let reportables = to_reportables(&err);
        assert_eq!(reportables.len(), 1);

        match &reportables[0] {
            Reportable::Diagnostic(d) => {
                assert_eq!(d.to_string(), "test error");
            }
            Reportable::Error(_) => panic!("Expected Diagnostic"),
        }
    }

    #[test]
    fn test_multiple_diagnostics() {
        let diags = vec![
            Diagnostic::error("first error")
                .with_code(ErrorCode::E300)
                .with_label(Span::new(0..5), "first"),
            Diagnostic::error("second error")
                .with_code(ErrorCode::E301)
                .with_label(Span::new(10..15), "second")
                .with_help("help for second"),
            Diagnostic::error("third error").with_label(Span::new(20..25), "third"),
        ];
        let parse_err = ParseError::from(diags);
        let err = FilamentError::new_parse_error(parse_err, "source code here...");

        let reportables = to_reportables(&err);

        // Each diagnostic is separate
        assert_eq!(reportables.len(), 3);
        assert_eq!(reportables[0].to_string(), "first error");
        assert_eq!(reportables[1].to_string(), "second error");
        assert_eq!(reportables[2].to_string(), "third error");
    }

    #[test]
    fn test_non_parse_error() {
        let err = FilamentError::Graph("graph error".to_string());

        let reportables = to_reportables(&err);

        assert_eq!(reportables.len(), 1);
        match &reportables[0] {
            Reportable::Error(e) => {
                assert_eq!(e.to_string(), "Graph error: graph error");
            }
            Reportable::Diagnostic(_) => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_all_labels_returned() {
        let diag = Diagnostic::error("error with labels")
            .with_label(Span::new(0..5), "primary label")
            .with_secondary_label(Span::new(10..15), "secondary label");

        let adapter = DiagnosticAdapter::new(&diag, "some source code");

        // labels() should return all labels
        let labels: Vec<_> = adapter.labels().unwrap().collect();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].label(), Some("primary label"));
        assert_eq!(labels[1].label(), Some("secondary label"));
    }

    #[test]
    fn test_primary_flag_on_labels() {
        let diag = Diagnostic::error("error with labels")
            .with_label(Span::new(0..5), "primary")
            .with_secondary_label(Span::new(10..15), "secondary");

        let adapter = DiagnosticAdapter::new(&diag, "some source code");

        let labels: Vec<_> = adapter.labels().unwrap().collect();
        assert_eq!(labels.len(), 2);
        // Primary label should be marked as primary
        assert!(labels[0].primary());
        // Secondary label should not be marked as primary
        assert!(!labels[1].primary());
    }
}
