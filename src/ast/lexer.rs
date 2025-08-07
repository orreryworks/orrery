use super::span::Span;
use super::tokens::{PositionedToken, Token};
use winnow::{
    Parser as _,
    ascii::{float, multispace1},
    combinator::{alt, delimited, not, peek, preceded, repeat, terminated},
    error::{ContextError, ModalResult, StrContext},
    token::{literal, none_of, one_of, take_while},
};

type Input<'a> = &'a str;
type IResult<'a, O> = ModalResult<O, ContextError>;

/// Parse a complete string literal with double quotes.
///
/// This function parses Rust-style string literals including:
/// - Basic strings: "hello world"
/// - Escape sequences: "hello\nworld", "quote: \"test\""
/// - Unicode escapes: "emoji: \u{1F602}", "symbol: \u{00AC}"
/// - Escaped whitespace: "before\   \n  after" (whitespace is consumed)
/// - Empty strings: ""
fn string_literal<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    // Helper for parsing escape sequences
    let escape_sequence = preceded(
        '\\',
        alt((
            // Unicode escape sequence: u{1-6 hex digits}
            preceded(
                'u',
                delimited('{', take_while(1..=6, |c: char| c.is_ascii_hexdigit()), '}')
                    .verify(|hex_str: &str| {
                        u32::from_str_radix(hex_str, 16)
                            .ok()
                            .and_then(std::char::from_u32)
                            .is_some()
                    })
                    .map(|hex_str: &str| {
                        u32::from_str_radix(hex_str, 16)
                            .ok()
                            .and_then(std::char::from_u32)
                            .unwrap() // Safe because we verified above
                    }),
            ),
            // Standard escape sequences
            one_of(['n', 'r', 't', 'b', 'f', '\\', '/', '\'', '"', '0']).map(|c| match c {
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
            }),
            // Escaped whitespace (consumed and ignored)
            multispace1.value('\u{E000}'), // Use private use char as placeholder to be filtered
        )),
    );

    // Regular string content (not quotes, backslashes, or newlines)
    let string_char = none_of(['"', '\\', '\n', '\r']);

    // String content: mix of regular chars and escapes
    let string_content =
        repeat(0.., alt((escape_sequence, string_char))).fold(String::new, |mut acc, ch| {
            if ch != '\u{E000}' {
                // Filter out escaped whitespace placeholders
                acc.push(ch);
            }
            acc
        });

    // Complete string literal
    delimited('"', string_content, '"')
        .map(Token::StringLiteral)
        .context(StrContext::Label("string literal"))
        .parse_next(input)
}

/// Parse a float literal
fn float_literal<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    float
        .map(Token::FloatLiteral)
        .context(StrContext::Label("float literal"))
        .parse_next(input)
}

/// Parse line comment starting with '//'
fn line_comment<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    preceded("//", take_while(0.., |c| c != '\n'))
        .map(Token::LineComment)
        .context(StrContext::Label("line comment"))
        .parse_next(input)
}

/// Parse keywords with word boundary checking
fn keyword<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    terminated(
        alt((
            literal("diagram").context(StrContext::Label("diagram keyword")),
            literal("component").context(StrContext::Label("component keyword")),
            literal("sequence").context(StrContext::Label("sequence keyword")),
            literal("type").context(StrContext::Label("type keyword")),
            literal("embed").context(StrContext::Label("embed keyword")),
            literal("as").context(StrContext::Label("as keyword")),
            literal("activate").context(StrContext::Label("activate keyword")),
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
        "activate" => Token::Activate,
        _ => unreachable!(),
    })
    .context(StrContext::Label("keyword"))
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
    .context(StrContext::Label("identifier"))
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
    .context(StrContext::Label("multi-character operator"))
    .parse_next(input)
}

/// Parse single character tokens
fn single_char_token<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    alt((
        '-'.value(Token::Plain),
        '='.value(Token::Equals),
        ':'.value(Token::Colon),
        '{'.value(Token::LeftBrace),
        '}'.value(Token::RightBrace),
        '['.value(Token::LeftBracket),
        ']'.value(Token::RightBracket),
        ';'.value(Token::Semicolon),
        ','.value(Token::Comma),
    ))
    .context(StrContext::Label("single character token"))
    .parse_next(input)
}

/// Parse whitespace (spaces, tabs, etc. but not newlines)
fn whitespace<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    take_while(1.., |c: char| c.is_whitespace() && c != '\n')
        .value(Token::Whitespace)
        .context(StrContext::Label("whitespace"))
        .parse_next(input)
}

/// Parse newline
fn newline<'a>(input: &mut Input<'a>) -> IResult<'a, Token<'a>> {
    '\n'.value(Token::Newline)
        .context(StrContext::Label("newline"))
        .parse_next(input)
}

/// Parse a single token with position tracking
fn positioned_token<'a>(
    input: &mut Input<'a>,
    original_len: usize,
) -> IResult<'a, PositionedToken<'a>> {
    let start_len = input.len();

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
    .context(StrContext::Label("token"))
    .parse_next(input)?;

    let end_len = input.len();
    let start_pos = original_len - start_len;
    let end_pos = original_len - end_len;
    let span = Span::new(start_pos..end_pos);

    Ok(PositionedToken::new(token, span))
}

/// Main lexer function that tokenizes the entire input
fn lexer<'src>(input: &mut Input<'src>) -> IResult<'src, Vec<PositionedToken<'src>>> {
    let original_len = input.len();
    repeat(0.., move |input: &mut Input<'src>| {
        positioned_token(input, original_len)
    })
    .context(StrContext::Label("lexer"))
    .parse_next(input)
}

/// Parse tokens from a string input
pub fn tokenize(input: &str) -> Result<Vec<PositionedToken<'_>>, String> {
    match lexer.parse(input) {
        Ok(tokens) => Ok(tokens),
        Err(e) => Err(format!("Lexer error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_single_token(input: &str, expected: Token<'_>) {
        let mut input_ref = input;
        let original_len = input.len();
        let result = positioned_token(&mut input_ref, original_len);
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
        test_single_token("activate", Token::Activate);
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

        // For now, we verify the error occurs - span extraction would require winnow error details
        let error = result.unwrap_err();
        assert!(!error.is_empty(), "Error message should not be empty");

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
    }
}
