use miette::{Diagnostic, SourceSpan};
use nom::error::{ContextError, ErrorKind, ParseError};
use nom_locate::LocatedSpan;
use thiserror::Error;

/// A single error occurrence in the parsing process
#[derive(Debug)]
struct ErrorItem {
    // FIXME: Field not in use.
    /// Input position where the error occurred
    _input: String,
    // FIXME: Field not in use.
    /// The kind of error that occurred
    _kind: ErrorKind,
    /// Optional context information (from the `context` combinator)
    context: Option<&'static str>,
}

/// A detailed error type for nom parsers using nom_locate
#[derive(Debug, Error, Diagnostic)]
#[error("Parse error {message}")]
pub struct ParserError {
    /// The source code being parsed
    #[source_code]
    pub src: String, // FIXME: src should not be part of this error. We don't have full context here.

    /// Error message to display
    pub message: String,

    offset: usize,

    /// The error span in the source
    #[label("here")]
    pub span: Option<SourceSpan>,

    /// Optional help text
    #[help]
    pub help: Option<String>,

    /// The stack of error contexts collected during parsing
    contexts: Vec<ErrorItem>,
}

impl ParserError {
    /// Create a new parser error from a span
    pub fn new(input: LocatedSpan<&str>, kind: ErrorKind) -> Self {
        let offset = input.location_offset();
        let line = input.location_line();
        let column = input.get_column() as u32;
        let fragment = *input.fragment();
        // Store the fragment as the source for error reporting
        let src = fragment.to_string();

        // Use a reasonable span size to show context
        let span_size = std::cmp::min(20, fragment.len());
        let span = Some((offset, span_size).into());

        let mut error = ParserError {
            src,
            message: format!("{:?} error at line {}, column {}", kind, line, column),
            offset,
            span,
            help: None,
            contexts: vec![ErrorItem {
                _input: fragment.to_string(),
                _kind: kind,
                context: None,
            }],
        };

        // Extract the context around the error for more informative messages
        if !fragment.is_empty() {
            let context_snippet = if fragment.len() > 20 {
                format!("...{}", &fragment[..20])
            } else {
                fragment.to_string()
            };
            error.message = format!("{:?} error at \"{}\"", kind, context_snippet);
        }

        error
    }

    /// Add a help message to the error
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

impl<'a> ParseError<LocatedSpan<&'a str>> for ParserError {
    fn from_error_kind(input: LocatedSpan<&'a str>, kind: ErrorKind) -> Self {
        Self::new(input, kind)
    }

    fn append(input: LocatedSpan<&'a str>, kind: ErrorKind, mut other: Self) -> Self {
        // Always keep the position of the first error, but accumulate context
        other.contexts.push(ErrorItem {
            _input: input.fragment().to_string(),
            _kind: kind,
            context: None,
        });

        other
    }

    fn from_char(input: LocatedSpan<&'a str>, c: char) -> Self {
        let mut error = Self::from_error_kind(input, ErrorKind::Char);

        // For character errors, provide a more specific message
        let fragment = *input.fragment();
        let found_char = fragment.chars().next().unwrap_or(' ');

        error.message = format!("Expected '{}' but found '{}'", c, found_char);
        error.help = Some(format!("Expected the character '{}' here", c)); // TODO: This is useless.

        error
    }

    fn or(self, other: Self) -> Self {
        // In branch selection (alt), we prioritize:
        // 1. The error with the most context
        // 2. If equal context, the one at the furthest position
        if self.contexts.len() > other.contexts.len() {
            self
        } else if self.contexts.len() < other.contexts.len() {
            other
        } else if self.offset > other.offset {
            // If we have equal context levels, pick the one that went furthest
            self
        } else {
            other
        }
    }
}

impl<'a> ContextError<LocatedSpan<&'a str>> for ParserError {
    fn add_context(input: LocatedSpan<&'a str>, ctx: &'static str, mut other: Self) -> Self {
        // Add the context to the last error item
        if let Some(last) = other.contexts.last_mut() {
            last.context = Some(ctx);
        }

        // Also add this as a new context item
        other.contexts.push(ErrorItem {
            _input: input.fragment().to_string(),
            _kind: ErrorKind::Tag, // Use an existing ErrorKind as there's no specific Context
            context: Some(ctx),
        });

        // Update the message to include context
        other.message = format!("{} (in {})", other.message, ctx);

        // TODO: Not useful!
        // Add or update help message
        if let Some(ref mut help) = other.help {
            *help = format!("{} - while parsing {}", help, ctx);
        } else {
            other.help = Some(format!("Error occurred while parsing {}", ctx));
        }

        other
    }
}

/// Convert a ParserError into a FilamentError
impl From<ParserError> for crate::error::FilamentError {
    fn from(err: ParserError) -> Self {
        crate::error::FilamentError::ParseDiagnostic(crate::error::ParseDiagnosticError {
            src: err.src,
            message: err.message,
            span: err.span,
            help: err.help,
        })
    }
}
