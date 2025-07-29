//! Error message improvement module for the Filament parser
//!
//! This module transforms cryptic chumsky parser errors into user-friendly,
//! actionable error messages.

use chumsky::error::{Rich, RichReason};

use super::span::SpanImpl;
use super::tokens::Token;

/// Common error message constants
pub mod messages {

    pub const UNEXPECTED_END_OF_INPUT: &str = "unexpected end of input";
    pub const UNEXPECTED_TOKEN: &str = "unexpected token";
}

/// Help text for common syntax issues
pub const COMMON_SYNTAX_HELP: &str = "Common syntax issues include:\n\
     • Missing semicolon ';' after statements\n\
     • Missing colon ':' in component definitions (use 'name: Type;')\n\
     • Unmatched brackets '[', ']', '{', '}'\n\
     • Invalid relation syntax (use 'source -> target;')";

/// Improve error messages by making chumsky Rich errors more user-friendly
pub fn improve_error_message(error: &Rich<(Token, SpanImpl)>) -> String {
    match error.reason() {
        RichReason::ExpectedFound {
            expected: _,
            found: None,
        } => messages::UNEXPECTED_END_OF_INPUT.to_string(),
        RichReason::ExpectedFound {
            expected: _,
            found: Some(_),
        } => messages::UNEXPECTED_TOKEN.to_string(),
        RichReason::Custom(msg) => msg.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constants() {
        assert_eq!(messages::UNEXPECTED_END_OF_INPUT, "unexpected end of input");
        assert_eq!(messages::UNEXPECTED_TOKEN, "unexpected token");
    }
}
