use crate::{
    ast::{
        parser_types as types,
        span::{Span, Spanned},
        tokens::{PositionedToken, Token},
    },
    error::ParseDiagnosticError,
};
use winnow::{
    Parser as _,
    combinator::{alt, delimited, not, opt, preceded, repeat, separated},
    error::{ContextError, ErrMode, StrContext},
    stream::{Offset, Stream, TokenSlice},
    token::any,
};

type Input<'src> = FilamentTokenSlice<'src>;
type IResult<'src, O> = Result<O, ErrMode<ContextError>>;
/// Type alias for winnow TokenSlice with our positioned tokens
type FilamentTokenSlice<'src> = TokenSlice<'src, PositionedToken<'src>>;

/// Helper function to create a spanned value
fn make_spanned<T>(value: T, span: Span) -> Spanned<T> {
    Spanned::new(value, span)
}

/// Parse whitespace and comments
fn ws_comment<'src>(input: &mut Input<'src>) -> IResult<'src, ()> {
    any.verify(|token: &PositionedToken<'_>| {
        matches!(
            token.token,
            Token::Whitespace | Token::Newline | Token::LineComment(_)
        )
    })
    .void()
    .parse_next(input)
}

/// Parse zero or more whitespace/comments
fn ws_comments0<'src>(input: &mut Input<'src>) -> IResult<'src, ()> {
    repeat(0.., ws_comment).parse_next(input)
}

/// Parse one or more whitespace/comments
fn ws_comments1<'src>(input: &mut Input<'src>) -> IResult<'src, ()> {
    repeat(1.., ws_comment).parse_next(input)
}

/// Parse semicolon with optional whitespace
fn semicolon<'src>(input: &mut Input<'src>) -> IResult<'src, ()> {
    preceded(
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Semicolon))
            .void(),
    )
    .context(StrContext::Label("semicolon"))
    .parse_next(input)
}

/// Parse a standard identifier with span preservation
fn identifier<'src>(input: &mut Input<'src>) -> IResult<'src, (&'src str, Span)> {
    any.verify_map(|token: &PositionedToken<'_>| match &token.token {
        Token::Identifier(name) => Some((*name, token.span)),
        // Allow keywords to be used as identifiers in appropriate contexts
        Token::Component => Some(("Component", token.span)),
        Token::Sequence => Some(("Sequence", token.span)),
        Token::Type => Some(("Type", token.span)),
        Token::Diagram => Some(("Diagram", token.span)),
        Token::Embed => Some(("Embed", token.span)),
        Token::As => Some(("As", token.span)),
        _ => None,
    })
    .context(StrContext::Label("identifier"))
    .parse_next(input)
}

/// Parse nested identifier with :: separators
fn nested_identifier<'src>(input: &mut Input<'src>) -> IResult<'src, (String, Span)> {
    let first = identifier.parse_next(input)?;
    let mut parts = vec![first];

    loop {
        // Try to parse `::` using DoubleColon token
        let checkpoint = input.checkpoint();
        let double_colon_result = any::<_, ErrMode<ContextError>>
            .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::DoubleColon))
            .parse_next(input);

        match double_colon_result {
            Ok(_) => {
                // Successfully parsed `::`, now parse the identifier
                let next_identifier = identifier.parse_next(input)?;
                parts.push(next_identifier);
            }
            Err(_) => {
                // Failed to parse `::`, reset and exit loop
                input.reset(&checkpoint);
                break;
            }
        }
    }

    let names: Vec<&str> = parts.iter().map(|(name, _span)| *name).collect();
    let joined_name = names.join("::");

    let unified_span = if let Some((_, first_span)) = parts.first() {
        parts
            .iter()
            .skip(1)
            .fold(*first_span, |acc, (_, span)| acc.union(*span))
    } else {
        unreachable!("This shouldn't happen since we have at least one part")
    };

    Ok((joined_name, unified_span))
}

/// Parse string literal
fn string_literal<'src>(input: &mut Input<'src>) -> IResult<'src, String> {
    any.verify_map(|token: &PositionedToken<'_>| match &token.token {
        Token::StringLiteral(s) => Some(s.clone()),
        _ => None,
    })
    .context(StrContext::Label("string literal"))
    .parse_next(input)
}

/// Parse a single attribute
fn attribute<'src>(input: &mut Input<'src>) -> IResult<'src, types::Attribute<'src>> {
    let (name, name_span) = identifier.parse_next(input)?;
    let name_spanned = make_spanned(name, name_span);

    preceded(
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Equals)),
    )
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let value = string_literal.parse_next(input)?;

    Ok(types::Attribute {
        name: name_spanned,
        value: make_spanned(value, Span::new(0..0)), // TODO: track proper span
    })
}

/// Parse comma-separated attributes
fn attributes<'src>(input: &mut Input<'src>) -> IResult<'src, Vec<types::Attribute<'src>>> {
    separated(
        0..,
        attribute,
        (
            ws_comments0,
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Comma)),
            ws_comments0,
        ),
    )
    .context(StrContext::Label("attributes"))
    .parse_next(input)
}

/// Parse attributes wrapped in brackets
fn wrapped_attributes<'src>(input: &mut Input<'src>) -> IResult<'src, Vec<types::Attribute<'src>>> {
    delimited(
        (
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBracket)),
            ws_comments0,
        ),
        opt(attributes).map(|attrs| attrs.unwrap_or_default()),
        (
            ws_comments0,
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBracket)),
        ),
    )
    .context(StrContext::Label("wrapped attributes"))
    .parse_next(input)
}

/// Parse a type definition
fn type_definition<'src>(input: &mut Input<'src>) -> IResult<'src, types::TypeDefinition<'src>> {
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Type))
        .parse_next(input)?;

    ws_comments1.parse_next(input)?;
    let (name, name_span) = identifier.parse_next(input)?;

    preceded(
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Equals)),
    )
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;
    let (base_type, base_type_span) = identifier.parse_next(input)?;

    ws_comments0.parse_next(input)?;
    let attributes = opt(wrapped_attributes)
        .map(|attrs| attrs.unwrap_or_default())
        .parse_next(input)?;

    semicolon.parse_next(input)?;

    Ok(types::TypeDefinition {
        name: make_spanned(name, name_span),
        base_type: make_spanned(base_type, base_type_span),
        attributes,
    })
}

/// Parse type definitions section
fn type_definitions<'src>(
    input: &mut Input<'src>,
) -> IResult<'src, Vec<types::TypeDefinition<'src>>> {
    repeat(0.., preceded(ws_comments0, type_definition)).parse_next(input)
}

/// Parse relation type specification inside brackets
fn relation_type_spec<'src>(
    input: &mut Input<'src>,
) -> IResult<'src, types::RelationTypeSpec<'src>> {
    alt((
        // [TypeName; attributes] - type name with semicolon followed by attributes
        (
            identifier,
            ws_comments0,
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Semicolon)),
            ws_comments0,
            attributes,
        )
            .map(
                |(type_name_pair, _, _, _, attributes)| types::RelationTypeSpec {
                    type_name: Some(make_spanned(type_name_pair.0, type_name_pair.1)),
                    attributes,
                },
            ),
        // [TypeName] (no attributes or semicolon) - identifier NOT followed by equals
        (
            identifier,
            ws_comments0,
            not(any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Equals))),
        )
            .map(|(type_name_pair, _, _)| types::RelationTypeSpec {
                type_name: Some(make_spanned(type_name_pair.0, type_name_pair.1)),
                attributes: Vec::new(),
            }),
        // [attributes] (no type name) - attributes only
        attributes.map(|attributes| types::RelationTypeSpec {
            type_name: None,
            attributes,
        }),
        // [] (empty type spec)
        ws_comments0.map(|_| types::RelationTypeSpec {
            type_name: None,
            attributes: Vec::new(),
        }),
    ))
    .parse_next(input)
}

/// Parse relation type (arrow with optional type specification)
fn relation_type<'src>(input: &mut Input<'src>) -> IResult<'src, &'src str> {
    let arrow = any
        .verify_map(|token: &PositionedToken<'_>| match &token.token {
            Token::Arrow_ => Some("->"),
            Token::LeftArrow => Some("<-"),
            Token::DoubleArrow => Some("<->"),
            Token::Plain => Some("-"),
            _ => None,
        })
        .parse_next(input)?;

    Ok(arrow)
}

/// Parse a component with optional nested elements
fn component_with_elements<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    let start_checkpoint = input.checkpoint();

    let (name, name_span) = identifier.parse_next(input)?;
    let name_spanned = make_spanned(name, name_span);

    ws_comments0.parse_next(input)?;

    // Optional "as" followed by a string literal
    let display_name = opt((
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::As)),
        ws_comments1,
        string_literal,
    ))
    .map(|opt| opt.map(|(_, _, s)| s))
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Colon))
        .parse_next(input)?;
    ws_comments0.parse_next(input)?;

    let (type_name, type_name_span) = identifier.parse_next(input)?;
    let type_name_spanned = make_spanned(type_name, type_name_span);

    ws_comments0.parse_next(input)?;
    let attributes = opt(wrapped_attributes)
        .map(|attrs| attrs.unwrap_or_default())
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Optional nested block: parse nested elements inside braces
    let nested_elements = opt(delimited(
        (
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace)),
            ws_comments0,
        ),
        elements,
        (
            ws_comments0,
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace)),
        ),
    ))
    .map(|nested| nested.unwrap_or_default())
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;
    semicolon.parse_next(input)?;

    let end_checkpoint = input.checkpoint();
    let span = Span::new(input.offset_from(&start_checkpoint)..input.offset_from(&end_checkpoint));

    Ok(types::Element::Component {
        name: name_spanned,
        display_name: display_name.map(|s| make_spanned(s, span)),
        type_name: type_name_spanned,
        attributes,
        nested_elements,
    })
}

/// Parse a complete relation statement
fn relation<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    let (from_component, from_span) = nested_identifier.parse_next(input)?;

    ws_comments0.parse_next(input)?;
    let relation_type = relation_type.parse_next(input)?;
    ws_comments0.parse_next(input)?;

    // Optional relation type specification in brackets
    let type_spec = opt(delimited(
        (
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBracket)),
            ws_comments0,
        ),
        relation_type_spec,
        (
            ws_comments0,
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBracket)),
        ),
    ))
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;
    let (to_component, to_span) = nested_identifier.parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Optional relation label as string literal
    let label = opt(preceded(
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Colon)),
        preceded(ws_comments0, string_literal),
    ))
    .parse_next(input)?;

    semicolon.parse_next(input)?;

    Ok(types::Element::Relation {
        source: make_spanned(from_component, from_span),
        target: make_spanned(to_component, to_span),
        relation_type: make_spanned(relation_type, Span::new(0..0)), // TODO: track proper span
        type_spec,
        label: label.map(|l| make_spanned(l, Span::new(0..0))), // TODO: track proper span
    })
}

/// Parse any element (component or relation)
fn elements<'src>(input: &mut Input<'src>) -> IResult<'src, Vec<types::Element<'src>>> {
    repeat(
        0..,
        preceded(ws_comments0, alt((component_with_elements, relation))),
    )
    .parse_next(input)
}

/// Parse diagram type (component, sequence, etc.)
fn diagram_type<'src>(input: &mut Input<'src>) -> IResult<'src, &'src str> {
    any.verify_map(|token: &PositionedToken<'_>| match &token.token {
        Token::Component => Some("component"),
        Token::Sequence => Some("sequence"),
        _ => None,
    })
    .context(StrContext::Label("diagram type"))
    .parse_next(input)
}

/// Parse diagram header with unwrapped attributes
fn diagram_header<'src>(
    input: &mut Input<'src>,
) -> IResult<'src, (&'src str, Vec<types::Attribute<'src>>)> {
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Diagram))
        .parse_next(input)?;
    ws_comments1.parse_next(input)?;
    let kind = diagram_type.parse_next(input)?;
    ws_comments0.parse_next(input)?;
    let attributes = opt(wrapped_attributes)
        .map(|attrs| attrs.unwrap_or_default())
        .parse_next(input)?;
    Ok((kind, attributes))
}

/// Parse diagram header with semicolon
pub fn diagram_header_with_semicolon<'src>(
    input: &mut Input<'src>,
) -> IResult<'src, (Spanned<&'src str>, Vec<types::Attribute<'src>>)> {
    let (kind, attributes) = diagram_header.parse_next(input)?;
    semicolon.parse_next(input)?;
    // For now, use a default span - in a real implementation we'd track this properly
    let kind_spanned = make_spanned(kind, Span::new(0..0));
    Ok((kind_spanned, attributes))
}

/// Parse complete diagram
fn diagram<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    ws_comments0.parse_next(input)?;
    let (kind, attributes) = diagram_header_with_semicolon.parse_next(input)?;
    let type_definitions = type_definitions.parse_next(input)?;
    let elements = elements.parse_next(input)?;
    ws_comments0.parse_next(input)?;

    // Check if we've consumed all tokens (equivalent to `end()` in chumsky)
    if !input.is_empty() {
        return Err(ErrMode::Cut(ContextError::new()));
    }

    Ok(types::Element::Diagram(types::Diagram {
        kind,
        attributes,
        type_definitions,
        elements,
    }))
}

/// Utility function to convert winnow errors to our custom error format
fn convert_error(error: ErrMode<ContextError>, source: &str) -> ParseDiagnosticError {
    match error {
        ErrMode::Backtrack(e) | ErrMode::Cut(e) => {
            // Extract context information for better error messages
            let contexts: Vec<String> = e
                .context()
                .filter_map(|ctx| match ctx {
                    StrContext::Label(label) => Some(format!("expected {label}")),
                    StrContext::Expected(exp) => Some(format!("expected {exp}")),
                    _ => None,
                })
                .collect();

            let message = if contexts.is_empty() {
                "unexpected token or end of input".to_string()
            } else {
                contexts.join(", ")
            };

            ParseDiagnosticError {
                src: source.to_string(),
                message: format!("Parse error: {message}"),
                span: None, // TODO: Extract span from context if available
                help: Some("Check syntax and token positioning".to_string()),
            }
        }
        ErrMode::Incomplete(_) => ParseDiagnosticError {
            src: source.to_string(),
            message: "Incomplete input - more tokens expected".to_string(),
            span: None,
            help: Some("Ensure input is complete".to_string()),
        },
    }
}

/// Build a diagram from tokens
pub fn build_diagram<'src>(
    tokens: &'src [PositionedToken<'src>],
    source: &str,
) -> Result<Spanned<types::Element<'src>>, ParseDiagnosticError> {
    let mut token_slice = TokenSlice::new(tokens);

    match diagram.parse_next(&mut token_slice) {
        Ok(diagram) => {
            let total_span = if tokens.is_empty() {
                0..0
            } else {
                let first = tokens[0].span;
                let last = tokens[tokens.len() - 1].span;
                first.start()..last.end()
            };
            Ok(make_spanned(diagram, Span::new(total_span)))
        }
        Err(e) => Err(convert_error(e, source)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::lexer::tokenize;

    fn parse_tokens(input: &str) -> Vec<PositionedToken<'_>> {
        tokenize(input).expect("Failed to tokenize input")
    }

    #[test]
    fn test_identifier() {
        let tokens = parse_tokens("test_id");
        let mut slice = TokenSlice::new(&tokens);
        let result = identifier.parse_next(&mut slice);
        assert!(result.is_ok());
        let (name, _span) = result.unwrap();
        assert_eq!(name, "test_id");
    }

    #[test]
    fn test_string_literal() {
        let tokens = parse_tokens("\"hello world\"");
        let mut slice = TokenSlice::new(&tokens);
        let result = string_literal.parse_next(&mut slice);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_nested_identifier() {
        let tokens = parse_tokens("parent::child::grandchild");
        let mut slice = TokenSlice::new(&tokens);
        let result = nested_identifier.parse_next(&mut slice);
        assert!(result.is_ok());
        let (name, _span) = result.unwrap();
        assert_eq!(name, "parent::child::grandchild");
    }

    #[test]
    fn test_simple_diagram() {
        let input = r#"diagram component;
        app: Rectangle;"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_relation() {
        let input = r#"diagram component;
        frontend -> backend;"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_diagram() {
        let input = r#"diagram component [layout="force"];
        type CustomBox = Rectangle [color="blue"];

        frontend: CustomBox [label="Frontend"];
        backend: Rectangle;

        frontend -> backend: "API calls";"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens, input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_keyword_tokenization_word_boundaries() {
        // Test that identifiers starting with keywords are tokenized correctly
        // This prevents regression of the bug where component_default was split into
        // [Component, Identifier("_default")] instead of [Identifier("component_default")]

        let test_cases = vec![
            ("component_default", "component_default"),
            ("type_system", "type_system"),
            ("diagram_flow", "diagram_flow"),
            ("sequence_number", "sequence_number"),
            ("embed_data", "embed_data"),
            ("as_string", "as_string"),
        ];

        for (input, expected) in test_cases {
            let tokens = parse_tokens(input);

            assert_eq!(
                tokens.len(),
                1,
                "Input '{}' should tokenize as single identifier, got {} tokens: {:?}",
                input,
                tokens.len(),
                tokens
            );

            match &tokens[0].token {
                crate::ast::tokens::Token::Identifier(name) => {
                    assert_eq!(
                        *name, expected,
                        "Expected identifier '{}', got '{}'",
                        expected, name
                    );
                }
                other => panic!("Expected Identifier token for '{}', got {:?}", input, other),
            }
        }
    }

    #[test]
    fn test_ws_comment_function() {
        // Test whitespace parsing
        let tokens = tokenize(" ").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_ok());

        let tokens = tokenize("\t").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_ok());

        // Test comment parsing
        let tokens = tokenize("// this is a comment").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_ok());

        // Test failure cases
        let tokens = tokenize("identifier").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_err());
    }

    #[test]
    fn test_semicolon_function() {
        // Test basic semicolon
        let tokens = tokenize(";").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(semicolon(&mut input).is_ok());

        // Test semicolon with leading whitespace
        let tokens = tokenize("  ;").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(semicolon(&mut input).is_ok());

        // Test failure cases
        let tokens = tokenize(":").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(semicolon(&mut input).is_err());
    }

    #[test]
    fn test_identifier_function() {
        // Test basic identifiers with span validation
        let tokens = tokenize("hello").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = identifier(&mut input);
        assert!(result.is_ok());
        let (name, span) = result.unwrap();
        assert_eq!(name, "hello");
        assert!(span.len() > 0);

        // Test keywords as identifiers
        let tokens = tokenize("Component").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = identifier(&mut input);
        assert!(result.is_ok());
        let (name, _) = result.unwrap();
        assert_eq!(name, "Component");

        // Test failure cases
        let tokens = tokenize("->").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(identifier(&mut input).is_err());
    }

    #[test]
    fn test_nested_identifier_function() {
        // Test simple identifier
        let tokens = tokenize("simple").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = nested_identifier(&mut input);
        assert!(result.is_ok());
        let (name, span) = result.unwrap();
        assert_eq!(name, "simple");
        assert!(span.len() > 0);

        // Test nested identifiers
        let tokens = tokenize("parent::child").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = nested_identifier(&mut input);
        assert!(result.is_ok());
        let (name, _) = result.unwrap();
        assert_eq!(name, "parent::child");
    }

    #[test]
    fn test_string_literal_function() {
        // Test basic string literals
        let tokens = tokenize("\"hello\"").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = string_literal(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");

        // Test strings with escape sequences
        let tokens = tokenize("\"hello\\nworld\"").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = string_literal(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello\nworld");

        // Test failure cases
        let tokens = tokenize("identifier").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(string_literal(&mut input).is_err());
    }

    #[test]
    fn test_attribute_function() {
        // Test basic attribute parsing
        let tokens = tokenize("color=\"red\"").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "color");
        assert_eq!(*attr.value.inner(), "red");

        // Test failure cases
        let tokens = tokenize("color=unquoted").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(attribute(&mut input).is_err());
    }

    #[test]
    fn test_wrapped_attributes_function() {
        // Test empty brackets
        let tokens = tokenize("[]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = wrapped_attributes(&mut input);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test single attribute in brackets
        let tokens = tokenize("[color=\"red\"]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = wrapped_attributes(&mut input);
        assert!(result.is_ok());
        let attrs = result.unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(*attrs[0].name.inner(), "color");
    }

    #[test]
    fn test_type_definition_function() {
        // Test basic type definition
        let tokens = tokenize("type MyType = Rectangle;").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = type_definition(&mut input);
        assert!(result.is_ok());
        let typedef = result.unwrap();
        assert_eq!(*typedef.name.inner(), "MyType");
        assert_eq!(*typedef.base_type.inner(), "Rectangle");

        // Test failure cases
        let tokens = tokenize("MyType = Rectangle;").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(type_definition(&mut input).is_err());
    }

    #[test]
    fn test_relation_type_function() {
        // Test all relation types
        let tokens = tokenize("->").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = relation_type(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "->");

        let tokens = tokenize("<-").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = relation_type(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "<-");

        // Test failure cases
        let tokens = tokenize("=").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(relation_type(&mut input).is_err());
    }

    #[test]
    fn test_relation_type_spec_functiontest_relation_type_spec_function() {
        // Test empty type spec
        let tokens = tokenize("").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = relation_type_spec(&mut input);
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert!(spec.type_name.is_none());
        assert!(spec.attributes.is_empty());

        // Test type name only
        let tokens = tokenize("RedArrow").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = relation_type_spec(&mut input);
        assert!(result.is_ok());
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.as_ref().unwrap().inner(), "RedArrow");
    }

    #[test]
    fn test_standalone_keywords_still_work() {
        // Ensure that standalone keywords are still recognized as keywords
        let test_cases = vec![
            ("component", crate::ast::tokens::Token::Component),
            ("type", crate::ast::tokens::Token::Type),
            ("diagram", crate::ast::tokens::Token::Diagram),
            ("sequence", crate::ast::tokens::Token::Sequence),
            ("embed", crate::ast::tokens::Token::Embed),
            ("as", crate::ast::tokens::Token::As),
        ];

        for (input, expected_token) in test_cases {
            let tokens = parse_tokens(input);

            assert_eq!(
                tokens.len(),
                1,
                "Keyword '{}' should be single token, got {}: {:?}",
                input,
                tokens.len(),
                tokens
            );

            assert_eq!(
                std::mem::discriminant(&tokens[0].token),
                std::mem::discriminant(&expected_token),
                "Expected keyword token for '{}', got {:?}",
                input,
                tokens[0].token
            );
        }
    }
}
