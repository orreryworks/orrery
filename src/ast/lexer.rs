use super::span::SpanImpl;
use super::tokens::Token;
use chumsky::{
    IterParser as _, Parser,
    error::Rich,
    extra,
    primitive::{any, choice, just, none_of, one_of},
    text,
};
type Spanned<T> = (T, SpanImpl);

/// Parse a complete string literal with double quotes.
///
/// This function parses Rust-style string literals including:
/// - Basic strings: "hello world"
/// - Escape sequences: "hello\nworld", "quote: \"test\""
/// - Unicode escapes: "emoji: \u{1F602}", "symbol: \u{00AC}"
/// - Escaped whitespace: "before\   \n  after" (whitespace is consumed)
/// - Empty strings: ""
fn string_literal<'a>() -> impl Parser<'a, &'a str, Token<'a>, extra::Err<Rich<'a, char>>> {
    // All escape sequences start with backslash
    let escape = just('\\').ignore_then(choice((
        // Unicode escape sequence: u{1-6 hex digits}
        just('u')
            .ignore_then(
                any()
                    .filter(|c: &char| c.is_ascii_hexdigit())
                    .repeated()
                    .at_least(1)
                    .at_most(6)
                    .collect::<String>()
                    .delimited_by(just('{'), just('}'))
                    .try_map(|digits, span| {
                        u32::from_str_radix(&digits, 16)
                            .ok()
                            .and_then(std::char::from_u32)
                            .ok_or_else(|| Rich::custom(span, "Invalid Unicode escape sequence"))
                    }),
            )
            .map(Some)
            .labelled("unicode escape"),
        // Standard escape sequences
        one_of("nrtbf\\/'\"0")
            .map(|c| {
                Some(match c {
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
            })
            .labelled("escape sequence"),
        // Escaped whitespace (consumed and ignored)
        one_of(" \t\r\n")
            .repeated()
            .at_least(1)
            .to(None) // None means "don't add to output"
            .labelled("escaped whitespace"),
    )));

    // Regular string content (not quotes or backslashes)
    let string_char = none_of("\"\\");

    // String content: mix of regular chars and escapes
    let string_content = choice((escape, string_char.map(Some)))
        .repeated()
        .collect::<Vec<_>>()
        .map(|chars| chars.into_iter().flatten().collect::<String>());

    // Complete string literal
    string_content
        .delimited_by(just('"'), just('"'))
        .map(Token::StringLiteral)
        .labelled("string literal")
}

pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, SpanImpl>>> {
    // String literal parser - now with comprehensive escape sequence support
    let string_literal_parser = string_literal();

    // Line comment parser
    let line_comment = just("//")
        .then(any().and_is(just('\n').not()).repeated())
        .to_slice()
        .map(|s: &'src str| Token::LineComment(&s[2..]));

    // Keywords (must come before identifier)
    let keyword = choice((
        text::keyword("diagram").to(Token::Diagram),
        text::keyword("component").to(Token::Component),
        text::keyword("sequence").to(Token::Sequence),
        text::keyword("type").to(Token::Type),
        text::keyword("embed").to(Token::Embed),
        text::keyword("as").to(Token::As),
    ));

    // Identifier parser
    let identifier = text::ident().map(Token::Identifier);

    // Multi-character operators (order matters - longest first)
    let multi_char_op = choice((
        just("<->").to(Token::DoubleArrow),
        just("->").to(Token::Arrow_),
        just("<-").to(Token::LeftArrow),
    ));

    // Single character tokens
    let single_char = choice((
        just('-').to(Token::Plain),
        just('=').to(Token::Equals),
        just(':').to(Token::Colon),
        just('{').to(Token::LeftBrace),
        just('}').to(Token::RightBrace),
        just('[').to(Token::LeftBracket),
        just(']').to(Token::RightBracket),
        just(';').to(Token::Semicolon),
        just(',').to(Token::Comma),
    ));

    // Whitespace (spaces, tabs, etc. but not newlines)
    let whitespace = any()
        .filter(|c: &char| c.is_whitespace() && *c != '\n')
        .repeated()
        .at_least(1)
        .to(Token::Whitespace);

    // Newline
    let newline = just('\n').to(Token::Newline);

    // Combine all token types - order is important!
    let token = choice((
        line_comment,          // Must come before single char '-'
        string_literal_parser, // Must come before any single char
        multi_char_op,         // Must come before single char operators
        keyword,               // Must come before identifier
        identifier,            // Must come before single chars
        single_char,           // Single character tokens
        newline,               // Must come before whitespace
        whitespace,            // General whitespace
    ));

    // Parse tokens with their spans
    token
        .map_with(|tok, extra| (tok, extra.span()))
        .repeated()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;

    #[test]
    fn test_string_literal_parsing() {
        let input = r#"diagram "My System" component "UI Component""#;

        let lexer = lexer();
        let result = lexer.parse(input);

        assert!(result.has_output());
        let tokens = result.into_output().unwrap();

        // Find string literal tokens
        let string_tokens: Vec<&Token> = tokens
            .iter()
            .map(|(token, _span)| token)
            .filter(|token| matches!(token, Token::StringLiteral(_)))
            .collect();

        assert_eq!(string_tokens.len(), 2);

        // Verify first string literal
        if let Token::StringLiteral(s) = &string_tokens[0] {
            assert_eq!(s, "My System");
            // Verify it's a String that can be cloned
            let _owned: String = s.clone();
        } else {
            panic!("Expected StringLiteral token");
        }

        // Verify second string literal
        if let Token::StringLiteral(s) = &string_tokens[1] {
            assert_eq!(s, "UI Component");
            // Verify it's a String that can be cloned
            let _owned: String = s.clone();
        } else {
            panic!("Expected StringLiteral token");
        }
    }

    #[test]
    fn test_basic_strings() {
        let parser = string_literal();

        assert_eq!(
            parser.parse("\"hello world\"").unwrap(),
            Token::StringLiteral("hello world".to_string())
        );
        assert_eq!(
            parser.parse("\"\"").unwrap(),
            Token::StringLiteral("".to_string())
        );
        assert_eq!(
            parser.parse("\"abc123\"").unwrap(),
            Token::StringLiteral("abc123".to_string())
        );
    }

    #[test]
    fn test_escape_sequences() {
        let parser = string_literal();

        assert_eq!(
            parser.parse("\"hello\\nworld\"").unwrap(),
            Token::StringLiteral("hello\nworld".to_string())
        );
        assert_eq!(
            parser.parse("\"quote: \\\"test\\\"\"").unwrap(),
            Token::StringLiteral("quote: \"test\"".to_string())
        );
        assert_eq!(
            parser.parse("\"tab:\\tafter\"").unwrap(),
            Token::StringLiteral("tab:\tafter".to_string())
        );
        assert_eq!(
            parser.parse("\"backslash: \\\\\"").unwrap(),
            Token::StringLiteral("backslash: \\".to_string())
        );
        assert_eq!(
            parser.parse("\"carriage:\\rreturn\"").unwrap(),
            Token::StringLiteral("carriage:\rreturn".to_string())
        );
    }

    #[test]
    fn test_unicode_escapes() {
        let parser = string_literal();

        assert_eq!(
            parser.parse("\"emoji: \\u{1F602}\"").unwrap(),
            Token::StringLiteral("emoji: 😂".to_string())
        );
        assert_eq!(
            parser.parse("\"unicode: \\u{00AC}\"").unwrap(),
            Token::StringLiteral("unicode: ¬".to_string())
        );
        assert_eq!(
            parser.parse("\"letter: \\u{41}\"").unwrap(),
            Token::StringLiteral("letter: A".to_string())
        );
        assert_eq!(
            parser.parse("\"short: \\u{A}\"").unwrap(),
            Token::StringLiteral("short: \n".to_string())
        );
    }

    #[test]
    fn test_escaped_whitespace() {
        let parser = string_literal();

        assert_eq!(
            parser.parse("\"before\\   \n  after\"").unwrap(),
            Token::StringLiteral("beforeafter".to_string())
        );
        assert_eq!(
            parser.parse("\"line1\\    \n    \t  line2\"").unwrap(),
            Token::StringLiteral("line1line2".to_string())
        );
        assert_eq!(
            parser.parse("\"start\\ \t\r\n  end\"").unwrap(),
            Token::StringLiteral("startend".to_string())
        );
    }

    #[test]
    fn test_control_characters() {
        let parser = string_literal();

        assert_eq!(
            parser.parse("\"bell: \\b form: \\f\"").unwrap(),
            Token::StringLiteral("bell: \u{08} form: \u{0C}".to_string())
        );
        assert_eq!(
            parser.parse("\"null: \\0 slash: \\/\"").unwrap(),
            Token::StringLiteral("null: \0 slash: /".to_string())
        );
        assert_eq!(
            parser.parse("\"single: \\'\"").unwrap(),
            Token::StringLiteral("single: '".to_string())
        );
    }

    #[test]
    fn test_complex_strings() {
        let parser = string_literal();

        assert_eq!(
            parser.parse("\"Mixed: \\n\\t\\r\\\\\\\"\"").unwrap(),
            Token::StringLiteral("Mixed: \n\t\r\\\"".to_string())
        );

        let complex = "\"tab:\\tafter tab, newline:\\nnew line, quote: \\\", emoji: \\u{1F602}\"";
        let expected = Token::StringLiteral(
            "tab:\tafter tab, newline:\nnew line, quote: \", emoji: 😂".to_string(),
        );
        assert_eq!(parser.parse(complex).unwrap(), expected);
    }

    #[test]
    fn test_mixed_content() {
        let parser = string_literal();

        assert_eq!(
            parser
                .parse("\"Hello\\    \n  \\u{1F44B} World\\n!\"")
                .unwrap(),
            Token::StringLiteral("Hello👋 World\n!".to_string())
        );
    }

    #[test]
    fn test_error_cases() {
        let parser = string_literal();

        // Missing quotes
        assert!(parser.parse("hello").into_result().is_err());

        // Unclosed string
        assert!(parser.parse("\"unclosed").into_result().is_err());

        // Invalid escape sequence
        assert!(parser.parse("\"invalid\\x\"").into_result().is_err());

        // Invalid Unicode sequences
        assert!(parser.parse("\"invalid\\u{GHIJK}\"").into_result().is_err());
        assert!(
            parser
                .parse("\"invalid\\u{1234567}\"")
                .into_result()
                .is_err()
        ); // Too long
        assert!(parser.parse("\"invalid\\u{}\"").into_result().is_err()); // Empty
        assert!(
            parser
                .parse("\"unterminated\\u{123\"")
                .into_result()
                .is_err()
        );
        assert!(
            parser
                .parse("\"missing\\u{110000}\"")
                .into_result()
                .is_err()
        ); // Invalid code point
    }

    #[test]
    fn test_edge_cases() {
        let parser = string_literal();

        // Only escapes
        assert_eq!(
            parser.parse("\"\\n\\t\\r\"").unwrap(),
            Token::StringLiteral("\n\t\r".to_string())
        );

        // Only unicode
        assert_eq!(
            parser.parse("\"\\u{41}\\u{42}\\u{43}\"").unwrap(),
            Token::StringLiteral("ABC".to_string())
        );

        // Mixed escaped whitespace
        assert_eq!(
            parser.parse("\"a\\ \nb\\  \tc\"").unwrap(),
            Token::StringLiteral("abc".to_string())
        );
    }

    #[test]
    fn test_string_literals_in_lexer() {
        let input = r#"diagram "Hello\nWorld" component "Test \u{1F602}""#;

        let lexer = lexer();
        let result = lexer.parse(input);

        assert!(result.has_output());
        let tokens = result.into_output().unwrap();

        // Find string literal tokens
        let string_tokens: Vec<&String> = tokens
            .iter()
            .filter_map(|(token, _span)| {
                if let Token::StringLiteral(s) = token {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(string_tokens.len(), 2);
        assert_eq!(string_tokens[0], "Hello\nWorld");
        assert_eq!(string_tokens[1], "Test 😂");
    }
}
