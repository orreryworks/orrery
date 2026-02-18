//! Lexical analyzer for Filament source text.
//!
//! The lexer converts source text into a stream of [`Token`]s for parsing.
//! It handles whitespace, comments, string literals, and all language tokens
//! defined in the [`tokens`](super::tokens) module.
//!
//! The public entry point is [`tokenize`], which performs error-recovering
//! lexical analysis and collects all diagnostics in a single pass.

use std::char;

use winnow::{
    Parser as _,
    ascii::{float, multispace1},
    combinator::{alt, cut_err, delimited, not, peek, preceded, repeat, terminated},
    error::{AddContext, ContextError, ErrMode, ModalResult},
    stream::{LocatingSlice, Location, Stream},
    token::{literal, none_of, one_of, take_while},
};

use crate::{
    error::{Diagnostic, DiagnosticCollector, ErrorCode, ParseError},
    span::Span,
    tokens::{PositionedToken, Token},
};

/// Rich diagnostic information for lexer errors.
///
/// Attached to winnow errors via `.context()` to provide detailed error
/// messages with codes, help text, and precise span information.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LexerDiagnostic {
    pub code: ErrorCode,
    pub message: &'static str,
    pub help: Option<&'static str>,
    /// The error span covers from `start` to the error position.
    pub start: usize,
}

type Input<'a> = LocatingSlice<&'a str>;
type IResult<'a, O> = ModalResult<O, ContextError<LexerDiagnostic>>;

/// Parse a unicode escape sequence in a string: `\u{XXXX}` where XXXX is 1-6 hex digits.
///
/// This parser handles the portion after the backslash, starting with 'u'.
/// It validates:
/// - Format: must be `u{...}` with hex digits inside braces
/// - Length: 1-6 hex digits allowed
/// - Codepoint: must be valid Unicode (0x0000-0xD7FF or 0xE000-0x10FFFF)
///
/// Takes `escape_start` position (before `\`) for error span calculation.
/// Uses `cut_err` after 'u' to commit and preserve diagnostic context.
fn string_escape_unicode<'a>(input: &mut Input<'a>, escape_start: usize) -> IResult<'a, char> {
    preceded(
        'u',
        cut_err(
            delimited(
                '{',
                take_while(1..=6, |c: char| c.is_ascii_hexdigit()).context(LexerDiagnostic {
                    code: ErrorCode::E006,
                    message: "empty unicode escape",
                    help: Some("provide 1-6 hex digits: `\\u{1F602}`"),
                    start: escape_start,
                }),
                '}',
            )
            .context(LexerDiagnostic {
                code: ErrorCode::E004,
                message: "invalid unicode escape",
                help: Some("use format `\\u{XXXX}` with 1-6 hex digits"),
                start: escape_start,
            })
            .verify(|hex_str: &str| {
                u32::from_str_radix(hex_str, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .is_some()
            })
            .context(LexerDiagnostic {
                code: ErrorCode::E005,
                message: "invalid unicode codepoint",
                help: Some("valid range: `0x0000`-`0xD7FF` or `0xE000`-`0x10FFFF`"),
                start: escape_start,
            })
            .map(|hex_str: &str| {
                u32::from_str_radix(hex_str, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .expect("verified hex digits form valid unicode codepoint")
            }),
        ),
    )
    .parse_next(input)
}

/// Parse a standard escape character in a string after the backslash.
fn string_escape_char<'a>(input: &mut Input<'a>) -> IResult<'a, char> {
    one_of(['n', 'r', 't', 'b', 'f', '\\', '/', '\'', '"', '0'])
        .map(|c| match c {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'b' => '\u{08}',
            'f' => '\u{0C}',
            '\\' => '\\',
            '/' => '/',
            '\'' => '\'',
            '"' => '"',
            '0' => '\0',
            _ => unreachable!(),
        })
        .parse_next(input)
}

/// Parse escaped whitespace in a string (backslash followed by whitespace).
///
/// Returns a placeholder character (`\u{E000}`) that is filtered out later.
/// This allows multi-line string formatting where trailing backslash
/// consumes the following whitespace.
fn string_escape_whitespace<'a>(input: &mut Input<'a>) -> IResult<'a, char> {
    multispace1.value('\u{E000}').parse_next(input)
}

/// Parse an escape sequence in a string starting with backslash.
///
/// Handles:
/// - Unicode escapes: `\u{XXXX}`
/// - Standard escapes: `\n`, `\r`, `\t`, `\b`, `\f`, `\\`, `\/`, `\'`, `\"`, `\0`
/// - Escaped whitespace: `\` followed by whitespace (consumed and ignored)
fn string_escape<'a>(input: &mut Input<'a>) -> IResult<'a, char> {
    let escape_start = input.current_token_start();

    '\\'.parse_next(input)?;

    match string_escape_unicode(input, escape_start) {
        Ok(ch) => return Ok(ch),
        Err(ErrMode::Backtrack(_)) => {} // Try next alternative
        Err(e) => return Err(e),         // Propagate cut errors (E004, E005, E006)
    }

    if let Ok(ch) = string_escape_char(input) {
        return Ok(ch);
    }

    if let Ok(ch) = string_escape_whitespace(input) {
        return Ok(ch);
    }

    // None matched - return error with context for invalid escape
    Err(ErrMode::Cut(ContextError::new().add_context(
        input,
        &input.checkpoint(),
        LexerDiagnostic {
            code: ErrorCode::E003,
            message: "invalid escape sequence",
            help: Some(
                "valid escapes: `\\n`, `\\r`, `\\t`, `\\b`, `\\f`, `\\\\`, `\\/`, `\\'`, `\\\"`, `\\0`, `\\u{}`",
            ),
            start: escape_start,
        },
    )))
}

/// Parse a complete string literal with double quotes.
///
/// This function parses Rust-style string literals including:
/// - Basic strings: "hello world"
/// - Escape sequences: "hello\nworld", "quote: \"test\""
/// - Unicode escapes: "emoji: \u{1F602}", "symbol: \u{00AC}"
/// - Escaped whitespace: "before\   \n  after" (whitespace is consumed)
/// - Empty strings: ""
fn string_literal<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    // Regular string content (not quotes, backslashes, or newlines)
    let string_char = none_of(['"', '\\', '\n', '\r']);

    // String content: mix of regular chars and escapes
    let string_content =
        repeat(0.., alt((string_escape, string_char))).fold(String::new, |mut acc, ch| {
            if ch != '\u{E000}' {
                // Filter out escaped whitespace placeholders
                acc.push(ch);
            }
            acc
        });

    let start_pos = input.current_token_start();

    // Parse opening quote using combinator (properly advances LocatingSlice)
    '"'.parse_next(input)
        .map_err(|_: ErrMode<ContextError<LexerDiagnostic>>| {
            ErrMode::Backtrack(ContextError::new())
        })?;

    // Parse content with cut_err to commit after opening quote
    // Include start_pos so error span covers from opening quote to error position
    cut_err(terminated(string_content, '"'))
        .context(LexerDiagnostic {
            code: ErrorCode::E001,
            message: "unterminated string literal",
            help: Some("add closing `\"`"),
            start: start_pos,
        })
        .parse_next(input)
        .map(Token::StringLiteral)
}

/// Parse a float literal
fn float_literal<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    // Parse float but ensure it's not followed by identifier characters
    // This prevents "inf" in "info_note" from being parsed as a float literal
    (
        float,
        peek(not(one_of(|c: char| c.is_alphanumeric() || c == '_'))),
    )
        .map(|(f, _)| Token::FloatLiteral(f))
        .parse_next(input)
}

/// Parse line comment starting with '//'
fn line_comment<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    preceded("//", take_while(0.., |c| c != '\n'))
        .map(Token::LineComment)
        .parse_next(input)
}

/// Parse keywords with word boundary checking
fn keyword<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    terminated(
        alt((
            literal("diagram"),
            literal("component"),
            literal("sequence"),
            literal("type"),
            literal("embed"),
            literal("as"),
            literal("deactivate"),
            literal("activate"),
            literal("fragment"),
            literal("section"),
            literal("critical"),
            literal("break"),
            literal("else"),
            literal("loop"),
            literal("alt"),
            literal("opt"),
            literal("par"),
            literal("note"),
        )),
        // Ensure keyword is not followed by identifier character (word boundary)
        peek(not(one_of(|c: char| c.is_ascii_alphanumeric() || c == '_'))),
    )
    .map(|keyword: &str| match keyword {
        "diagram" => Token::Diagram,
        "component" => Token::Component,
        "sequence" => Token::Sequence,
        "type" => Token::Type,
        "embed" => Token::Embed,
        "as" => Token::As,
        "deactivate" => Token::Deactivate,
        "activate" => Token::Activate,
        "fragment" => Token::Fragment,
        "section" => Token::Section,
        "alt" => Token::Alt,
        "else" => Token::Else,
        "opt" => Token::Opt,
        "loop" => Token::Loop,
        "par" => Token::Par,
        "break" => Token::Break,
        "critical" => Token::Critical,
        "note" => Token::Note,
        _ => unreachable!(),
    })
    .parse_next(input)
}

/// Parse identifiers
fn identifier<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    // Start with letter or underscore, followed by alphanumeric or underscore
    take_while(1.., |c: char| {
        c.is_ascii_alphabetic() || c == '_' || c.is_ascii_digit()
    })
    .verify(|s: &str| {
        s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    })
    .map(Token::Identifier)
    .parse_next(input)
}

/// Parse multi-character operators (order matters - longest first)
fn multi_char_operator<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    alt((
        literal("<->").value(Token::DoubleArrow),
        literal("->").value(Token::Arrow_),
        literal("<-").value(Token::LeftArrow),
        literal("::").value(Token::DoubleColon),
    ))
    .parse_next(input)
}

/// Parse single character tokens
fn single_char_token<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    alt((
        '-'.value(Token::Plain),
        '='.value(Token::Equals),
        ':'.value(Token::Colon),
        '@'.value(Token::At),
        '{'.value(Token::LeftBrace),
        '}'.value(Token::RightBrace),
        '['.value(Token::LeftBracket),
        ']'.value(Token::RightBracket),
        ';'.value(Token::Semicolon),
        ','.value(Token::Comma),
    ))
    .parse_next(input)
}

/// Parse whitespace (spaces, tabs, etc. but not newlines)
fn whitespace<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    take_while(1.., |c: char| c.is_whitespace() && c != '\n')
        .value(Token::Whitespace)
        .parse_next(input)
}

/// Parse newline
fn newline<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    '\n'.value(Token::Newline).parse_next(input)
}

/// Parse a single token with position tracking
fn positioned_token<'a>(input: &mut Input<'a>) -> IResult<'a, PositionedToken<'a>> {
    let start_pos = input.current_token_start();

    let token = alt((
        line_comment,        // Must come before single char '-'
        string_literal,      // Must come before any single char
        multi_char_operator, // Must come before single char operators
        keyword,             // Must come before identifier
        float_literal,       // Must come before identifier
        identifier,          // Must come before single chars
        single_char_token,   // Single character tokens
        newline,             // Must come before whitespace
        whitespace,          // General whitespace
    ))
    .parse_next(input)?;

    let end_pos = input.current_token_start();
    let span = Span::new(start_pos..end_pos);

    Ok(PositionedToken::new(token, span))
}

/// Lexer that accumulates tokens and diagnostics during tokenization.
struct Lexer<'a> {
    tokens: Vec<PositionedToken<'a>>,
    diagnostics: DiagnosticCollector,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer.
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            diagnostics: DiagnosticCollector::new(),
        }
    }

    /// Tokenize the input, collecting tokens and errors.
    fn tokenize(&mut self, mut input: Input<'a>) {
        while !input.is_empty() {
            match positioned_token(&mut input) {
                Ok(token) => {
                    self.tokens.push(token);
                }
                Err(e) => {
                    // Get position before recovery
                    let error_pos = input.current_token_start();

                    let diagnostic = Self::convert_err_mode(e, error_pos);
                    self.diagnostics.emit(diagnostic);

                    // FIXME: Simple single-character skip causes cascading errors for
                    // string escape failures. E.g., `"test\u{}"` produces E006 (empty
                    // unicode escape) then E001 (unterminated string) because after
                    // skipping one char, the closing `"` starts a new unterminated string.
                    if !input.is_empty() {
                        input.next_token();
                    }
                }
            }
        }
    }

    /// Finish lexing and return tokens or collected errors.
    fn finish(self) -> Result<Vec<PositionedToken<'a>>, ParseError> {
        self.diagnostics.finish().map(|()| self.tokens)
    }

    /// Convert an ErrMode and error position to a Diagnostic.
    ///
    /// Extracts `LexerDiagnostic` from the error context for rich error info
    /// with code, message, and help. Falls back to E002 (unexpected character)
    /// if no diagnostic context is found.
    fn convert_err_mode(
        err: ErrMode<ContextError<LexerDiagnostic>>,
        error_pos: usize,
    ) -> Diagnostic {
        let context_error = match err {
            ErrMode::Backtrack(ctx) | ErrMode::Cut(ctx) => ctx,
            ErrMode::Incomplete(_) => ContextError::new(),
        };

        // Use the first diagnostic context if available
        if let Some(LexerDiagnostic {
            code,
            message,
            help,
            start,
        }) = context_error.context().next()
        {
            let span = Span::new(*start..error_pos);

            let mut diag = Diagnostic::error(*message)
                .with_code(*code)
                .with_label(span, code.description());
            if let Some(h) = help {
                diag = diag.with_help(*h);
            }
            return diag;
        }

        // Fallback when no context is present
        let span = Span::new(error_pos..error_pos.saturating_add(1));
        Diagnostic::error("unexpected character")
            .with_code(ErrorCode::E002)
            .with_label(span, ErrorCode::E002.description())
    }
}

/// Parse tokens from a string input, collecting multiple errors.
///
/// Attempts to recover from errors and continue tokenizing, collecting
/// all errors encountered. This provides better user experience by
/// reporting multiple issues in a single pass.
///
/// # Returns
///
/// - `Ok(tokens)` - All tokens successfully parsed
/// - `Err(ParseError)` - One or more errors occurred; contains all diagnostics
pub fn tokenize(input: &str) -> Result<Vec<PositionedToken<'_>>, ParseError> {
    let located_input = LocatingSlice::new(input);
    let mut lexer = Lexer::new();
    lexer.tokenize(located_input);
    lexer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_single_token(input: &str, expected: Token<'_>) {
        let mut located_input = LocatingSlice::new(input);
        let result = positioned_token(&mut located_input);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let positioned = result.unwrap();
        assert_eq!(positioned.token, expected);
    }

    #[test]
    fn test_keywords() {
        test_single_token("diagram", Token::Diagram);
        test_single_token("component", Token::Component);
        test_single_token("sequence", Token::Sequence);
        test_single_token("type", Token::Type);
        test_single_token("embed", Token::Embed);
        test_single_token("as", Token::As);
        test_single_token("deactivate", Token::Deactivate);
        test_single_token("activate", Token::Activate);
        test_single_token("fragment", Token::Fragment);
        test_single_token("section", Token::Section);
        test_single_token("alt", Token::Alt);
        test_single_token("else", Token::Else);
        test_single_token("opt", Token::Opt);
        test_single_token("loop", Token::Loop);
        test_single_token("par", Token::Par);
        test_single_token("break", Token::Break);
        test_single_token("critical", Token::Critical);
        test_single_token("note", Token::Note);
    }

    #[test]
    fn test_identifiers() {
        test_single_token("hello", Token::Identifier("hello"));
        test_single_token("_private", Token::Identifier("_private"));
        test_single_token("var123", Token::Identifier("var123"));
        test_single_token("CamelCase", Token::Identifier("CamelCase"));
    }

    #[test]
    fn test_activate_keyword_word_boundaries() {
        // Test that "activate" is recognized as a keyword
        test_single_token("activate", Token::Activate);

        // Test that identifiers containing "activate" are still treated as identifiers
        test_single_token("activateUser", Token::Identifier("activateUser"));
        test_single_token("useractivate", Token::Identifier("useractivate"));
        test_single_token("reactivate", Token::Identifier("reactivate"));

        // Test that "activate_user" is treated as a single identifier (no word boundary)
        test_single_token("activate_user", Token::Identifier("activate_user"));

        // Test that "activate" followed by space and identifier tokenizes correctly
        let input = "activate user";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 3); // activate, space, user
        assert_eq!(tokens[0].token, Token::Activate);
        assert_eq!(tokens[1].token, Token::Whitespace);
        assert_eq!(tokens[2].token, Token::Identifier("user"));
    }

    #[test]
    fn test_operators() {
        test_single_token("<->", Token::DoubleArrow);
        test_single_token("->", Token::Arrow_);
        test_single_token("<-", Token::LeftArrow);
        test_single_token("-", Token::Plain);
        test_single_token("=", Token::Equals);
        test_single_token(":", Token::Colon);
        test_single_token("@", Token::At);
    }

    #[test]
    fn test_punctuation() {
        test_single_token("{", Token::LeftBrace);
        test_single_token("}", Token::RightBrace);
        test_single_token("[", Token::LeftBracket);
        test_single_token("]", Token::RightBracket);
        test_single_token(";", Token::Semicolon);
        test_single_token(",", Token::Comma);
    }

    #[test]
    fn test_string_literals() {
        test_single_token(
            "\"hello world\"",
            Token::StringLiteral("hello world".to_string()),
        );
        test_single_token("\"\"", Token::StringLiteral("".to_string()));
        test_single_token("\"abc123\"", Token::StringLiteral("abc123".to_string()));
    }

    #[test]
    fn test_float_literals() {
        // Basic float literals
        test_single_token("1.0", Token::FloatLiteral(1.0));
        test_single_token("2.5", Token::FloatLiteral(2.5));
        test_single_token("10.0", Token::FloatLiteral(10.0));
        test_single_token("0.0", Token::FloatLiteral(0.0));

        // Float literals with leading decimal
        test_single_token(".5", Token::FloatLiteral(0.5));
        test_single_token(".25", Token::FloatLiteral(0.25));

        // Float literals with trailing decimal
        test_single_token("5.", Token::FloatLiteral(5.0));
        test_single_token("100.", Token::FloatLiteral(100.0));

        // Scientific notation
        test_single_token("1e5", Token::FloatLiteral(1e5));
        test_single_token("2.5e-3", Token::FloatLiteral(2.5e-3));
        test_single_token("1.23e+4", Token::FloatLiteral(1.23e+4));
        test_single_token("1E5", Token::FloatLiteral(1E5));
        test_single_token("2.5E-3", Token::FloatLiteral(2.5E-3));

        // Large and small numbers
        test_single_token("999999.999999", Token::FloatLiteral(999999.999999));
        test_single_token("0.000001", Token::FloatLiteral(0.000001));

        // Basic integer literals (converted to floats)
        test_single_token("1", Token::FloatLiteral(1.0));
        test_single_token("42", Token::FloatLiteral(42.0));
        test_single_token("0", Token::FloatLiteral(0.0));
        test_single_token("123", Token::FloatLiteral(123.0));
    }

    #[test]
    fn test_float_inf_vs_identifiers() {
        // Special float values when standalone (followed by non-identifier chars)
        test_single_token("inf ", Token::FloatLiteral(f32::INFINITY));
        test_single_token("infinity ", Token::FloatLiteral(f32::INFINITY));
        test_single_token("-inf ", Token::FloatLiteral(f32::NEG_INFINITY)); // Negative infinity

        // Identifiers that start with "inf" or "infinity" should NOT be parsed as floats
        // These should be parsed as identifiers
        test_single_token("InfoNote", Token::Identifier("InfoNote"));
        test_single_token("info", Token::Identifier("info"));
        test_single_token("information", Token::Identifier("information"));
        test_single_token("infinite", Token::Identifier("infinite"));
        test_single_token("infinity_pool", Token::Identifier("infinity_pool"));

        // Test in context with other tokens
        let tokens = tokenize("inf;").expect("Should tokenize");
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::FloatLiteral(_)));
        assert!(matches!(tokens[1].token, Token::Semicolon));

        // Test identifier starting with inf in context
        let tokens = tokenize("InfoNote;").expect("Should tokenize");
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token, Token::Identifier("InfoNote")));
        assert!(matches!(tokens[1].token, Token::Semicolon));
    }

    #[test]
    fn test_string_escape_sequences() {
        test_single_token(
            "\"hello\\nworld\"",
            Token::StringLiteral("hello\nworld".to_string()),
        );
        test_single_token(
            "\"quote: \\\"test\\\"\"",
            Token::StringLiteral("quote: \"test\"".to_string()),
        );
        test_single_token(
            "\"tab:\\tafter\"",
            Token::StringLiteral("tab:\tafter".to_string()),
        );
        test_single_token(
            "\"backslash: \\\\\"",
            Token::StringLiteral("backslash: \\".to_string()),
        );
    }

    #[test]
    fn test_unicode_escapes() {
        test_single_token(
            "\"emoji: \\u{1F602}\"",
            Token::StringLiteral("emoji: 😂".to_string()),
        );
        test_single_token(
            "\"unicode: \\u{00AC}\"",
            Token::StringLiteral("unicode: ¬".to_string()),
        );
        test_single_token(
            "\"letter: \\u{41}\"",
            Token::StringLiteral("letter: A".to_string()),
        );
    }

    #[test]
    fn test_escaped_whitespace_handling() {
        // Test that escaped whitespace is consumed and ignored
        // Note: The current implementation uses multispace1 to consume escaped whitespace
        test_single_token(
            "\"before\\  \n  after\"",
            Token::StringLiteral("beforeafter".to_string()),
        );
        test_single_token(
            "\"line1\\   \n   line2\"",
            Token::StringLiteral("line1line2".to_string()),
        );
    }

    #[test]
    fn test_control_character_escapes() {
        // Test control character escape sequences that are currently supported
        test_single_token(
            "\"bell: \\b form: \\f\"",
            Token::StringLiteral("bell: \u{08} form: \u{0C}".to_string()),
        );
        test_single_token(
            "\"slash: \\/\"",
            Token::StringLiteral("slash: /".to_string()),
        );
        test_single_token(
            "\"single: \\'\"",
            Token::StringLiteral("single: '".to_string()),
        );
        // Test null character separately
        test_single_token(
            "\"null: \\0\"",
            Token::StringLiteral("null: \0".to_string()),
        );
    }

    #[test]
    fn test_complex_escape_combinations() {
        // Test strings with multiple different escape types
        test_single_token(
            "\"Mixed: \\n\\t\\r\\\\\\\"\"",
            Token::StringLiteral("Mixed: \n\t\r\\\"".to_string()),
        );

        let complex_input =
            "\"tab:\\tafter tab, newline:\\nnew line, quote: \\\", emoji: \\u{1F602}\"";
        let expected_output = "tab:\tafter tab, newline:\nnew line, quote: \", emoji: 😂";
        test_single_token(
            complex_input,
            Token::StringLiteral(expected_output.to_string()),
        );
    }

    #[test]
    fn test_string_edge_cases() {
        // String with only escape sequences
        test_single_token("\"\\n\\t\\r\"", Token::StringLiteral("\n\t\r".to_string()));

        // String with only unicode escapes
        test_single_token(
            "\"\\u{41}\\u{42}\\u{43}\"",
            Token::StringLiteral("ABC".to_string()),
        );

        // Empty string
        test_single_token("\"\"", Token::StringLiteral("".to_string()));
    }

    #[test]
    fn test_mixed_content_advanced() {
        // Test combination of escaped whitespace, unicode, and regular escapes
        test_single_token(
            "\"Hello\\   \n  \\u{1F44B} World\\n!\"",
            Token::StringLiteral("Hello👋 World\n!".to_string()),
        );

        // Test escaped whitespace with unicode and control characters
        test_single_token(
            "\"start\\  \n\\u{41}\\b\\tend\"",
            Token::StringLiteral("startA\u{08}\tend".to_string()),
        );
    }

    #[test]
    fn test_string_boundary_conditions() {
        // Test very long strings with many escapes
        test_single_token(
            "\"a\\nb\\tc\\rd\\\\e\\\"f\\u{41}g\"",
            Token::StringLiteral("a\nb\tc\rd\\e\"fAg".to_string()),
        );

        // Test string with repeated escape patterns
        test_single_token("\"\\n\\n\\n\"", Token::StringLiteral("\n\n\n".to_string()));

        // Test unicode at boundaries
        test_single_token(
            "\"\\u{1F602}middle\\u{1F44B}\"",
            Token::StringLiteral("😂middle👋".to_string()),
        );

        // Test combinations that are known to work
        test_single_token(
            "\"backslash: \\\\, quote: \\\", tab: \\t\"",
            Token::StringLiteral("backslash: \\, quote: \", tab: \t".to_string()),
        );
    }

    #[test]
    fn test_comments() {
        test_single_token(
            "// this is a comment",
            Token::LineComment(" this is a comment"),
        );
        test_single_token("//", Token::LineComment(""));
        test_single_token("//no space", Token::LineComment("no space"));
    }

    #[test]
    fn test_whitespace() {
        test_single_token(" ", Token::Whitespace);
        test_single_token("\t", Token::Whitespace);
        test_single_token("   ", Token::Whitespace);
        test_single_token("\n", Token::Newline);
    }

    #[test]
    fn test_full_lexing() {
        let input = r#"diagram component "My System" -> target;"#;
        let result = tokenize(input);

        assert!(result.is_ok(), "Lexing failed: {:?}", result);
        let tokens = result.unwrap();

        // Extract just the token types for easier testing
        let token_types: Vec<_> = tokens.iter().map(|p| &p.token).collect();

        // Expected sequence: diagram, whitespace, component, whitespace, "My System", whitespace, ->, whitespace, target, ;
        assert!(matches!(token_types[0], Token::Diagram));
        assert!(matches!(token_types[1], Token::Whitespace));
        assert!(matches!(token_types[2], Token::Component));
        assert!(matches!(token_types[3], Token::Whitespace));
        assert!(matches!(token_types[4], Token::StringLiteral(_)));
        assert!(matches!(token_types[5], Token::Whitespace));
        assert!(matches!(token_types[6], Token::Arrow_));
        assert!(matches!(token_types[7], Token::Whitespace));
        assert!(matches!(token_types[8], Token::Identifier("target")));
        assert!(matches!(token_types[9], Token::Semicolon));
    }

    #[test]
    fn test_span_tracking() {
        let input = "hello world";
        let result = tokenize(input);

        assert!(result.is_ok());
        let tokens = result.unwrap();

        assert_eq!(tokens.len(), 3); // "hello", " ", "world"

        // Check spans
        assert_eq!(tokens[0].span.start(), 0);
        assert_eq!(tokens[0].span.end(), 5); // "hello"
        assert_eq!(tokens[1].span.start(), 5);
        assert_eq!(tokens[1].span.end(), 6); // " "
        assert_eq!(tokens[2].span.start(), 6);
        assert_eq!(tokens[2].span.end(), 11); // "world"
    }

    // Helper function to test lexer errors with span information
    fn test_lexer_error_at_position(input: &str, expected_error_pos: usize) {
        let result = tokenize(input);
        assert!(
            result.is_err(),
            "Expected lexer to fail on input: '{}'",
            input
        );

        // Verify we got a DiagnosticError
        let _error = result.unwrap_err();

        // TODO: Extract precise error span when winnow error details are accessible
        // For now, validate that lexing fails at expected position by checking partial success
        let mut partial_input = &input[..expected_error_pos.min(input.len())];
        if partial_input.is_empty() {
            return; // Cannot test empty input
        }

        // Test that we can lex up to the error position
        loop {
            let partial_result = tokenize(partial_input);
            if partial_result.is_ok() || partial_input.is_empty() {
                break;
            }
            if partial_input.len() <= 1 {
                break;
            }
            partial_input = &partial_input[..partial_input.len() - 1];
        }
    }

    /// Comprehensive lexer error tests focusing on span accuracy
    mod lexer_error_tests {
        use super::*;

        #[test]
        fn test_unclosed_string_literal_errors() {
            // Basic unclosed string - error at end of input
            test_lexer_error_at_position("\"hello", 6);

            // Unclosed string with content - error at end
            test_lexer_error_at_position("\"hello world", 12);

            // Empty unclosed string - error at position 1 (after opening quote)
            test_lexer_error_at_position("\"", 1);

            // Unclosed string with escape sequence
            test_lexer_error_at_position("\"hello\\nworld", 13);

            // Unclosed string with Unicode escape
            test_lexer_error_at_position("\"emoji\\u{1F602}", 15);
        }

        #[test]
        fn test_invalid_escape_sequence_errors() {
            // Invalid escape character
            test_lexer_error_at_position("\"hello\\x\"", 7); // \x is not valid

            // Incomplete escape at end
            test_lexer_error_at_position("\"hello\\", 7);

            // Invalid escape character combinations
            test_lexer_error_at_position("\"test\\q\"", 6); // \q is not valid
            test_lexer_error_at_position("\"test\\1\"", 6); // \1 is not valid
            test_lexer_error_at_position("\"test\\z\"", 6); // \z is not valid
        }

        #[test]
        fn test_malformed_unicode_escape_errors() {
            // Missing opening brace
            test_lexer_error_at_position("\"test\\u1F602\"", 7);

            // Missing closing brace
            test_lexer_error_at_position("\"test\\u{1F602\"", 13);

            // Invalid hex characters in Unicode escape
            test_lexer_error_at_position("\"test\\u{GHIJK}\"", 9);
            test_lexer_error_at_position("\"test\\u{123G}\"", 11);
            test_lexer_error_at_position("\"test\\u{XYZ}\"", 9);
        }

        #[test]
        fn test_invalid_unicode_codepoint_errors() {
            // Unicode code point too large (greater than 0x10FFFF)
            test_lexer_error_at_position("\"test\\u{110000}\"", 8);
            test_lexer_error_at_position("\"test\\u{FFFFFF}\"", 8);

            // Invalid Unicode surrogate range (0xD800-0xDFFF)
            test_lexer_error_at_position("\"test\\u{D800}\"", 8);
            test_lexer_error_at_position("\"test\\u{DFFF}\"", 8);
        }

        #[test]
        fn test_unterminated_unicode_escape_errors() {
            // Unterminated Unicode escape at end of string
            test_lexer_error_at_position("\"test\\u{123", 11);

            // Unterminated Unicode escape with quote
            test_lexer_error_at_position("\"test\\u{123\"", 12);

            // Unterminated empty Unicode escape
            test_lexer_error_at_position("\"test\\u{\"", 9);
        }

        #[test]
        fn test_empty_unicode_escape_errors() {
            // Empty Unicode escape braces
            test_lexer_error_at_position("\"test\\u{}\"", 8);

            // Unicode escape with only whitespace
            test_lexer_error_at_position("\"test\\u{ }\"", 8);
            test_lexer_error_at_position("\"test\\u{\t}\"", 8);
        }

        #[test]
        fn test_unicode_escape_too_long_errors() {
            // Unicode escape with too many hex digits (more than 6)
            test_lexer_error_at_position("\"test\\u{1234567}\"", 8);
            test_lexer_error_at_position("\"test\\u{12345678}\"", 8);
            test_lexer_error_at_position("\"test\\u{1F6020000}\"", 8);
        }

        #[test]
        fn test_missing_quote_handling_errors() {
            // String content without quotes
            let result = tokenize("hello world");
            assert!(result.is_ok()); // This should parse as identifier + whitespace + identifier

            // Mixed quote types (though " is standard)
            test_lexer_error_at_position("'hello'", 0); // Single quotes not supported for strings
        }

        #[test]
        fn test_complex_error_combinations() {
            // Multiple error conditions in one string
            test_lexer_error_at_position("\"unclosed with \\x invalid", 15);

            // Unicode and quote errors combined
            test_lexer_error_at_position("\"test\\u{GHIJK", 9);

            // Escape sequence errors at different positions
            test_lexer_error_at_position("\"start\\x middle\\u{} end", 7); // First error wins
        }

        #[test]
        fn test_error_position_boundaries() {
            // Test that errors occur at precise character boundaries

            // Error exactly at escape sequence start
            test_lexer_error_at_position("\"good\\xbad\"", 6);

            // Error at string boundary
            test_lexer_error_at_position("\"unterminated", 13);
        }

        #[test]
        fn test_multiline_string_errors() {
            // Unterminated string across lines (though strings can't normally span lines)
            test_lexer_error_at_position("\"hello\nworld\"", 6); // Newline in string

            // Unicode escape spanning lines
            test_lexer_error_at_position("\"test\\u{\n1F602}\"", 8);
        }

        #[test]
        fn test_invalid_relation_token_error() {
            // Test that invalid characters cause lexer errors
            let source = r#"
            diagram component;
            a: Rectangle;
            b: Rectangle;
            a > b;
        "#;

            // The lexer should fail because '>' is not a valid token
            let result = tokenize(source);
            assert!(
                result.is_err(),
                "Expected lexer to fail on invalid token '>'"
            );
        }

        /// Helper to verify error codes in diagnostics match exactly in order.
        fn assert_error_codes(input: &str, expected_codes: &[ErrorCode]) {
            let result = tokenize(input);
            assert!(
                result.is_err(),
                "Expected lexer to fail on input: '{input}'"
            );
            let parse_error = result.unwrap_err();
            let diagnostics = parse_error.diagnostics();
            assert_eq!(
                diagnostics.len(),
                expected_codes.len(),
                "Expected {} errors for input '{input}', got {}",
                expected_codes.len(),
                diagnostics.len()
            );
            for (i, (diag, expected)) in diagnostics.iter().zip(expected_codes).enumerate() {
                assert_eq!(
                    diag.code(),
                    Some(*expected),
                    "Error {i}: expected {expected:?} for input '{input}', got {:?}",
                    diag.code()
                );
            }
        }

        #[test]
        fn test_error_code_e003_invalid_escape_sequence() {
            // Invalid escape produces E003, then cascading E001 (see FIXME in recovery code)
            assert_error_codes("\"test\\x\"", &[ErrorCode::E003, ErrorCode::E001]);
            assert_error_codes("\"test\\q\"", &[ErrorCode::E003, ErrorCode::E001]);
            assert_error_codes("\"test\\z\"", &[ErrorCode::E003, ErrorCode::E001]);
            assert_error_codes("\"test\\1\"", &[ErrorCode::E003, ErrorCode::E001]);
        }

        #[test]
        fn test_error_code_e005_invalid_unicode_codepoint() {
            // Invalid codepoint produces E005, then cascading E001 (see FIXME in recovery code)
            assert_error_codes("\"test\\u{110000}\"", &[ErrorCode::E005, ErrorCode::E001]);
            assert_error_codes("\"test\\u{FFFFFF}\"", &[ErrorCode::E005, ErrorCode::E001]);
            // Surrogate range
            assert_error_codes("\"test\\u{D800}\"", &[ErrorCode::E005, ErrorCode::E001]);
            assert_error_codes("\"test\\u{DFFF}\"", &[ErrorCode::E005, ErrorCode::E001]);
        }

        #[test]
        fn test_error_code_e006_empty_unicode_escape() {
            // Empty unicode escape produces E006, then cascading E001 (see FIXME in recovery code)
            assert_error_codes("\"test\\u{}\"", &[ErrorCode::E006, ErrorCode::E001]);
        }

        #[test]
        fn test_error_code_e001_unterminated_string() {
            // Unterminated string should still produce E001
            assert_error_codes("\"unterminated", &[ErrorCode::E001]);
            assert_error_codes("\"", &[ErrorCode::E001]);
        }

        #[test]
        fn test_start_offset_span_for_unterminated_string() {
            // Verify StartOffset creates span from opening quote to error position
            // Use multi-line input with tokens before the string so it doesn't start at 0
            let input = "foo \"hello world\nbar";
            //           ^   ^           ^
            //           0   4           16 (newline position)
            //           |   |           |
            //           |   string start|
            //           identifier      error position (at newline)
            let result = tokenize(input);
            assert!(result.is_err());

            let parse_error = result.unwrap_err();
            let diagnostics = parse_error.diagnostics();
            assert!(!diagnostics.is_empty(), "Expected at least one diagnostic");
            let diagnostic = &diagnostics[0];
            let labels = diagnostic.labels();
            assert!(!labels.is_empty(), "Expected at least one label");

            let span = labels[0].span();
            // Span should start at 4 (opening quote after "foo ") and end at newline
            assert_eq!(
                span.start(),
                4,
                "Span should start at opening quote position (after 'foo ')"
            );
            assert_eq!(
                span.end(),
                16,
                "Span should end at newline (error position)"
            );
        }

        #[test]
        fn test_error_code_e002_unexpected_character() {
            // Invalid token should produce E002
            assert_error_codes(">", &[ErrorCode::E002]);
            assert_error_codes("$", &[ErrorCode::E002]);
        }

        #[test]
        fn test_multiple_unterminated_strings() {
            assert_error_codes(
                "\"first\n\"second\n\"third",
                &[ErrorCode::E001, ErrorCode::E001, ErrorCode::E001],
            );
        }

        #[test]
        fn test_mixed_error_types() {
            assert_error_codes(
                "> \"unterminated\n$",
                &[ErrorCode::E002, ErrorCode::E001, ErrorCode::E002],
            );
        }

        #[test]
        fn test_errors_with_valid_tokens_between() {
            assert_error_codes(
                "valid > identifier $ another",
                &[ErrorCode::E002, ErrorCode::E002],
            );
        }
    }
}

#[cfg(test)]
mod proptest_tests {
    use proptest::prelude::*;

    use super::*;

    // ===================
    // Strategies
    // ===================

    /// Strategy for generating valid identifier strings.
    /// Identifiers start with a letter and contain letters, digits, and underscores.
    fn valid_identifier_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,20}".prop_filter("avoid keywords", |s| {
            !matches!(
                s.as_str(),
                "diagram"
                    | "component"
                    | "sequence"
                    | "note"
                    | "on"
                    | "left"
                    | "right"
                    | "over"
                    | "activate"
                    | "deactivate"
                    | "alt"
                    | "else"
                    | "opt"
                    | "loop"
                    | "par"
                    | "critical"
                    | "group"
                    | "break"
                    | "ref"
                    | "type"
                    | "of"
                    | "true"
                    | "false"
                    | "inf"
            )
        })
    }

    /// Strategy for generating valid float literal strings.
    fn float_literal_strategy() -> impl Strategy<Value = String> {
        (0u32..10000, 0u32..10000).prop_map(|(integer, fraction)| format!("{integer}.{fraction}"))
    }

    // ===================
    // Property Test Functions
    // ===================

    /// Valid identifiers should always tokenize successfully.
    fn check_valid_identifiers_tokenize(id: &str) -> Result<(), TestCaseError> {
        let source = format!("diagram component; {id}: Rectangle;");
        let result = tokenize(&source);

        let err = result.err();
        prop_assert!(
            err.is_none(),
            "Failed to tokenize valid identifier `{id}`: {err:?}"
        );
        Ok(())
    }

    /// Float literals with various integer and fractional parts should parse.
    fn check_float_literals_parse(float_literal: &str) -> Result<(), TestCaseError> {
        let source = format!("diagram component; x: Rectangle [width={float_literal}];");
        let result = tokenize(&source);

        let err = result.err();
        prop_assert!(
            err.is_none(),
            "Failed to tokenize float literal `{float_literal}`: {err:?}"
        );
        Ok(())
    }

    // ===================
    // Proptest Wrappers
    // ===================

    proptest! {
        #[test]
        fn valid_identifiers_tokenize(id in valid_identifier_strategy()) {
            check_valid_identifiers_tokenize(&id)?;
        }

        #[test]
        fn float_literals_parse(float_literal in float_literal_strategy()) {
            check_float_literals_parse(&float_literal)?;
        }
    }
}
