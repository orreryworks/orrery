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
    stream::{Stream, TokenSlice},
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

fn cut_err<'src, O, F>(input: &mut Input<'src>, f: F) -> IResult<'src, O>
where
    F: FnOnce(&mut Input<'src>) -> IResult<'src, O>,
{
    match f(input) {
        Ok(o) => Ok(o),
        Err(ErrMode::Backtrack(e)) | Err(ErrMode::Cut(e)) => Err(ErrMode::Cut(e)),
        Err(e) => Err(e),
    }
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
fn string_literal<'src>(input: &mut Input<'src>) -> IResult<'src, Spanned<String>> {
    any.verify_map(|token: &PositionedToken<'_>| match &token.token {
        Token::StringLiteral(s) => Some(Spanned::new(s.clone(), token.span)),
        _ => None,
    })
    .context(StrContext::Label("string literal"))
    .parse_next(input)
}

/// Parse identifiers: `[id1, id2, id3]`
///
/// Fails if it encounters `=` after an identifier (which would indicate nested attributes).
///
/// Examples:
/// - `[component]` - single identifier
/// - `[client, server]` - multiple identifiers
/// - `[frontend::app]` - nested identifier with namespace separator
/// - `[client, server::api, database]` - mixed simple and nested identifiers
///
/// Note: Empty `[]` is handled by `empty_brackets` parser
fn identifiers<'src>(input: &mut Input<'src>) -> IResult<'src, Vec<Spanned<String>>> {
    delimited(
        // Opening bracket with optional whitespace
        (
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBracket)),
            ws_comments0,
        ),
        {
            // Closure to parse identifier list content with disambiguation logic
            // We need to check if the first identifier is followed by '=' to distinguish
            // from nested attributes [attr=val]
            move |input: &mut Input<'src>| {
                // Parse the first identifier (required - empty case handled elsewhere)
                let (id_str, id_span) = nested_identifier.parse_next(input)?;
                ws_comments0.parse_next(input)?;

                // Disambiguation: Check if next token is '='
                // If so, this is nested attributes [name=value], not identifiers
                let checkpoint = input.checkpoint();
                let result: Result<_, ErrMode<ContextError>> = any
                    .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Equals))
                    .parse_next(input);
                if result.is_ok() {
                    // Found '=' - this is nested attributes, backtrack
                    return Err(ErrMode::Backtrack(ContextError::new()));
                }
                input.reset(&checkpoint);

                // Build identifier list starting with first identifier
                let mut ids = vec![Spanned::new(id_str, id_span)];
                
                // Parse remaining identifiers separated by commas
                // Uses `repeat` combinator for clean separation of concerns
                let rest: Vec<Spanned<String>> = repeat(
                    0..,
                    preceded(
                        // Comma separator with optional whitespace
                        (
                            ws_comments0,
                            any.verify(|token: &PositionedToken<'_>| {
                                matches!(token.token, Token::Comma)
                            }),
                            ws_comments0,
                        ),
                        // Parse identifier and convert tuple to Spanned
                        nested_identifier.map(|(s, span)| Spanned::new(s, span)),
                    ),
                )
                .parse_next(input)?;
                ids.extend(rest);

                Ok(ids)
            }
        },
        // Closing bracket with optional whitespace
        (
            ws_comments0,
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBracket)),
        ),
    )
    .context(StrContext::Label("identifiers"))
    .parse_next(input)
}

/// Parse empty brackets: `[]`
///
/// Returns an Empty attribute value that can be interpreted as either
/// empty identifiers or empty nested attributes depending on context.
fn empty_brackets<'src>(input: &mut Input<'src>) -> IResult<'src, ()> {
    delimited(
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBracket)),
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBracket)),
    )
    .context(StrContext::Label("empty brackets"))
    .parse_next(input)
}

/// Parse an attribute value (string, float, identifier list, or nested attributes)
///
/// Attributes in Filament can have different value types depending on their purpose:
///
/// **Value Types:**
/// 1. **Empty** - `[]` - Ambiguous empty brackets (can be identifiers or attributes)
/// 2. **Identifiers** - `[id1, id2, ...]` - List of element identifiers (used in `on` attribute)
/// 3. **Attributes** - `[attr=val, ...]` - Nested attribute key-value pairs (stroke, text)
/// 4. **String** - `"value"` - Text values (colors, names, alignment)
/// 5. **Float** - `2.5` or `10` - Numeric values (widths, sizes, dimensions)
fn attribute_value<'src>(input: &mut Input<'src>) -> IResult<'src, types::AttributeValue<'src>> {
    alt((
        // Parse empty brackets [] first - can be interpreted as either empty identifiers or empty attributes
        empty_brackets.map(|_| types::AttributeValue::Empty),
        // Try identifiers: [id1, id2, ...]
        // This needs to be before wrapped_attributes since both start with '['
        identifiers.map(types::AttributeValue::Identifiers),
        // Parse nested attributes [attr1=val1, attr2=val2]
        wrapped_attributes.map(types::AttributeValue::Attributes),
        // Parse string or float literals
        any.verify_map(|token: &PositionedToken<'_>| match &token.token {
            Token::StringLiteral(s) => Some(types::AttributeValue::String(Spanned::new(
                s.clone(),
                token.span,
            ))),
            Token::FloatLiteral(f) => {
                Some(types::AttributeValue::Float(Spanned::new(*f, token.span)))
            }
            _ => None,
        }),
    ))
    .context(StrContext::Label("attribute value"))
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

    let value = attribute_value.parse_next(input)?;

    Ok(types::Attribute {
        name: name_spanned,
        value,
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

    Ok(types::Element::Component {
        name: name_spanned,
        display_name,
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
        label,
    })
}

/// Parse an activate block
///
/// Grammar:
///   activate <nested_identifier> { <elements> };
///
/// Notes:
/// - Accepts nested identifiers (e.g., `parent::child`) and returns `Spanned<String>`
///   for the component. The element span equals the identifier span; the `activate`
///   keyword and the trailing semicolon are not included in the element span
///   (consistent with `Element::span()` semantics using the inner `component` span).
/// - Whitespace and line comments are allowed between tokens as handled by
///   `ws_comments0/1`.
fn activate_block<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    // Parse "activate" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Activate))
        .context(StrContext::Label("activate keyword"))
        .parse_next(input)?;

    // Require at least one space or comment after the keyword
    ws_comments1
        .context(StrContext::Label("whitespace after activate"))
        .parse_next(input)?;

    let (component_name, component_span) = nested_identifier
        .context(StrContext::Label("component nested identifier"))
        .parse_next(input)?;
    let component_spanned = make_spanned(component_name, component_span);

    ws_comments0.parse_next(input)?;

    // Parse opening brace
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
        .context(StrContext::Label("opening brace '{'"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse nested elements
    let nested_elements = elements
        .context(StrContext::Label("activate block content"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse closing brace
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
        .context(StrContext::Label("closing brace '}'"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse semicolon
    semicolon
        .context(StrContext::Label("semicolon after activate block"))
        .parse_next(input)?;

    Ok(types::Element::ActivateBlock {
        component: component_spanned,
        elements: nested_elements,
    })
}

/// Parse a section block: `section "title" { elements };`
fn section_block<'src>(input: &mut Input<'src>) -> IResult<'src, types::FragmentSection<'src>> {
    // Parse "section" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Section))
        .context(StrContext::Label("section keyword"))
        .parse_next(input)?;

    cut_err(input, |input| {
        // Optional whitespace or comments after the keyword
        ws_comments0.parse_next(input)?;

        // Optional section title as a spanned string literal
        let title = opt(string_literal.context(StrContext::Label("section title string literal")))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse opening brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
            .context(StrContext::Label("opening brace '{'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse nested elements inside the section
        let elems = elements
            .context(StrContext::Label("section content"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse closing brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
            .context(StrContext::Label("closing brace '}'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse semicolon after the section block
        semicolon
            .context(StrContext::Label("semicolon after section"))
            .parse_next(input)?;

        Ok(types::FragmentSection {
            title,
            elements: elems,
        })
    })
}

/// Parse a section's content: "title"? { elements }
/// This is the common structure shared by all fragment sugar syntax blocks
fn parse_section_content<'src>(
    input: &mut Input<'src>,
    title_context: &'static str,
) -> IResult<'src, types::FragmentSection<'src>> {
    ws_comments0.parse_next(input)?;

    let title = opt(string_literal.context(StrContext::Label(title_context))).parse_next(input)?;

    ws_comments0.parse_next(input)?;

    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
        .context(StrContext::Label("opening brace '{'"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let elems = elements.parse_next(input)?;

    ws_comments0.parse_next(input)?;

    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
        .context(StrContext::Label("closing brace '}'"))
        .parse_next(input)?;

    Ok(types::FragmentSection {
        title,
        elements: elems,
    })
}

/// Macro for generating single-section fragment keyword parsers
macro_rules! single_section_parser {
    ($fn_name:ident, $token:ident, $title_ctx:expr, $element_variant:ident) => {
        fn $fn_name<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
            let keyword_token = any
                .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::$token))
                .context(StrContext::Label(concat!(stringify!($token), " keyword")))
                .parse_next(input)?;
            let keyword_span = keyword_token.span;

            cut_err(input, |input| {
                ws_comments0.parse_next(input)?;

                let attributes = opt(wrapped_attributes)
                    .map(|attrs| attrs.unwrap_or_default())
                    .parse_next(input)?;

                let section = parse_section_content(input, $title_ctx)?;

                ws_comments0.parse_next(input)?;
                semicolon
                    .context(StrContext::Label(concat!(
                        "semicolon after ",
                        stringify!($token),
                        " block"
                    )))
                    .parse_next(input)?;

                Ok(types::Element::$element_variant {
                    keyword_span,
                    section,
                    attributes,
                })
            })
        }
    };
}

/// Macro for generating multi-section fragment keyword parsers
macro_rules! multi_section_parser {
    ($fn_name:ident, $first_token:ident, $cont_token:ident, $first_ctx:expr, $cont_ctx:expr, $element_variant:ident) => {
        fn $fn_name<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
            let keyword_token = any
                .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::$first_token))
                .context(StrContext::Label(concat!(
                    stringify!($first_token),
                    " keyword"
                )))
                .parse_next(input)?;
            let keyword_span = keyword_token.span;

            cut_err(input, |input| {
                ws_comments0.parse_next(input)?;

                let attributes = opt(wrapped_attributes)
                    .map(|attrs| attrs.unwrap_or_default())
                    .parse_next(input)?;

                let first_section = parse_section_content(input, $first_ctx)?;
                let mut sections = vec![first_section];

                loop {
                    ws_comments0.parse_next(input)?;

                    let has_continuation = opt(any.verify(|token: &PositionedToken<'_>| {
                        matches!(token.token, Token::$cont_token)
                    }))
                    .parse_next(input)?;

                    if has_continuation.is_none() {
                        break;
                    }

                    sections.push(parse_section_content(input, $cont_ctx)?);
                }

                ws_comments0.parse_next(input)?;
                semicolon
                    .context(StrContext::Label(concat!(
                        "semicolon after ",
                        stringify!($first_token),
                        " block"
                    )))
                    .parse_next(input)?;

                Ok(types::Element::$element_variant {
                    keyword_span,
                    sections,
                    attributes,
                })
            })
        }
    };
}

// Generate single-section parsers
single_section_parser!(opt_block, Opt, "opt title", OptBlock);
single_section_parser!(loop_block, Loop, "loop title", LoopBlock);
single_section_parser!(break_block, Break, "break title", BreakBlock);
single_section_parser!(critical_block, Critical, "critical title", CriticalBlock);

// Generate multi-section parsers
multi_section_parser!(
    alt_else_block,
    Alt,
    Else,
    "alt title",
    "else title",
    AltElseBlock
);
multi_section_parser!(par_block, Par, Par, "par title", "par title", ParBlock);

/// Parse a fragment block: `fragment [attributes] "operation" { section+ };`
fn fragment_block<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    // Parse "fragment" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Fragment))
        .context(StrContext::Label("fragment keyword"))
        .parse_next(input)?;

    cut_err(input, |input| {
        ws_comments0.parse_next(input)?;

        // Parse optional attributes
        let attributes = opt(wrapped_attributes)
            .map(|attrs| attrs.unwrap_or_default())
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse the fragment operation (title) as a spanned string literal
        let operation = string_literal
            .context(StrContext::Label("fragment operation string literal"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse opening brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
            .context(StrContext::Label("opening brace '{'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse one or more sections
        let sections = repeat(1.., preceded(ws_comments0, section_block))
            .context(StrContext::Label("fragment sections"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse closing brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
            .context(StrContext::Label("closing brace '}'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse semicolon after the fragment block
        semicolon
            .context(StrContext::Label("semicolon after fragment"))
            .parse_next(input)?;

        Ok(types::Element::Fragment(types::Fragment {
            operation,
            sections,
            attributes,
        }))
    })
}

/// Shared helper to parse an activation-style statement after a specific keyword.
///
/// Grammar:
///   <keyword> <nested_identifier> ;
///
/// Where:
/// - <keyword> is one of: `activate`, `deactivate` (passed as a Token)
/// - <nested_identifier> supports `::`-qualified names and returns (String, Span)
/// - Optional whitespace and line comments are permitted between tokens
///
/// Span guarantees:
/// - The returned `Spanned<String>` covers only the nested identifier; the keyword
///   and the trailing semicolon are excluded.
/// - This aligns with `Element::span()` behavior that mirrors the identifier span.
fn parse_keyword_then_nested_identifier_then_semicolon<'src>(
    input: &mut Input<'src>,
    keyword: Token,
) -> IResult<'src, Spanned<String>> {
    any.verify(|token: &PositionedToken<'_>| token.token == keyword)
        .void()
        .context(StrContext::Label("keyword"))
        .parse_next(input)?;
    // Require at least one whitespace/comment after the keyword
    ws_comments1
        .context(StrContext::Label("whitespace after keyword"))
        .parse_next(input)?;
    let (name, name_span) = nested_identifier
        .context(StrContext::Label("component nested identifier"))
        .parse_next(input)?;
    ws_comments0.parse_next(input)?;
    semicolon
        .context(StrContext::Label("semicolon after activation statement"))
        .parse_next(input)?;
    Ok(Spanned::new(name, name_span))
}

/// Parse an explicit activate statement: `activate <nested_identifier>;`
///
/// Notes:
/// - Produces `Element::Activate { component: Spanned<String> }`
/// - The element span is the identifier span (see `Element::span()` in parser_types)
fn activate_statement<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    let component = parse_keyword_then_nested_identifier_then_semicolon(input, Token::Activate)?;
    Ok(types::Element::Activate { component })
}

/// Parse an explicit deactivate statement: `deactivate <nested_identifier>;`
///
/// Notes:
/// - Produces `Element::Deactivate { component: Spanned<String> }`
/// - The element span is the identifier span (see `Element::span()` in parser_types)
fn deactivate_statement<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    let component = parse_keyword_then_nested_identifier_then_semicolon(input, Token::Deactivate)?;
    Ok(types::Element::Deactivate { component })
}

/// Parse an activate element (explicit statement or block) with checkpoint routing
///
/// Behavior:
/// - If an `activate {` block is present, parse the block
/// - Otherwise, parse an explicit `activate <nested_identifier>;` statement
fn activate_element<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    // Try parsing an activate block first; if it fails, reset and parse explicit statement.
    let checkpoint = input.checkpoint();
    match activate_block.parse_next(input) {
        Ok(elem) => Ok(elem),
        Err(_) => {
            input.reset(&checkpoint);
            activate_statement.parse_next(input)
        }
    }
}

/// Parse a note element: `note [attributes]: "content";`
///
/// Syntax:
/// - `note` keyword
/// - Optional `[attributes]` block
/// - `:` separator
/// - String literal content
/// - `;` terminator
///
/// Examples:
/// - `note: "Simple note";`
/// - `note [on=[component]]: "Note attached to component";`
/// - `note [on=[a, b], align="left"]: "Note spanning multiple elements";`
fn note_element<'src>(input: &mut Input<'src>) -> IResult<'src, types::Element<'src>> {
    // Parse 'note' keyword
    let _ = any
        .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Note))
        .context(StrContext::Label("note keyword"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse optional attributes
    let attributes = opt(wrapped_attributes)
        .map(|attrs| attrs.unwrap_or_default())
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse colon separator
    let _ = any
        .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Colon))
        .context(StrContext::Label("colon"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse string literal content
    let content = string_literal.parse_next(input)?;

    // Parse semicolon
    semicolon.parse_next(input)?;

    Ok(types::Element::Note(types::Note {
        attributes,
        content,
    }))
}
/// Parse any element (component or relation)
fn elements<'src>(input: &mut Input<'src>) -> IResult<'src, Vec<types::Element<'src>>> {
    repeat(
        0..,
        preceded(
            ws_comments0,
            // Prioritize keyword-based items; explicit activate/deactivate first,
            // then blocks, then relations and components.
            alt((
                activate_element,
                deactivate_statement,
                note_element,
                alt_else_block,
                par_block,
                opt_block,
                loop_block,
                break_block,
                critical_block,
                fragment_block,
                relation,
                component_with_elements,
            )),
        ),
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

    // Test helpers
    fn parse_tokens(input: &str) -> Vec<PositionedToken<'_>> {
        tokenize(input).expect("Failed to tokenize input")
    }

    // Helpers for span assertions in tests
    fn first_identifier_span(tokens: &[PositionedToken<'_>]) -> Span {
        tokens
            .iter()
            .find_map(|t| match &t.token {
                Token::Identifier(_) => Some(t.span),
                _ => None,
            })
            .expect("identifier token not found")
    }

    fn nested_identifier_span(tokens: &[PositionedToken<'_>]) -> Span {
        let mut id_spans = tokens.iter().filter_map(|t| match &t.token {
            Token::Identifier(_) => Some(t.span),
            _ => None,
        });

        let first = id_spans
            .next()
            .expect("at least one identifier expected for nested identifier");
        let last = id_spans.next_back().unwrap_or(first);

        Span::new(first.start()..last.end())
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
        assert_eq!(result.unwrap().inner(), "hello world");
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

    // Activation statement tests

    #[test]
    fn test_activate_statement_parsing_basic() {
        let input = "activate user;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = activate_statement.parse_next(&mut slice);
        assert!(elem.is_ok(), "activate statement should parse");

        match elem.unwrap() {
            types::Element::Activate { component } => {
                assert_eq!(component.inner(), "user");
                let id_span = first_identifier_span(&tokens);
                assert_eq!(
                    component.span(),
                    id_span,
                    "component span should match identifier span"
                );
            }
            other => panic!("expected Activate element, got {:?}", other),
        }
    }

    #[test]
    fn test_deactivate_statement_parsing_basic() {
        let input = "deactivate server;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = deactivate_statement.parse_next(&mut slice);
        assert!(elem.is_ok(), "deactivate statement should parse");

        match elem.unwrap() {
            types::Element::Deactivate { component } => {
                assert_eq!(component.inner(), "server");
                let id_span = first_identifier_span(&tokens);
                assert_eq!(
                    component.span(),
                    id_span,
                    "component span should match identifier span"
                );
            }
            other => panic!("expected Deactivate element, got {:?}", other),
        }
    }

    #[test]
    fn test_activate_statement_whitespace_and_comments() {
        let input = "activate // comment\n  user   ;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = activate_statement.parse_next(&mut slice);
        assert!(
            elem.is_ok(),
            "activate with comments/whitespace should parse"
        );
        match elem.unwrap() {
            types::Element::Activate { component } => {
                assert_eq!(component.inner(), "user");
            }
            other => panic!("expected Activate element, got {:?}", other),
        }
    }

    #[test]
    fn test_deactivate_statement_whitespace_and_comments() {
        let input = "deactivate  \n  user // trailing\n ;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = deactivate_statement.parse_next(&mut slice);
        assert!(
            elem.is_ok(),
            "deactivate with comments/whitespace should parse"
        );
        match elem.unwrap() {
            types::Element::Deactivate { component } => {
                assert_eq!(component.inner(), "user");
            }
            other => panic!("expected Deactivate element, got {:?}", other),
        }
    }

    #[test]
    fn test_activate_statement_missing_semicolon_error() {
        let input = "activate user";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = activate_statement.parse_next(&mut slice);
        assert!(elem.is_err(), "missing semicolon should fail");
    }

    #[test]
    fn test_deactivate_statement_missing_identifier_error() {
        let input = "deactivate ;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = deactivate_statement.parse_next(&mut slice);
        assert!(elem.is_err(), "missing identifier should fail");
    }

    #[test]
    fn test_activate_statement_span_accuracy_nested_identifier() {
        let input = "activate parent::child;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = activate_statement.parse_next(&mut slice);
        assert!(elem.is_ok(), "activate with nested identifier should parse");
        match elem.unwrap() {
            types::Element::Activate { component } => {
                assert_eq!(component.inner(), "parent::child");
                let expected_span = nested_identifier_span(&tokens);
                assert_eq!(
                    component.span(),
                    expected_span,
                    "component span should cover from first to last identifier"
                );
            }
            other => panic!("expected Activate element, got {:?}", other),
        }
    }

    #[test]
    fn test_deactivate_statement_span_accuracy_nested_identifier() {
        let input = "deactivate a::b::c;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = deactivate_statement.parse_next(&mut slice);
        assert!(
            elem.is_ok(),
            "deactivate with nested identifier should parse"
        );
        match elem.unwrap() {
            types::Element::Deactivate { component } => {
                assert_eq!(component.inner(), "a::b::c");
                let expected_span = nested_identifier_span(&tokens);
                assert_eq!(
                    component.span(),
                    expected_span,
                    "component span should cover from first to last identifier"
                );
            }
            other => panic!("expected Deactivate element, got {:?}", other),
        }
    }

    // Activation statement tests

    #[test]
    fn test_activate_block_parsing() {
        let input = r#"activate user {
            user -> server: "Request";
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(
            result.is_ok(),
            "Failed to parse activate block: {:?}",
            result
        );

        let element = result.unwrap();
        if let types::Element::ActivateBlock {
            component,
            elements,
        } = element
        {
            assert_eq!(component.inner(), "user");
            assert_eq!(elements.len(), 1);
        } else {
            panic!("Expected ActivateBlock element, got {:?}", element);
        }
    }

    #[test]
    fn test_activate_block_missing_identifier() {
        let input = r#"activate {
            user -> server;
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(result.is_err(), "Should fail when identifier is missing");
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
        assert!(!span.is_empty());

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
        assert!(!span.is_empty());

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
        assert_eq!(result.unwrap().inner(), "hello");

        // Test strings with escape sequences
        let tokens = tokenize("\"hello\\nworld\"").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = string_literal(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().inner(), "hello\nworld");

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
        assert!(matches!(&attr.value, types::AttributeValue::String(s) if s.inner() == "red"));

        // Test failure cases
        let tokens = tokenize("color=unquoted").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        assert!(attribute(&mut input).is_err());

        // Test float attribute parsing
        let tokens = tokenize("width=2.5").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "width");
        assert!(matches!(&attr.value, types::AttributeValue::Float(f) if *f.inner() == 2.5));
    }

    #[test]
    fn test_nested_attribute_parsing() {
        // Test basic nested attribute parsing
        let tokens = tokenize("text=[font_size=12, padding=6.5]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");

        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 2);
            assert_eq!(*nested_attrs[0].name.inner(), "font_size");
            assert!(
                matches!(&nested_attrs[0].value, types::AttributeValue::Float(f) if *f.inner() == 12.0)
            );
            assert_eq!(*nested_attrs[1].name.inner(), "padding");
            assert!(
                matches!(&nested_attrs[1].value, types::AttributeValue::Float(f) if *f.inner() == 6.5)
            );
        } else {
            panic!("Expected nested attributes");
        }

        // Test empty nested attributes
        let tokens = tokenize("text=[]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");
        // Empty brackets [] are parsed as Empty variant
        if let types::AttributeValue::Empty = &attr.value {
            // Verify it can be interpreted as empty attributes
            assert_eq!(attr.value.as_attributes().unwrap().len(), 0);
        } else {
            panic!("Expected Empty attribute value for text=[]");
        }

        // Test single nested attribute
        let tokens = tokenize("text=[font_size=16]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 1);
            assert_eq!(*nested_attrs[0].name.inner(), "font_size");
            assert!(
                matches!(&nested_attrs[0].value, types::AttributeValue::Float(f) if *f.inner() == 16.0)
            );
        } else {
            panic!("Expected nested attributes");
        }

        // Test nested attributes with mixed types
        let tokens =
            tokenize("text=[font_family=\"Arial\", font_size=14]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 2);
            assert_eq!(*nested_attrs[0].name.inner(), "font_family");
            assert!(
                matches!(&nested_attrs[0].value, types::AttributeValue::String(s) if s.inner() == "Arial")
            );
            assert_eq!(*nested_attrs[1].name.inner(), "font_size");
            assert!(
                matches!(&nested_attrs[1].value, types::AttributeValue::Float(f) if *f.inner() == 14.0)
            );
        } else {
            panic!("Expected nested attributes");
        }
    }

    #[test]
    fn test_nested_attribute_whitespace() {
        // Test nested attributes with various whitespace
        let tokens =
            tokenize("text=[ font_size = 12 , padding = 6.5 ]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 2);
            assert_eq!(*nested_attrs[0].name.inner(), "font_size");
            assert!(
                matches!(&nested_attrs[0].value, types::AttributeValue::Float(f) if *f.inner() == 12.0)
            );
            assert_eq!(*nested_attrs[1].name.inner(), "padding");
            assert!(
                matches!(&nested_attrs[1].value, types::AttributeValue::Float(f) if *f.inner() == 6.5)
            );
        } else {
            panic!("Expected nested attributes");
        }
    }

    #[test]
    fn test_nested_attribute_error_handling() {
        // Test unclosed bracket
        let tokens = tokenize("text=[font_size=12").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test missing equals in nested attribute
        let tokens = tokenize("text=[font_size 12]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test missing value in nested attribute
        let tokens = tokenize("text=[font_size=]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test invalid comma usage in nested attributes
        let tokens = tokenize("text=[,font_size=12]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test trailing comma in nested attributes (should fail)
        let tokens = tokenize("text=[font_size=12,]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test nested brackets (should error - not supported)
        let tokens = tokenize("text=[style=[curved=true]]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_text_attribute_parsing() {
        // Test complete text attribute group
        let tokens = tokenize(
            "text=[font_size=16, font_family=\"Arial\", background_color=\"white\", padding=8.0]",
        )
        .expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");

        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 4);

            // Check font_size
            assert_eq!(*nested_attrs[0].name.inner(), "font_size");
            assert!(
                matches!(&nested_attrs[0].value, types::AttributeValue::Float(f) if *f.inner() == 16.0)
            );

            // Check font_family
            assert_eq!(*nested_attrs[1].name.inner(), "font_family");
            assert!(
                matches!(&nested_attrs[1].value, types::AttributeValue::String(s) if s.inner() == "Arial")
            );

            // Check background_color (simplified name)
            assert_eq!(*nested_attrs[2].name.inner(), "background_color");
            assert!(
                matches!(&nested_attrs[2].value, types::AttributeValue::String(s) if s.inner() == "white")
            );

            // Check padding (simplified name)
            assert_eq!(*nested_attrs[3].name.inner(), "padding");
            assert!(
                matches!(&nested_attrs[3].value, types::AttributeValue::Float(f) if *f.inner() == 8.0)
            );
        } else {
            panic!("Expected nested text attributes");
        }
    }

    #[test]
    fn test_text_attribute_minimal_cases() {
        // Test empty text attributes
        // Empty text attributes: text=[]
        let tokens = tokenize("text=[]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");
        // Empty brackets [] are parsed as Empty variant
        if let types::AttributeValue::Empty = &attr.value {
            // Verify it can be interpreted as empty attributes
            assert_eq!(attr.value.as_attributes().unwrap().len(), 0);
        } else {
            panic!("Expected Empty attribute value for text=[]");
        }

        // Test single text attribute
        let tokens = tokenize("text=[font_size=20]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 1);
            assert_eq!(*nested_attrs[0].name.inner(), "font_size");
            assert!(
                matches!(&nested_attrs[0].value, types::AttributeValue::Float(f) if *f.inner() == 20.0)
            );
        } else {
            panic!("Expected single text attribute");
        }

        // Test text attribute with whitespace variations
        let tokens = tokenize("text=[ font_size = 14 , font_family = \"Helvetica\" ]")
            .expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 2);
            assert_eq!(*nested_attrs[0].name.inner(), "font_size");
            assert_eq!(*nested_attrs[1].name.inner(), "font_family");
        } else {
            panic!("Expected text attributes with whitespace");
        }
    }

    #[test]
    fn test_text_attribute_type_combinations() {
        // Test all supported text attribute types
        let tokens = tokenize("text=[font_size=12, font_family=\"Courier\", background_color=\"#ff0000\", padding=5.5]").expect("Failed to tokenize");
        let mut input = FilamentTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();

        if let types::AttributeValue::Attributes(nested_attrs) = &attr.value {
            assert_eq!(nested_attrs.len(), 4);

            // Verify each attribute type
            assert!(matches!(
                &nested_attrs[0].value,
                types::AttributeValue::Float(_)
            )); // font_size
            assert!(matches!(
                &nested_attrs[1].value,
                types::AttributeValue::String(_)
            )); // font_family
            assert!(matches!(
                &nested_attrs[2].value,
                types::AttributeValue::String(_)
            )); // background_color
            assert!(matches!(
                &nested_attrs[3].value,
                types::AttributeValue::Float(_)
            )); // padding

            // Verify specific values
            if let types::AttributeValue::Float(f) = &nested_attrs[0].value {
                assert_eq!(*f.inner(), 12.0);
            }
            if let types::AttributeValue::String(s) = &nested_attrs[1].value {
                assert_eq!(s.inner(), "Courier");
            }
            if let types::AttributeValue::String(s) = &nested_attrs[2].value {
                assert_eq!(s.inner(), "#ff0000");
            }
            if let types::AttributeValue::Float(f) = &nested_attrs[3].value {
                assert_eq!(*f.inner(), 5.5);
            }
        } else {
            panic!("Expected text attributes with various types");
        }
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

    #[test]
    fn test_activate_block_missing_opening_brace() {
        let input = r#"activate user
            user -> server;
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(result.is_err(), "Should fail when opening brace is missing");
    }

    #[test]
    fn test_activate_block_missing_closing_brace() {
        let input = r#"activate user {
            user -> server;"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(result.is_err(), "Should fail when closing brace is missing");
    }

    #[test]
    fn test_activate_block_missing_semicolon() {
        let input = r#"activate user {
            user -> server;
        }"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(result.is_err(), "Should fail when semicolon is missing");
    }

    #[test]
    fn test_activate_block_empty_block() {
        let input = r#"activate user {
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(result.is_ok(), "Empty activate block should be valid");

        let element = result.unwrap();
        if let types::Element::ActivateBlock {
            component,
            elements,
        } = element
        {
            assert_eq!(component.inner(), &"user");
            assert_eq!(elements.len(), 0);
        } else {
            panic!("Expected ActivateBlock element");
        }
    }

    #[test]
    fn test_nested_activate_blocks() {
        let input = r#"activate user {
            user -> server: "Request";
            activate server {
                server -> database: "Query";
            };
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(result.is_ok(), "Nested activate blocks should be valid");

        let element = result.unwrap();
        if let types::Element::ActivateBlock {
            component,
            elements,
        } = element
        {
            assert_eq!(component.inner(), &"user");
            assert_eq!(elements.len(), 2); // relation + nested activate block

            // Verify nested activate block exists
            let has_nested_activate = elements
                .iter()
                .any(|el| matches!(el, types::Element::ActivateBlock { .. }));
            assert!(has_nested_activate, "Should contain nested activate block");
        } else {
            panic!("Expected ActivateBlock element");
        }
    }

    #[test]
    fn test_activate_block_with_components() {
        let input = r#"activate user {
            service: Rectangle;
            user -> service: "Call";
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = activate_block(&mut token_slice);
        assert!(
            result.is_ok(),
            "Activate block with components should be valid"
        );

        let element = result.unwrap();
        if let types::Element::ActivateBlock {
            component,
            elements,
        } = element
        {
            assert_eq!(component.inner(), &"user");
            assert_eq!(elements.len(), 2); // component + relation
        } else {
            panic!("Expected ActivateBlock element");
        }
    }

    // Fragment keyword sugar syntax tests

    #[test]
    fn test_opt_block_parsing() {
        let input = r#"opt "user authenticated" {
            user -> profile: "Load";
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = opt_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse opt block: {:?}", result);

        let element = result.unwrap();
        if let types::Element::OptBlock {
            keyword_span,
            section,
            attributes,
        } = element
        {
            assert!(keyword_span.start() < keyword_span.end());
            assert_eq!(
                section.title.as_ref().unwrap().inner(),
                "user authenticated"
            );
            assert_eq!(section.elements.len(), 1);
            assert!(attributes.is_empty());
        } else {
            panic!("Expected OptBlock element, got {:?}", element);
        }
    }

    #[test]
    fn test_opt_block_with_attributes() {
        let input = r##"opt [background_color="#f0f0f0", border_style="dashed"] "condition" {
            a -> b;
        };"##;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = opt_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse opt block with attributes");

        let element = result.unwrap();
        if let types::Element::OptBlock { attributes, .. } = element {
            assert_eq!(attributes.len(), 2);
        } else {
            panic!("Expected OptBlock element");
        }
    }

    #[test]
    fn test_opt_block_no_title() {
        let input = r#"opt {
            a -> b;
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = opt_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse opt block without title");

        let element = result.unwrap();
        if let types::Element::OptBlock { section, .. } = element {
            assert!(section.title.is_none());
        } else {
            panic!("Expected OptBlock element");
        }
    }

    #[test]
    fn test_loop_block_parsing() {
        let input = r#"loop "for each item" {
            client -> server: "Process";
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = loop_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse loop block: {:?}", result);

        let element = result.unwrap();
        if let types::Element::LoopBlock {
            keyword_span,
            section,
            ..
        } = element
        {
            assert!(keyword_span.start() < keyword_span.end());
            assert_eq!(section.title.as_ref().unwrap().inner(), "for each item");
            assert_eq!(section.elements.len(), 1);
        } else {
            panic!("Expected LoopBlock element");
        }
    }

    #[test]
    fn test_break_block_parsing() {
        let input = r#"break "timeout" {
            client -> server: "Cancel";
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = break_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse break block: {:?}", result);

        let element = result.unwrap();
        if let types::Element::BreakBlock {
            keyword_span,
            section,
            ..
        } = element
        {
            assert!(keyword_span.start() < keyword_span.end());
            assert_eq!(section.title.as_ref().unwrap().inner(), "timeout");
        } else {
            panic!("Expected BreakBlock element");
        }
    }

    #[test]
    fn test_critical_block_parsing() {
        let input = r#"critical "database lock" {
            app -> db: "UPDATE";
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = critical_block(&mut token_slice);
        assert!(
            result.is_ok(),
            "Failed to parse critical block: {:?}",
            result
        );

        let element = result.unwrap();
        if let types::Element::CriticalBlock {
            keyword_span,
            section,
            ..
        } = element
        {
            assert!(keyword_span.start() < keyword_span.end());
            assert_eq!(section.title.as_ref().unwrap().inner(), "database lock");
        } else {
            panic!("Expected CriticalBlock element");
        }
    }

    #[test]
    fn test_alt_else_block_parsing() {
        let input = r#"alt "x > 0" {
            a -> b;
        } else "x < 0" {
            b -> a;
        } else {
            a -> a;
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = alt_else_block(&mut token_slice);
        assert!(
            result.is_ok(),
            "Failed to parse alt/else block: {:?}",
            result
        );

        let element = result.unwrap();
        if let types::Element::AltElseBlock {
            keyword_span,
            sections,
            ..
        } = element
        {
            assert!(keyword_span.start() < keyword_span.end());
            assert_eq!(sections.len(), 3);
            assert_eq!(sections[0].title.as_ref().unwrap().inner(), "x > 0");
            assert_eq!(sections[1].title.as_ref().unwrap().inner(), "x < 0");
            assert!(sections[2].title.is_none());
        } else {
            panic!("Expected AltElseBlock element");
        }
    }

    #[test]
    fn test_alt_block_single_branch() {
        let input = r#"alt "condition" {
            a -> b;
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = alt_else_block(&mut token_slice);
        assert!(
            result.is_ok(),
            "Failed to parse alt block with single branch"
        );

        let element = result.unwrap();
        if let types::Element::AltElseBlock { sections, .. } = element {
            assert_eq!(sections.len(), 1);
        } else {
            panic!("Expected AltElseBlock element");
        }
    }

    #[test]
    fn test_par_block_parsing() {
        let input = r#"par "thread 1" {
            a -> b;
        } par "thread 2" {
            c -> d;
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = par_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse par block: {:?}", result);

        let element = result.unwrap();
        if let types::Element::ParBlock {
            keyword_span,
            sections,
            ..
        } = element
        {
            assert!(keyword_span.start() < keyword_span.end());
            assert_eq!(sections.len(), 2);
            assert_eq!(sections[0].title.as_ref().unwrap().inner(), "thread 1");
            assert_eq!(sections[1].title.as_ref().unwrap().inner(), "thread 2");
        } else {
            panic!("Expected ParBlock element");
        }
    }

    #[test]
    fn test_par_block_single_branch() {
        let input = r#"par "single thread" {
            a -> b;
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = par_block(&mut token_slice);
        assert!(
            result.is_ok(),
            "Failed to parse par block with single branch"
        );

        let element = result.unwrap();
        if let types::Element::ParBlock { sections, .. } = element {
            assert_eq!(sections.len(), 1);
        } else {
            panic!("Expected ParBlock element");
        }
    }

    #[test]
    fn test_nested_fragment_keywords() {
        let input = r#"opt "outer" {
            alt "inner condition" {
                a -> b;
            } else {
                b -> a;
            };
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = opt_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse nested fragment keywords");

        let element = result.unwrap();
        if let types::Element::OptBlock { section, .. } = element {
            assert_eq!(section.elements.len(), 1);
            // Inner element should be an AltElseBlock
            if let types::Element::AltElseBlock { .. } = &section.elements[0] {
                // Success
            } else {
                panic!("Expected nested AltElseBlock");
            }
        } else {
            panic!("Expected OptBlock element");
        }
    }

    #[test]
    fn test_fragment_keyword_with_empty_body() {
        let input = r#"opt "empty" {
        };"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = opt_block(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse opt block with empty body");

        let element = result.unwrap();
        if let types::Element::OptBlock { section, .. } = element {
            assert_eq!(section.elements.len(), 0);
        } else {
            panic!("Expected OptBlock element");
        }
    }

    #[test]
    fn test_note_element_simple() {
        let input = r#"note: "This is a simple note";"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = note_element(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse simple note element");

        let element = result.unwrap();
        if let types::Element::Note(note) = element {
            assert_eq!(note.attributes.len(), 0);
            assert_eq!(note.content.inner(), "This is a simple note");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_note_element_with_attributes() {
        let input = r#"note [align="left"]: "Note with attributes";"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = note_element(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse note with attributes");

        let element = result.unwrap();
        if let types::Element::Note(note) = element {
            assert_eq!(note.attributes.len(), 1);
            assert_eq!(*note.attributes[0].name.inner(), "align");
            assert_eq!(note.content.inner(), "Note with attributes");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_note_element_missing_semicolon() {
        let input = r#"note: "Missing semicolon""#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = note_element(&mut token_slice);
        assert!(result.is_err(), "Should fail without semicolon");
    }

    #[test]
    fn test_note_element_missing_content() {
        let input = r#"note [align="left"]: ;"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = note_element(&mut token_slice);
        assert!(result.is_err(), "Should fail without content");
    }

    #[test]
    fn test_note_element_with_whitespace_and_comments() {
        let input = r#"note  // comment
        [align="right"]  // another comment
        :  // more comments
        "Content with spacing"  ;  // final comment
        "#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = note_element(&mut token_slice);
        assert!(
            result.is_ok(),
            "Failed to parse note with whitespace and comments"
        );

        let element = result.unwrap();
        if let types::Element::Note(note) = element {
            assert_eq!(note.attributes.len(), 1);
            assert_eq!(note.content.inner(), "Content with spacing");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_note_in_complete_diagram() {
        let input = r#"diagram sequence;

        client: Rectangle;
        server: Rectangle;
        database: Rectangle;

        note: "This is a margin note";
        note [on=[client]]: "Note on client";

        client -> server: "Request";

        note [on=[server], align="right"]: "Processing request";

        server -> database: "Query";
        note [on=[server, database]]: "Note spanning multiple elements";

        database -> server: "Result";
        server -> client: "Response";
        "#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = diagram(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse diagram with notes");

        let element = result.unwrap();

        if let types::Element::Diagram(diagram) = element {
            // Count note elements
            let note_count = diagram
                .elements
                .iter()
                .filter(|e| matches!(e, types::Element::Note(_)))
                .count();
            assert_eq!(note_count, 4, "Expected 4 notes in diagram");

            // Verify first note is a margin note with no attributes
            if let Some(types::Element::Note(note)) = diagram
                .elements
                .iter()
                .find(|e| matches!(e, types::Element::Note(_)))
            {
                assert_eq!(note.content.inner(), "This is a margin note");
                assert_eq!(
                    note.attributes.len(),
                    0,
                    "Margin note should have no attributes"
                );
            } else {
                panic!("Expected to find note element");
            }

            // Verify we have notes with 'on' attribute
            let notes_with_on: Vec<_> = diagram
                .elements
                .iter()
                .filter_map(|e| match e {
                    types::Element::Note(note) => Some(note),
                    _ => None,
                })
                .filter(|note| {
                    note.attributes
                        .iter()
                        .any(|attr| *attr.name.inner() == "on")
                })
                .collect();
            assert_eq!(
                notes_with_on.len(),
                3,
                "Expected 3 notes with 'on' attribute"
            );
        } else {
            panic!("Expected Diagram element");
        }
    }

    #[test]
    fn test_identifiers_empty() {
        let input = r#"[]"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        // Empty lists are now handled by empty_brackets parser, not identifier_list
        let result = attribute_value(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse empty brackets");

        if let types::AttributeValue::Empty = result.unwrap() {
            // Success
        } else {
            panic!("Expected Empty attribute value");
        }
    }

    #[test]
    fn test_identifiers_single() {
        let input = r#"[component]"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = identifiers(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse single identifier");

        let ids = result.unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].inner(), "Component");
    }

    #[test]
    fn test_identifiers_multiple() {
        let input = r#"[client, server, database]"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = identifiers(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse multiple identifiers");

        let ids = result.unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0].inner(), "client");
        assert_eq!(ids[1].inner(), "server");
        assert_eq!(ids[2].inner(), "database");
    }

    #[test]
    fn test_identifiers_nested() {
        let input = r#"[frontend::app, backend::api]"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = identifiers(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse nested identifiers");

        let ids = result.unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].inner(), "frontend::app");
        assert_eq!(ids[1].inner(), "backend::api");
    }

    #[test]
    fn test_identifiers_with_whitespace() {
        let input = r#"[ client , server ]"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = identifiers(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse with whitespace");

        let ids = result.unwrap();
        assert_eq!(ids.len(), 2);
    }
}
