//! Error and diagnostic system for the Orrery parser.
//!
//! This module provides an error handling system with:
//! - Error codes for documentation and searchability
//! - Multiple labeled spans for rich error context
//! - Severity levels
//! - Diagnostic collector for accumulating multiple errors
//!
//! # Overview
//!
//! The error system is built around the [`Diagnostic`] type, which represents
//! a single error or warning message with optional error code, multiple source
//! locations, and help text. Multiple diagnostics are wrapped in [`ParseError`]
//! for returning from the parsing lifecycle.
//!
//! # Example
//!
//! ```
//! # use orrery_parser::error::{Diagnostic, ErrorCode};
//! # use orrery_parser::Span;
//!
//! let span = Span::new(100..120);
//! let original_span = Span::new(50..70);
//!
//! let diag = Diagnostic::error("type `User` is defined multiple times")
//!     .with_code(ErrorCode::E301)
//!     .with_label(span, "duplicate definition")
//!     .with_secondary_label(original_span, "first defined here")
//!     .with_help("remove the duplicate or use a different name");
//! ```

mod collector;
mod diagnostic;
mod error_code;
mod label;
mod parse_error;
mod severity;

pub(crate) use collector::DiagnosticCollector;
pub(crate) use parse_error::Result;

pub use diagnostic::Diagnostic;
pub use error_code::ErrorCode;
pub use label::Label;
pub use parse_error::ParseError;
pub use severity::Severity;
