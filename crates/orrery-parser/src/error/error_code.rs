//! Error codes for the Orrery diagnostic system.
//!
//! Error codes are organized by phase:
//! - `E0xx` - Lexer errors
//! - `E1xx` - Parser errors
//! - `E2xx` - Validation errors
//! - `E3xx` - Elaboration errors

use std::fmt;

/// Error codes for categorizing diagnostic errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // =========================================================================
    // Lexer Errors (E0xx)
    // =========================================================================
    /// Unterminated string literal.
    ///
    /// A string was opened with a quote but never closed.
    E001,

    /// Unexpected character.
    ///
    /// A character was encountered that is not valid in this context.
    E002,

    /// Invalid escape sequence.
    ///
    /// An unrecognized escape sequence was used in a string literal.
    /// Valid escapes are: `\n`, `\r`, `\t`, `\b`, `\f`, `\\`, `\/`, `\'`, `\"`, `\0`, `\u{...}`.
    E003,

    /// Invalid unicode escape format.
    ///
    /// A unicode escape sequence was malformed. Unicode escapes must use
    /// the format `\u{XXXX}` with 1-6 hexadecimal digits.
    E004,

    /// Invalid unicode codepoint.
    ///
    /// The unicode codepoint is out of range or in the surrogate range.
    /// Valid codepoints are 0x0000-0xD7FF and 0xE000-0x10FFFF.
    E005,

    /// Empty unicode escape.
    ///
    /// A unicode escape `\u{}` was found with no hexadecimal digits.
    E006,

    // =========================================================================
    // Parser Errors (E1xx)
    // =========================================================================
    /// Unexpected token.
    ///
    /// The parser encountered a token it did not expect at this position.
    E100,

    /// Incomplete input.
    ///
    /// The input ended unexpectedly before a complete construct was parsed.
    E101,

    // =========================================================================
    // Validation Errors (E2xx)
    // =========================================================================
    /// Undefined component reference.
    ///
    /// A component was referenced that has not been defined.
    E200,

    /// Unpaired activate statement.
    ///
    /// An `activate` statement has no matching `deactivate`.
    E201,

    /// Unpaired deactivate statement.
    ///
    /// A `deactivate` statement has no matching `activate`.
    E202,

    /// Invalid align value for diagram type.
    ///
    /// The specified alignment is not valid for this diagram type.
    E203,

    // =========================================================================
    // Elaboration Errors (E3xx)
    // =========================================================================
    /// Undefined type reference.
    ///
    /// A type was referenced that has not been defined.
    E300,

    /// Duplicate type definition.
    ///
    /// A type with this name has already been defined.
    E301,

    /// Invalid attribute value.
    ///
    /// An attribute value is not valid for the expected type.
    E302,

    /// Unknown attribute.
    ///
    /// An attribute was specified that is not recognized.
    E303,

    /// Unsupported attribute for shape/type.
    ///
    /// The attribute is valid but not supported for this particular shape or type.
    E304,

    /// Nested diagram not allowed.
    ///
    /// A diagram definition was found inside another diagram.
    E305,

    /// Invalid diagram structure.
    ///
    /// The diagram structure is invalid or malformed.
    E306,

    /// Type mismatch.
    ///
    /// A type was used in a context where a different kind of type was expected
    /// (e.g., using an Arrow type where a Shape type is required).
    E307,

    /// Shape does not support nested content.
    ///
    /// An attempt was made to add nested content to a shape type that
    /// does not support it.
    E308,

    /// Diagram cannot share scope with other elements.
    ///
    /// A diagram was placed alongside other elements where it must be
    /// the only element in its scope.
    E309,
}

impl ErrorCode {
    /// Returns the numeric code as a string (e.g., "E001").
    pub fn as_str(&self) -> &'static str {
        match self {
            // Lexer errors
            ErrorCode::E001 => "E001",
            ErrorCode::E002 => "E002",
            ErrorCode::E003 => "E003",
            ErrorCode::E004 => "E004",
            ErrorCode::E005 => "E005",
            ErrorCode::E006 => "E006",
            // Parser errors
            ErrorCode::E100 => "E100",
            ErrorCode::E101 => "E101",
            // Validation errors
            ErrorCode::E200 => "E200",
            ErrorCode::E201 => "E201",
            ErrorCode::E202 => "E202",
            ErrorCode::E203 => "E203",
            // Elaboration errors
            ErrorCode::E300 => "E300",
            ErrorCode::E301 => "E301",
            ErrorCode::E302 => "E302",
            ErrorCode::E303 => "E303",
            ErrorCode::E304 => "E304",
            ErrorCode::E305 => "E305",
            ErrorCode::E306 => "E306",
            ErrorCode::E307 => "E307",
            ErrorCode::E308 => "E308",
            ErrorCode::E309 => "E309",
        }
    }

    /// Returns a short description of what this error code means.
    pub fn description(&self) -> &'static str {
        match self {
            // Lexer errors
            ErrorCode::E001 => "unterminated string literal",
            ErrorCode::E002 => "unexpected character",
            ErrorCode::E003 => "invalid escape sequence",
            ErrorCode::E004 => "invalid unicode escape",
            ErrorCode::E005 => "invalid unicode codepoint",
            ErrorCode::E006 => "empty unicode escape",
            // Parser errors
            ErrorCode::E100 => "unexpected token",
            ErrorCode::E101 => "incomplete input",
            // Validation errors
            ErrorCode::E200 => "undefined component",
            ErrorCode::E201 => "unpaired activate",
            ErrorCode::E202 => "unpaired deactivate",
            ErrorCode::E203 => "invalid align value",
            // Elaboration errors
            ErrorCode::E300 => "undefined type",
            ErrorCode::E301 => "duplicate type definition",
            ErrorCode::E302 => "invalid attribute value",
            ErrorCode::E303 => "unknown attribute",
            ErrorCode::E304 => "unsupported attribute",
            ErrorCode::E305 => "nested diagram not allowed",
            ErrorCode::E306 => "invalid diagram structure",
            ErrorCode::E307 => "type mismatch",
            ErrorCode::E308 => "shape does not support nested content",
            ErrorCode::E309 => "diagram cannot share scope",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::E001.to_string(), "E001");
        assert_eq!(ErrorCode::E100.to_string(), "E100");
        assert_eq!(ErrorCode::E200.to_string(), "E200");
        assert_eq!(ErrorCode::E300.to_string(), "E300");
    }

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::E001.as_str(), "E001");
        assert_eq!(ErrorCode::E306.as_str(), "E306");
    }

    #[test]
    fn test_error_code_description() {
        assert_eq!(ErrorCode::E001.description(), "unterminated string literal");
        assert_eq!(ErrorCode::E200.description(), "undefined component");
        assert_eq!(ErrorCode::E301.description(), "duplicate type definition");
    }
}
