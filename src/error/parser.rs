use miette::{Diagnostic, SourceSpan};
use nom::error::{ContextError, ErrorKind, ParseError};
use nom_locate::LocatedSpan;
use std::rc::Rc;
use thiserror::Error;

/// A rich diagnostic error for parsing
#[derive(Debug, Error, Diagnostic)]
#[error("Parse error: {message}")]
pub struct ParseDiagnosticError {
    /// The source code being parsed
    #[source_code]
    pub src: String,

    /// Error message to display
    pub message: String,

    /// The error span in the source
    #[label("here")]
    pub span: Option<SourceSpan>,

    /// Optional help text
    #[help]
    pub help: Option<String>,
}

/// A slim error type for use during parsing
#[derive(Debug, Clone)]
pub struct SlimParserError {
    /// Current offset in the input
    pub offset: usize,

    /// Line number where the error occurred
    line: u32,

    /// Column number where the error occurred
    column: u32,

    /// Error kind from nom
    pub kind: ErrorKind,

    /// Context information, shared to avoid duplication
    context: Rc<Vec<&'static str>>,

    /// Span length to highlight in the error
    span_len: usize,
}

impl SlimParserError {
    /// Create a new slim error from a span
    pub fn new(input: LocatedSpan<&str>, kind: ErrorKind) -> Self {
        SlimParserError {
            offset: input.location_offset(),
            line: input.location_line(),
            column: input.get_column() as u32,
            kind,
            context: Rc::new(Vec::new()),
            span_len: 1,
        }
    }

    /// Convert to a full error with complete information
    pub fn move_to_full_error(self, input_str: &str) -> ParseDiagnosticError {
        // Calculate the right span length based on the available input
        let span_len = std::cmp::min(self.span_len, input_str.len().saturating_sub(self.offset));

        // Build the context string
        let context_str = if self.context.is_empty() {
            String::new()
        } else {
            let mut result = String::from(" (in ");
            let contexts: Vec<_> = self.context.iter().rev().collect();

            // Manual string joining
            for (i, ctx) in contexts.iter().enumerate() {
                if i > 0 {
                    result.push_str(" -> ");
                }
                result.push_str(ctx);
            }
            result.push(')');
            result
        };

        // Extract the character that was found (if available)
        let found_char = input_str
            .get(self.offset..)
            .and_then(|s| s.chars().next())
            .map_or_else(|| "end of input".into(), |c| format!("found '{c}'"));

        // Create a helpful message
        let message = match self.kind {
            ErrorKind::Char => {
                format!("Expected a specific character but {found_char}{context_str}")
            }
            _ => format!(
                "{:?} error at line {}, column {}{context_str}",
                self.kind, self.line, self.column,
            ),
        };

        // Build the help message
        let help = if self.context.is_empty() {
            None
        } else {
            let contexts: Vec<_> = self.context.iter().rev().collect();
            let mut help_text = String::from("Error occurred while parsing ");

            // Manual string joining for help text
            for (i, ctx) in contexts.iter().enumerate() {
                if i > 0 {
                    help_text.push_str(" -> ");
                }
                help_text.push_str(ctx);
            }

            Some(help_text)
        };

        ParseDiagnosticError {
            src: input_str.to_string(),
            message,
            span: Some((self.offset, span_len).into()),
            help,
        }
    }
}

impl<'a> ParseError<LocatedSpan<&'a str>> for SlimParserError {
    fn from_error_kind(input: LocatedSpan<&'a str>, kind: ErrorKind) -> Self {
        Self::new(input, kind)
    }

    fn append(_input: LocatedSpan<&'a str>, _kind: ErrorKind, other: Self) -> Self {
        // Just return the original error - we don't use append for extra context
        // (we use ContextError::add_context for that instead)
        SlimParserError {
            offset: other.offset,
            line: other.line,
            column: other.column,
            kind: other.kind,
            context: other.context,
            span_len: other.span_len,
        }
    }

    fn from_char(input: LocatedSpan<&'a str>, _c: char) -> Self {
        // We'll handle the specific character formatting in to_full_error
        Self::from_error_kind(input, ErrorKind::Char)
    }

    fn or(self, other: Self) -> Self {
        // Select the error that progressed further or has more context
        if self.context.len() > other.context.len() {
            self
        } else if other.context.len() > self.context.len() {
            other
        } else if self.offset >= other.offset {
            self
        } else {
            other
        }
    }
}

impl<'a> ContextError<LocatedSpan<&'a str>> for SlimParserError {
    fn add_context(_input: LocatedSpan<&'a str>, ctx: &'static str, other: Self) -> Self {
        // Clone the context and add the new entry
        let mut contexts = (*other.context).clone();
        contexts.push(ctx);

        SlimParserError {
            offset: other.offset,
            line: other.line,
            column: other.column,
            kind: other.kind,
            context: Rc::new(contexts),
            span_len: other.span_len,
        }
    }
}
