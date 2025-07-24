use std::fmt;

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
    Identifier(&'src str),

    // Operators
    Arrow_,      // ->
    LeftArrow,   // <-
    DoubleArrow, // <->
    Plain,       // -
    Equals,      // =
    Colon,       // :

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
            Token::Identifier(name) => write!(f, "{name}"),

            Token::Arrow_ => write!(f, "->"),
            Token::LeftArrow => write!(f, "<-"),
            Token::DoubleArrow => write!(f, "<->"),
            Token::Plain => write!(f, "-"),
            Token::Equals => write!(f, "="),
            Token::Colon => write!(f, ":"),

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
