//! Error adapter for converting FilamentError to miette diagnostics.
//!
//! This module provides the bridge between the library's standard error types
//! and miette's rich diagnostic formatting used in the CLI.

use std::fmt;

use miette::{Diagnostic, LabeledSpan, SourceSpan};

use filament::{FilamentError, ast::span::Span};

/// Adapter that wraps [`FilamentError`] and implements [`miette::Diagnostic`].
///
/// This adapter provides the bridge between the library's standard error types
/// and miette's rich diagnostic formatting. It converts library spans to miette
/// spans, provides source code for diagnostic variants, and formats help messages.
pub struct ErrorAdapter(pub FilamentError);

impl fmt::Debug for ErrorAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for ErrorAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ErrorAdapter {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl Diagnostic for ErrorAdapter {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match &self.0 {
            FilamentError::Io(_) => Some(Box::new("filament::error::io")),
            FilamentError::LexerDiagnostic { .. }
            | FilamentError::ParseDiagnostic { .. }
            | FilamentError::ElaborationDiagnostic { .. }
            | FilamentError::ValidationDiagnostic { .. } => {
                // Suppress error code for diagnostic errors
                None
            }
            FilamentError::Graph(_) => Some(Box::new("filament::error::graph")),
            FilamentError::Layout(_) => Some(Box::new("filament::error::layout")),
            FilamentError::Export(_) => Some(Box::new("filament::error::export")),
        }
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        match &self.0 {
            FilamentError::LexerDiagnostic { err, .. }
            | FilamentError::ParseDiagnostic { err, .. }
            | FilamentError::ElaborationDiagnostic { err, .. }
            | FilamentError::ValidationDiagnostic { err, .. } => {
                err.help().map(|h| Box::new(h) as Box<dyn fmt::Display>)
            }
            _ => None,
        }
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        match &self.0 {
            FilamentError::LexerDiagnostic { src, .. }
            | FilamentError::ParseDiagnostic { src, .. }
            | FilamentError::ElaborationDiagnostic { src, .. }
            | FilamentError::ValidationDiagnostic { src, .. } => {
                Some(src as &dyn miette::SourceCode)
            }
            _ => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        match &self.0 {
            FilamentError::LexerDiagnostic { err, .. }
            | FilamentError::ParseDiagnostic { err, .. }
            | FilamentError::ElaborationDiagnostic { err, .. }
            | FilamentError::ValidationDiagnostic { err, .. } => {
                let span = err.span();
                let miette_span = span_to_miette(span);
                let label = err.label().to_string();

                Some(Box::new(std::iter::once(LabeledSpan::new_with_span(
                    Some(label),
                    miette_span,
                ))))
            }
            _ => None,
        }
    }
}

/// Convert a filament [`Span`] to a miette [`SourceSpan`].
///
/// This function bridges the library's span representation with miette's
/// formatting requirements, allowing diagnostic errors to display source
/// code snippets with precise location information.
///
/// # Arguments
///
/// * `span` - The library span
///
/// # Returns
///
/// A miette [`SourceSpan`] suitable for use in diagnostic labels.
fn span_to_miette(span: Span) -> SourceSpan {
    SourceSpan::new(span.start().into(), span.len())
}
