//! String parsing module for Filament
//!
//! This module provides comprehensive Rust-style string literal parsing with support for:
//! - Unicode escape sequences (\u{XXXX})
//! - Standard escape sequences (\n, \t, \r, \\, \", \', \0, \b, \f, \/)
//! - Escaped whitespace (consumed/removed from output)
//! - Empty strings
//! - Proper error handling

use super::{PResult, Span, to_spanned};
use crate::error::SlimParserError;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{is_not, take_while_m_n},
    character::complete::{char, multispace1},
    combinator::{map, map_opt, map_res, value, verify},
    error::context,
    multi::fold,
    sequence::{delimited, preceded},
};
use nom_locate::LocatedSpan;

/// Parse a unicode sequence, of the form u{XXXX}, where XXXX is 1 to 6
/// hexadecimal numerals.
fn parse_unicode(input: Span) -> IResult<Span, char, SlimParserError> {
    let parse_hex = take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit());
    let parse_delimited_hex = preceded(char('u'), delimited(char('{'), parse_hex, char('}')));
    let parse_u32 = map_res(parse_delimited_hex, |hex: LocatedSpan<&str>| {
        u32::from_str_radix(hex.fragment(), 16)
    });
    map_opt(parse_u32, std::char::from_u32).parse(input)
}

/// Parse an escaped character: \n, \t, \r, \u{00AC}, etc.
fn parse_escaped_char(input: Span) -> IResult<Span, char, SlimParserError> {
    preceded(
        char('\\'),
        alt((
            parse_unicode,
            value('\n', char('n')),
            value('\r', char('r')),
            value('\t', char('t')),
            value('\u{08}', char('b')),
            value('\u{0C}', char('f')),
            value('\\', char('\\')),
            value('/', char('/')),
            value('"', char('"')),
            value('\'', char('\'')),
            value('\0', char('0')),
        )),
    )
    .parse(input)
}

/// Parse a backslash, followed by any amount of whitespace.
/// This whitespace will be consumed (removed from the output).
fn parse_escaped_whitespace(input: Span) -> IResult<Span, (), SlimParserError> {
    value((), preceded(char('\\'), multispace1)).parse(input)
}

/// Parse a non-empty block of text that doesn't include \ or "
fn parse_literal(input: Span) -> IResult<Span, &str, SlimParserError> {
    let not_quote_slash = is_not("\"\\");
    verify(not_quote_slash, |s: &LocatedSpan<&str>| {
        !s.fragment().is_empty()
    })
    .parse(input)
    .map(|(remaining, span)| (remaining, *span.fragment()))
}

/// A string fragment contains a fragment of a string being parsed:
/// either a non-empty Literal (a series of non-escaped characters),
/// a single parsed escaped character, or a block of escaped whitespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringFragment<'a> {
    Literal(&'a str),
    EscapedChar(char),
    EscapedWS,
}

/// Combine parse_literal, parse_escaped_whitespace, and parse_escaped_char
/// into a StringFragment.
fn parse_fragment(input: Span) -> IResult<Span, StringFragment, SlimParserError> {
    alt((
        map(parse_literal, StringFragment::Literal),
        map(parse_escaped_char, StringFragment::EscapedChar),
        value(StringFragment::EscapedWS, parse_escaped_whitespace),
    ))
    .parse(input)
}

/// Parse the content of a string (everything between the quotes).
/// Uses a fold to build up the final string from fragments.
fn parse_string_content(input: Span) -> IResult<Span, String, SlimParserError> {
    fold(0.., parse_fragment, String::new, |mut string, fragment| {
        match fragment {
            StringFragment::Literal(s) => string.push_str(s),
            StringFragment::EscapedChar(c) => string.push(c),
            StringFragment::EscapedWS => {} // Escaped whitespace is consumed/ignored
        }
        string
    })
    .parse(input)
}

/// Parse a complete string literal with double quotes.
///
/// This function parses Rust-style string literals including:
/// - Basic strings: "hello world"
/// - Escape sequences: "hello\nworld", "quote: \"test\""
/// - Unicode escapes: "emoji: \u{1F602}", "symbol: \u{00AC}"
/// - Escaped whitespace: "before\   \n  after" (whitespace is consumed)
/// - Empty strings: ""
///
/// # Examples
///
/// ```ignore
/// use nom_locate::LocatedSpan;
/// use filament::ast::parser::string::parse_string_literal;
///
/// let input = LocatedSpan::new("\"hello\\nworld\"");
/// let (_, result) = parse_string_literal(input).unwrap();
/// assert_eq!(*result, "hello\nworld");
///
/// let input = LocatedSpan::new("\"emoji: \\u{1F602}\"");
/// let (_, result) = parse_string_literal(input).unwrap();
/// assert_eq!(*result, "emoji: 😂");
/// ```
pub fn parse_string_literal(input: Span) -> PResult<String> {
    to_spanned(
        input,
        context(
            "string_literal",
            delimited(char('"'), parse_string_content, char('"')),
        )
        .parse(input),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_strings() {
        let input = LocatedSpan::new("\"hello world\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "hello world");

        let input = LocatedSpan::new("\"\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "");
    }

    #[test]
    fn test_escape_sequences() {
        let input = LocatedSpan::new("\"hello\\nworld\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "hello\nworld");

        let input = LocatedSpan::new("\"quote: \\\"test\\\"\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "quote: \"test\"");

        let input = LocatedSpan::new("\"tab:\\tafter\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "tab:\tafter");

        let input = LocatedSpan::new("\"backslash: \\\\\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "backslash: \\");
    }

    #[test]
    fn test_unicode_escapes() {
        let input = LocatedSpan::new("\"emoji: \\u{1F602}\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "emoji: 😂");

        let input = LocatedSpan::new("\"unicode: \\u{00AC}\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "unicode: ¬");

        let input = LocatedSpan::new("\"letter: \\u{41}\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "letter: A");
    }

    #[test]
    fn test_escaped_whitespace() {
        let input = LocatedSpan::new("\"before\\   \n  after\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "beforeafter");

        let input = LocatedSpan::new("\"line1\\    \n    \t  line2\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "line1line2");
    }

    #[test]
    fn test_control_characters() {
        let input = LocatedSpan::new("\"bell: \\b form: \\f\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "bell: \u{08} form: \u{0C}");

        let input = LocatedSpan::new("\"null: \\0 slash: \\/\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "null: \0 slash: /");
    }

    #[test]
    fn test_complex_strings() {
        let input = LocatedSpan::new("\"Mixed: \\n\\t\\r\\\\\\\"\"");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*result, "Mixed: \n\t\r\\\"");

        let input = LocatedSpan::new(
            "\"tab:\\tafter tab, newline:\\nnew line, quote: \\\", emoji: \\u{1F602}\"",
        );
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(
            *result,
            "tab:\tafter tab, newline:\nnew line, quote: \", emoji: 😂"
        );
    }

    #[test]
    fn test_error_cases() {
        // Missing quotes
        assert!(parse_string_literal(LocatedSpan::new("hello")).is_err());

        // Unclosed string
        assert!(parse_string_literal(LocatedSpan::new("\"unclosed")).is_err());

        // Invalid escape sequence
        assert!(parse_string_literal(LocatedSpan::new("\"invalid\\x\"")).is_err());

        // Invalid Unicode sequences
        assert!(parse_string_literal(LocatedSpan::new("\"invalid\\u{GHIJK}\"")).is_err());
        assert!(parse_string_literal(LocatedSpan::new("\"invalid\\u{1234567}\"")).is_err());
        assert!(parse_string_literal(LocatedSpan::new("\"invalid\\u{}\"")).is_err());
        assert!(parse_string_literal(LocatedSpan::new("\"unterminated\\u{123\"")).is_err());
    }

    #[test]
    fn test_with_content_after() {
        let input = LocatedSpan::new("\"hello\" world");
        let (rest, result) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), " world");
        assert_eq!(*result, "hello");
    }
}
