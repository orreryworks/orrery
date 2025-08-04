use crate::ast::span::Span;
use std::fmt;
use winnow::stream::Location;

/// Token types for the Filament language
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'src> {
    // Keywords
    Diagram,
    Component,
    Sequence,
    Type,
    Embed,
    As,

    // Literals
    StringLiteral(String),
    FloatLiteral(f32),
    Identifier(&'src str),

    // Operators
    Arrow_,      // ->
    LeftArrow,   // <-
    DoubleArrow, // <->
    Plain,       // -
    Equals,      // =
    Colon,       // :
    DoubleColon, // ::

    // Punctuation
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]
    Semicolon,    // ;
    Comma,        // ,

    // Comments
    LineComment(&'src str), // // comment

    // Whitespace
    Whitespace,
    Newline,
}

/// A token with position information for winnow integration
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedToken<'src> {
    pub token: Token<'src>,
    pub span: Span,
}

impl<'src> PositionedToken<'src> {
    pub fn new(token: Token<'src>, span: Span) -> Self {
        Self { token, span }
    }
}

impl<'src> std::ops::Deref for PositionedToken<'src> {
    type Target = Token<'src>;

    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

impl<'src> AsRef<Token<'src>> for PositionedToken<'src> {
    fn as_ref(&self) -> &Token<'src> {
        &self.token
    }
}

impl<'src> From<(Token<'src>, Span)> for PositionedToken<'src> {
    fn from((token, span): (Token<'src>, Span)) -> Self {
        Self::new(token, span)
    }
}

impl<'src> fmt::Display for PositionedToken<'src> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.token.fmt(f)
    }
}

impl<'src> Location for PositionedToken<'src> {
    fn previous_token_end(&self) -> usize {
        self.span.start()
    }

    fn current_token_start(&self) -> usize {
        self.span.start()
    }
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Diagram => write!(f, "diagram"),
            Token::Component => write!(f, "component"),
            Token::Sequence => write!(f, "sequence"),
            Token::Type => write!(f, "type"),
            Token::Embed => write!(f, "embed"),
            Token::As => write!(f, "as"),

            Token::StringLiteral(s) => write!(f, "\"{s}\""),
            Token::FloatLiteral(n) => write!(f, "{n}"),
            Token::Identifier(name) => write!(f, "{name}"),

            Token::Arrow_ => write!(f, "->"),
            Token::LeftArrow => write!(f, "<-"),
            Token::DoubleArrow => write!(f, "<->"),
            Token::Plain => write!(f, "-"),
            Token::Equals => write!(f, "="),
            Token::Colon => write!(f, ":"),
            Token::DoubleColon => write!(f, "::"),

            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),

            Token::LineComment(comment) => write!(f, "//{comment}"),
            Token::Whitespace => write!(f, " "),
            Token::Newline => write!(f, "\\n"),
        }
    }
}
