//! Parser for Orrery source tokens.
//!
//! This module transforms a token stream from the [`lexer`](super::lexer) into
//! a parsed AST defined in [`parser_types`](super::parser_types). The public
//! entry point is [`build_diagram`].

use winnow::{
    Parser as _,
    combinator::{alt, delimited, opt, preceded, repeat, separated},
    error::{ContextError, ErrMode},
    stream::{Stream, TokenSlice},
    token::any,
};

use orrery_core::{identifier::Id, semantic::DiagramKind};

use crate::{
    error::{Diagnostic, ErrorCode},
    parser_types as types,
    span::{Span, Spanned},
    tokens::{PositionedToken, Token},
};

/// Context type for parser errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Context {
    /// Description of what is currently being parsed
    Label(&'static str),
    /// Remaining token count (`eof_offset()`) at error start position
    ///
    /// Used to calculate start_offset as: `tokens.len() - start_offset_value`
    StartOffset(usize),
}

type Input<'src> = OrreryTokenSlice<'src>;
type IResult<O> = std::result::Result<O, ErrMode<ContextError<Context>>>;
/// Type alias for winnow TokenSlice with our positioned tokens
type OrreryTokenSlice<'src> = TokenSlice<'src, PositionedToken<'src>>;

/// Helper function to create a spanned value
fn make_spanned<T>(value: T, span: Span) -> Spanned<T> {
    Spanned::new(value, span)
}

fn cut_err<'src, O, F>(input: &mut Input<'src>, f: F) -> IResult<O>
where
    F: FnOnce(&mut Input<'src>) -> IResult<O>,
{
    let start_remaining = input.eof_offset();

    match f(input) {
        Ok(o) => Ok(o),
        Err(ErrMode::Backtrack(mut e)) | Err(ErrMode::Cut(mut e)) => {
            e.push(Context::StartOffset(start_remaining));
            Err(ErrMode::Cut(e))
        }
        Err(e) => Err(e),
    }
}

/// Helper to create a Cut error with StartOffset context
fn cut_error_with_offset<'src>(input: &Input<'src>) -> ErrMode<ContextError<Context>> {
    let mut e = ContextError::new();
    e.push(Context::StartOffset(input.eof_offset()));
    ErrMode::Cut(e)
}

/// Helper to create a Cut error with a specific StartOffset value
fn cut_error_from_offset(start_offset: usize) -> ErrMode<ContextError<Context>> {
    let mut e = ContextError::new();
    e.push(Context::StartOffset(start_offset));
    ErrMode::Cut(e)
}

/// Helper to create a Backtrack error with a specific StartOffset value
fn backtrack_error_from_offset(start_offset: usize) -> ErrMode<ContextError<Context>> {
    let mut e = ContextError::new();
    e.push(Context::StartOffset(start_offset));
    ErrMode::Backtrack(e)
}

/// Parse whitespace and comments
fn ws_comment<'src>(input: &mut Input<'src>) -> IResult<()> {
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
fn ws_comments0<'src>(input: &mut Input<'src>) -> IResult<()> {
    repeat(0.., ws_comment).parse_next(input)
}

/// Parse one or more whitespace/comments
fn ws_comments1<'src>(input: &mut Input<'src>) -> IResult<()> {
    repeat(1.., ws_comment).parse_next(input)
}

/// Parse semicolon with optional whitespace
fn semicolon<'src>(input: &mut Input<'src>) -> IResult<()> {
    preceded(
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Semicolon))
            .void(),
    )
    .context(Context::Label("semicolon"))
    .parse_next(input)
}

/// Parse a raw identifier string with span preservation (low-level)
///
/// Returns the identifier as &str.
fn raw_identifier<'src>(input: &mut Input<'src>) -> IResult<Spanned<&'src str>> {
    any.verify_map(|token: &PositionedToken<'_>| match &token.token {
        Token::Identifier(name) => Some(Spanned::new(*name, token.span)),
        // Allow keywords to be used as identifiers in appropriate contexts
        Token::Component => Some(Spanned::new("Component", token.span)),
        Token::Sequence => Some(Spanned::new("Sequence", token.span)),
        Token::Type => Some(Spanned::new("Type", token.span)),
        Token::Diagram => Some(Spanned::new("Diagram", token.span)),
        Token::Embed => Some(Spanned::new("Embed", token.span)),
        Token::As => Some(Spanned::new("As", token.span)),
        _ => None,
    })
    .context(Context::Label("identifier"))
    .parse_next(input)
}

/// Parse a standard identifier with span preservation (high-level)
///
/// Returns the identifier as an interned Id.
fn identifier<'src>(input: &mut Input<'src>) -> IResult<Spanned<Id>> {
    let raw = raw_identifier.parse_next(input)?;
    Ok(raw.map(|name| Id::new(name)))
}

/// Parse nested identifier with :: separators
fn nested_identifier<'src>(input: &mut Input<'src>) -> IResult<Spanned<Id>> {
    let first = identifier.parse_next(input)?;
    let mut current_id = *first.inner();
    let mut unified_span = first.span();

    loop {
        // Try to parse `::` using DoubleColon token
        let checkpoint = input.checkpoint();
        let double_colon_result = any::<_, ErrMode<ContextError>>
            .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::DoubleColon))
            .parse_next(input);

        match double_colon_result {
            Ok(_) => {
                // Successfully parsed `::`, now parse the identifier
                let next = identifier.parse_next(input)?;
                current_id = current_id.create_nested(*next.inner());
                unified_span = unified_span.union(next.span());
            }
            Err(_) => {
                // Failed to parse `::`, reset and exit loop
                input.reset(&checkpoint);
                break;
            }
        }
    }

    Ok(Spanned::new(current_id, unified_span))
}

/// Parse string literal
fn string_literal<'src>(input: &mut Input<'src>) -> IResult<Spanned<String>> {
    any.verify_map(|token: &PositionedToken<'_>| match &token.token {
        Token::StringLiteral(s) => Some(Spanned::new(s.clone(), token.span)),
        _ => None,
    })
    .context(Context::Label("string literal"))
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
fn identifiers<'src>(input: &mut Input<'src>) -> IResult<Vec<Spanned<Id>>> {
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
                let first_id = nested_identifier.parse_next(input)?;
                ws_comments0.parse_next(input)?;

                // Disambiguation: Check if next token is '='
                // If so, this is nested attributes [name=value], not identifiers
                let checkpoint = input.checkpoint();
                let result: IResult<_> = any
                    .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Equals))
                    .parse_next(input);
                if result.is_ok() {
                    // Found '=' - this is nested attributes, backtrack
                    return Err(ErrMode::Backtrack(ContextError::new()));
                }
                input.reset(&checkpoint);

                // Build identifier list starting with first identifier
                let mut ids = vec![first_id];

                // Parse remaining identifiers separated by commas
                // Uses `repeat` combinator for clean separation of concerns
                let rest: Vec<Spanned<Id>> = repeat(
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
                        nested_identifier,
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
    .context(Context::Label("identifiers"))
    .parse_next(input)
}

/// Parse empty brackets: `[]`
///
/// Returns an Empty attribute value that can be interpreted as either
/// empty identifiers or empty nested attributes depending on context.
fn empty_brackets<'src>(input: &mut Input<'src>) -> IResult<()> {
    delimited(
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBracket)),
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBracket)),
    )
    .context(Context::Label("empty brackets"))
    .parse_next(input)
}

/// Parse an attribute value (string, float, identifier list, or type spec)
///
/// Attributes in Orrery can have different value types depending on their purpose:
///
/// **Value Types:**
/// 1. **Empty** - `[]` - Ambiguous empty brackets (can be identifiers or type specs)
/// 2. **Identifiers** - `[id1, id2, ...]` - List of element identifiers (used in `on` attribute)
/// 3. **TypeSpec** - `TypeName[attr=val]`, `TypeName`, or `[attr=val]`
/// 4. **String** - `"value"` - Text values (colors, names, alignment)
/// 5. **Float** - `2.5` or `10` - Numeric values (widths, sizes, dimensions)
fn attribute_value<'src>(input: &mut Input<'src>) -> IResult<types::AttributeValue<'src>> {
    alt((
        // Parse empty brackets [] first - can be interpreted as either empty identifiers or empty attributes
        empty_brackets.map(|_| types::AttributeValue::Empty),
        // Try identifiers: [id1, id2, ...]
        // This needs to be before attribute_type_spec since both start with '['
        identifiers.map(types::AttributeValue::Identifiers),
        // Parse type spec: TypeName[attrs], TypeName, or [attrs]
        attribute_type_spec.map(types::AttributeValue::TypeSpec),
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
    .context(Context::Label("attribute value"))
    .parse_next(input)
}

/// Parse a single attribute
fn attribute<'src>(input: &mut Input<'src>) -> IResult<types::Attribute<'src>> {
    let name = raw_identifier.parse_next(input)?;

    preceded(
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Equals)),
    )
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let value = attribute_value.parse_next(input)?;

    Ok(types::Attribute { name, value })
}

/// Parse comma-separated attributes
fn attributes<'src>(input: &mut Input<'src>) -> IResult<Vec<types::Attribute<'src>>> {
    separated(
        0..,
        attribute,
        (
            ws_comments0,
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Comma)),
            ws_comments0,
        ),
    )
    .context(Context::Label("attributes"))
    .parse_next(input)
}

/// Parse attributes wrapped in brackets
fn wrapped_attributes<'src>(input: &mut Input<'src>) -> IResult<Vec<types::Attribute<'src>>> {
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
    .context(Context::Label("wrapped attributes"))
    .parse_next(input)
}

/// Parse a TypeSpec for attribute values: TypeName\[attrs\], TypeName, or \[attrs\]
///
/// This is used for attribute values that support type specifications.
/// It handles three forms:
/// - `TypeName[attrs]` → TypeSpec with both type_name and attributes
/// - `TypeName` → TypeSpec with type_name but no attributes
/// - `[attrs]` → TypeSpec with no type_name (anonymous)
///
/// Returns:
/// - `TypeName[attrs]` → TypeSpec { type_name: Some(TypeName), attributes: [...] }
/// - `TypeName` → TypeSpec { type_name: Some(TypeName), attributes: [] }
/// - `[attrs]` → TypeSpec { type_name: None, attributes: [...] }
fn attribute_type_spec<'src>(input: &mut Input<'src>) -> IResult<types::TypeSpec<'src>> {
    alt((
        // Try TypeName[attrs] or TypeName (reuse type_spec)
        type_spec,
        // Try [attrs] only (anonymous TypeSpec)
        |input: &mut Input<'src>| {
            let attributes = wrapped_attributes.parse_next(input)?;
            Ok(types::TypeSpec {
                type_name: None,
                attributes,
            })
        },
    ))
    .context(Context::Label("attribute type spec"))
    .parse_next(input)
}

/// Parse a TypeSpec: TypeName\[attrs\] or TypeName
///
/// Returns a TypeSpec with:
/// - type_name: Always Some(id) - identifier is required
/// - attributes: parsed attributes if present, empty vec otherwise
fn type_spec<'src>(input: &mut Input<'src>) -> IResult<types::TypeSpec<'src>> {
    // Parse REQUIRED identifier
    let type_name = identifier.parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let attributes = opt(wrapped_attributes)
        .map(|attrs| attrs.unwrap_or_default())
        .parse_next(input)?;

    Ok(types::TypeSpec {
        type_name: Some(type_name),
        attributes,
    })
}

/// Parse an invocation type spec: @TypeName\[attrs\]? or \[attrs\] or nothing
///
/// This is used for invocations
/// Similar to `type_spec()` but with `@` prefix for the identifier.
///
/// Returns:
/// - `@TypeName[attrs]` → TypeSpec { type_name: Some(TypeName), attributes: [...] }
/// - `@TypeName` → TypeSpec { type_name: Some(TypeName), attributes: [] }
/// - `[attrs]` → TypeSpec { type_name: None, attributes: [...] }
/// - (nothing) → TypeSpec::default() (sugar syntax)
fn invocation_type_spec<'src>(input: &mut Input<'src>) -> IResult<types::TypeSpec<'src>> {
    // Parse optional @TypeName
    // If @ is present, identifier is REQUIRED (cut_err prevents backtracking)
    // If @ is absent, we can still parse [attributes] or nothing (sugar)
    let type_name = opt(|input: &mut Input<'src>| {
        // Parse @ token
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::At))
            .parse_next(input)?;

        // After @, identifier is REQUIRED
        cut_err(input, |input| {
            ws_comments0.parse_next(input)?;
            identifier
                .context(Context::Label("type name after @"))
                .parse_next(input)
        })
    })
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let attributes = opt(wrapped_attributes)
        .map(|attrs| attrs.unwrap_or_default())
        .parse_next(input)?;

    Ok(types::TypeSpec {
        type_name,
        attributes,
    })
}

/// Parse a type definition
///
/// Syntax: `type Name = TypeSpec;`
///
/// Examples:
/// - `type Button = Rectangle;`
/// - `type StyledBox = Rectangle[fill_color="blue", border_width="2"];`
fn type_definition<'src>(input: &mut Input<'src>) -> IResult<types::TypeDefinition<'src>> {
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Type))
        .parse_next(input)?;

    ws_comments1.parse_next(input)?;
    let name = identifier.parse_next(input)?;

    preceded(
        ws_comments0,
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Equals)),
    )
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let type_spec = type_spec.parse_next(input)?;

    semicolon.parse_next(input)?;

    Ok(types::TypeDefinition { name, type_spec })
}

/// Parse type definitions section
fn type_definitions<'src>(input: &mut Input<'src>) -> IResult<Vec<types::TypeDefinition<'src>>> {
    repeat(0.., preceded(ws_comments0, type_definition)).parse_next(input)
}

/// Parse relation type (arrow with optional type specification)
fn relation_type<'src>(input: &mut Input<'src>) -> IResult<&'src str> {
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
///
/// Syntax: `identifier as "display_name" : TypeSpec { nested_elements };`
///
/// Examples:
/// - `user: Person;`
/// - `server as "API Server": Rectangle[fill_color="blue"];`
/// - `container: Box { nested_component: Circle; };`
fn component_with_elements<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    let name = identifier.parse_next(input)?;

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

    let type_spec = type_spec.parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Optional nested content: either embedded diagram or nested elements in braces
    let nested_elements = opt(alt((
        // Try parsing embedded diagram first (starts with 'embed' keyword)
        embedded_diagram.map(|diag| vec![diag]),
        // Fall back to nested elements in braces
        delimited(
            (
                any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace)),
                ws_comments0,
            ),
            elements,
            (
                ws_comments0,
                any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace)),
            ),
        ),
    )))
    .map(|nested| nested.unwrap_or_default())
    .parse_next(input)?;

    ws_comments0.parse_next(input)?;
    semicolon.parse_next(input)?;

    Ok(types::Element::Component {
        name,
        display_name,
        type_spec,
        nested_elements,
    })
}

/// Parse a complete relation statement
///
/// Syntax: `source -> @TypeSpec target : "label";`
///
/// Examples:
/// - `user -> server;`
/// - `user -> @AsyncCall server: "Request";`
/// - `user -> @AsyncCall[color="blue"] server: "Request";`
/// - `user -> [color="red"] server;` (anonymous TypeSpec)
fn relation<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    let source = nested_identifier.parse_next(input)?;

    ws_comments0.parse_next(input)?;
    let relation_type = relation_type.parse_next(input)?;

    // After parsing arrow, commit to parsing relation
    cut_err(input, |input| {
        ws_comments0.parse_next(input)?;

        let type_spec = opt(invocation_type_spec)
            .parse_next(input)?
            .unwrap_or_default();

        ws_comments0.parse_next(input)?;
        let target = nested_identifier
            .context(Context::Label("target identifier after arrow"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Optional relation label as string literal
        let label = opt(preceded(
            any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Colon)),
            preceded(ws_comments0, string_literal),
        ))
        .parse_next(input)?;

        semicolon
            .context(Context::Label("semicolon after relation"))
            .parse_next(input)?;

        Ok(types::Element::Relation {
            source,
            target,
            relation_type: make_spanned(relation_type, Span::new(0..0)), // TODO: track proper span
            type_spec,
            label,
        })
    })
}

/// Parse an activate block
///
/// ## Grammar:
///   `activate @TypeSpec <nested_identifier> { <elements> };`
///
/// ## Notes:
/// - Accepts nested identifiers (e.g., `parent::child`) and returns [`Spanned<String>`]
///   for the component. The element span equals the identifier span; the `activate`
///   keyword and the trailing semicolon are not included in the element span
///   (consistent with `Element::span()` semantics using the inner `component` span).
/// - Whitespace and line comments are allowed between tokens as handled by
///   `ws_comments0/1`.
fn activate_block<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    // Parse "activate" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Activate))
        .context(Context::Label("activate keyword"))
        .parse_next(input)?;

    // Require at least one space or comment after the keyword
    ws_comments1
        .context(Context::Label("whitespace after activate"))
        .parse_next(input)?;

    let type_spec = opt(invocation_type_spec)
        .parse_next(input)?
        .unwrap_or_default();

    ws_comments0.parse_next(input)?;

    let component = nested_identifier
        .context(Context::Label("component nested identifier"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse opening brace
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
        .context(Context::Label("opening brace '{'"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse nested elements
    let nested_elements = elements
        .context(Context::Label("activate block content"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse closing brace
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
        .context(Context::Label("closing brace '}'"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse semicolon
    semicolon
        .context(Context::Label("semicolon after activate block"))
        .parse_next(input)?;

    Ok(types::Element::ActivateBlock {
        component,
        type_spec,
        elements: nested_elements,
    })
}

/// Parse a section block: `section "title" { elements };`
fn section_block<'src>(input: &mut Input<'src>) -> IResult<types::FragmentSection<'src>> {
    // Parse "section" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Section))
        .context(Context::Label("section keyword"))
        .parse_next(input)?;

    cut_err(input, |input| {
        // Optional whitespace or comments after the keyword
        ws_comments0.parse_next(input)?;

        // Optional section title as a spanned string literal
        let title = opt(string_literal.context(Context::Label("section title string literal")))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse opening brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
            .context(Context::Label("opening brace '{'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse nested elements inside the section
        let elems = elements
            .context(Context::Label("section content"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse closing brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
            .context(Context::Label("closing brace '}'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse semicolon after the section block
        semicolon
            .context(Context::Label("semicolon after section"))
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
) -> IResult<types::FragmentSection<'src>> {
    ws_comments0.parse_next(input)?;

    let title = opt(string_literal.context(Context::Label(title_context))).parse_next(input)?;

    ws_comments0.parse_next(input)?;

    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
        .context(Context::Label("opening brace '{'"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let elems = elements.parse_next(input)?;

    ws_comments0.parse_next(input)?;

    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
        .context(Context::Label("closing brace '}'"))
        .parse_next(input)?;

    Ok(types::FragmentSection {
        title,
        elements: elems,
    })
}

/// Macro for generating single-section fragment keyword parsers
macro_rules! single_section_parser {
    ($fn_name:ident, $token:ident, $title_ctx:expr, $element_variant:ident) => {
        fn $fn_name<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
            let keyword_token = any
                .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::$token))
                .context(Context::Label(concat!(stringify!($token), " keyword")))
                .parse_next(input)?;
            let keyword_span = keyword_token.span;

            cut_err(input, |input| {
                ws_comments0.parse_next(input)?;

                let type_spec = opt(invocation_type_spec)
                    .parse_next(input)?
                    .unwrap_or_default();

                let section = parse_section_content(input, $title_ctx)?;

                ws_comments0.parse_next(input)?;
                semicolon
                    .context(Context::Label(concat!(
                        "semicolon after ",
                        stringify!($token),
                        " block"
                    )))
                    .parse_next(input)?;

                Ok(types::Element::$element_variant {
                    keyword_span,
                    type_spec,
                    section,
                })
            })
        }
    };
}

/// Macro for generating multi-section fragment keyword parsers
macro_rules! multi_section_parser {
    ($fn_name:ident, $first_token:ident, $cont_token:ident, $first_ctx:expr, $cont_ctx:expr, $element_variant:ident) => {
        fn $fn_name<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
            let keyword_token = any
                .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::$first_token))
                .context(Context::Label(concat!(
                    stringify!($first_token),
                    " keyword"
                )))
                .parse_next(input)?;
            let keyword_span = keyword_token.span;

            cut_err(input, |input| {
                ws_comments0.parse_next(input)?;

                let type_spec = opt(invocation_type_spec)
                    .parse_next(input)?
                    .unwrap_or_default();

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
                    .context(Context::Label(concat!(
                        "semicolon after ",
                        stringify!($first_token),
                        " block"
                    )))
                    .parse_next(input)?;

                Ok(types::Element::$element_variant {
                    keyword_span,
                    type_spec,
                    sections,
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

/// Parse a fragment block: `fragment @TypeSpec "operation" { section+ };`
fn fragment_block<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    // Parse "fragment" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Fragment))
        .context(Context::Label("fragment keyword"))
        .parse_next(input)?;

    cut_err(input, |input| {
        ws_comments0.parse_next(input)?;

        let type_spec = opt(invocation_type_spec)
            .parse_next(input)?
            .unwrap_or_default();

        ws_comments0.parse_next(input)?;

        // Parse the fragment operation (title) as a spanned string literal
        let operation = string_literal
            .context(Context::Label("fragment operation string literal"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse opening brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
            .context(Context::Label("opening brace '{'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse one or more sections
        let sections = repeat(1.., preceded(ws_comments0, section_block))
            .context(Context::Label("fragment sections"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse closing brace
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
            .context(Context::Label("closing brace '}'"))
            .parse_next(input)?;

        ws_comments0.parse_next(input)?;

        // Parse semicolon after the fragment block
        semicolon
            .context(Context::Label("semicolon after fragment"))
            .parse_next(input)?;

        Ok(types::Element::Fragment(types::Fragment {
            operation,
            type_spec,
            sections,
        }))
    })
}

/// Parse an explicit activate statement
///
/// ## Grammar:
///   `activate @TypeSpec <nested_identifier> ;`
///
/// ## Where:
/// - `@TypeSpec` is optional invocation type spec: `@TypeName[attrs]`, `@TypeName`, or omitted (sugar)
/// - `<nested_identifier>` supports `::`-qualified component names
/// - Optional whitespace and line comments are permitted between tokens
fn activate_statement<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    // Parse "activate" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Activate))
        .context(Context::Label("activate keyword"))
        .parse_next(input)?;

    ws_comments1
        .context(Context::Label("whitespace after activate"))
        .parse_next(input)?;

    let type_spec = opt(invocation_type_spec)
        .parse_next(input)?
        .unwrap_or_default();

    ws_comments0.parse_next(input)?;

    let component = nested_identifier
        .context(Context::Label("component nested identifier"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    semicolon
        .context(Context::Label("semicolon after activate statement"))
        .parse_next(input)?;

    Ok(types::Element::Activate {
        component,
        type_spec,
    })
}

/// Parse an explicit deactivate statement: `deactivate <nested_identifier>;`
fn deactivate_statement<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    // Parse "deactivate" keyword
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Deactivate))
        .context(Context::Label("deactivate keyword"))
        .parse_next(input)?;

    ws_comments1
        .context(Context::Label("whitespace after deactivate"))
        .parse_next(input)?;

    let component = nested_identifier
        .context(Context::Label("component nested identifier"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    semicolon
        .context(Context::Label("semicolon after deactivate statement"))
        .parse_next(input)?;

    Ok(types::Element::Deactivate { component })
}

/// Parse an activate element (explicit statement or block) with checkpoint routing
///
/// Behavior:
/// - If an `activate {` block is present, parse the block
/// - Otherwise, parse an explicit `activate <nested_identifier>;` statement
fn activate_element<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
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

/// Parse a note element: `note @TypeSpec: "content";`
///
/// Syntax:
/// - `note` keyword
/// - Optional `@TypeSpec` (invocation pattern with `@` prefix or sugar syntax)
/// - `:` separator
/// - String literal content
/// - `;` terminator
///
/// Examples:
/// - `note: "Simple note";`
/// - `note @NoteType: "Typed note";`
/// - `note @NoteType[on=[component]]: "Note attached to component";`
/// - `note [on=[a, b], align="left"]: "Note with anonymous TypeSpec";`
fn note_element<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    // Parse 'note' keyword
    let _ = any
        .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Note))
        .context(Context::Label("note keyword"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    let type_spec = opt(invocation_type_spec)
        .parse_next(input)?
        .unwrap_or_default();

    ws_comments0.parse_next(input)?;

    // Parse colon separator
    let _ = any
        .verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Colon))
        .context(Context::Label("colon"))
        .parse_next(input)?;

    ws_comments0.parse_next(input)?;

    // Parse string literal content
    let content = string_literal.parse_next(input)?;

    // Parse semicolon
    semicolon.parse_next(input)?;

    Ok(types::Element::Note(types::Note { type_spec, content }))
}
/// Parse any element (component or relation)
fn elements<'src>(input: &mut Input<'src>) -> IResult<Vec<types::Element<'src>>> {
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
                invalid_statement_with_semicolon,
            )),
        ),
    )
    .parse_next(input)
}

/// Catch-all parser for invalid syntax.
/// This is used as the last alternative in elements() to provide
/// better error reporting when no valid parser matches.
///
/// Strategy: Consume tokens up to semicolon or until hitting a block delimiter.
/// Returns Cut error if semicolon found or if meaningful tokens consumed before delimiter.
fn invalid_statement_with_semicolon<'src>(
    input: &mut Input<'src>,
) -> IResult<types::Element<'src>> {
    let mut consumed_meaningful_tokens = false;
    let start_offset = input.eof_offset();

    // Consume tokens until we find semicolon or hit a block delimiter
    loop {
        let checkpoint = input.checkpoint();
        match any::<_, ErrMode<ContextError>>.parse_next(input) {
            Ok(token) => {
                // Track if we consumed a meaningful content token
                // Exclude whitespace, newlines, and delimiters (which are structural)
                if !matches!(
                    token.token,
                    Token::Whitespace | Token::Newline | Token::RightBrace | Token::RightBracket
                ) {
                    consumed_meaningful_tokens = true;
                }

                // Check if it's a block delimiter - don't consume it
                // If we consumed meaningful tokens before delimiter, that's an error (missing semicolon)
                if matches!(token.token, Token::RightBrace | Token::RightBracket) {
                    input.reset(&checkpoint);
                    if consumed_meaningful_tokens {
                        return Err(cut_error_from_offset(start_offset));
                    }
                    break;
                }

                if matches!(token.token, Token::Semicolon) {
                    return Err(cut_error_from_offset(start_offset));
                }
            }
            Err(_) => {
                input.reset(&checkpoint);
                break;
            }
        }
    }

    Err(backtrack_error_from_offset(start_offset))
}

/// Parse diagram type (component, sequence, etc.)
fn diagram_type<'src>(input: &mut Input<'src>) -> IResult<Spanned<DiagramKind>> {
    any.verify_map(|token: &PositionedToken<'_>| match &token.token {
        Token::Component => Some(make_spanned(DiagramKind::Component, token.span)),
        Token::Sequence => Some(make_spanned(DiagramKind::Sequence, token.span)),
        _ => None,
    })
    .context(Context::Label("diagram type"))
    .parse_next(input)
}

/// Parse diagram header with unwrapped attributes
fn diagram_header<'src>(
    input: &mut Input<'src>,
) -> IResult<(Spanned<DiagramKind>, Vec<types::Attribute<'src>>)> {
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
) -> IResult<(Spanned<DiagramKind>, Vec<types::Attribute<'src>>)> {
    let (kind, attributes) = diagram_header.parse_next(input)?;
    semicolon.parse_next(input)?;
    Ok((kind, attributes))
}

/// Parse complete diagram
fn diagram<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    ws_comments0.parse_next(input)?;
    let (kind, attributes) = diagram_header_with_semicolon.parse_next(input)?;
    let type_definitions = type_definitions.parse_next(input)?;
    let elements = elements.parse_next(input)?;
    ws_comments0.parse_next(input)?;

    // Check if we've consumed all tokens (equivalent to `end()` in chumsky)
    if !input.is_empty() {
        return Err(cut_error_with_offset(input));
    }

    Ok(types::Element::Diagram(types::Diagram {
        kind,
        attributes,
        type_definitions,
        elements,
    }))
}

/// Parse an embedded diagram within a component
///
/// Syntax: `embed diagram [diagram_type] [[attributes]]? { type_definitions? elements }`
fn embedded_diagram<'src>(input: &mut Input<'src>) -> IResult<types::Element<'src>> {
    // Parse: embed
    any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::Embed))
        .parse_next(input)?;

    cut_err(input, |input| {
        ws_comments1.parse_next(input)?;

        // Parse: diagram [type] [attributes]?
        let (kind, attributes) = diagram_header.parse_next(input)?;
        ws_comments0.parse_next(input)?;

        // Parse: { type_definitions? elements }
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::LeftBrace))
            .parse_next(input)?;
        ws_comments0.parse_next(input)?;

        let type_definitions = type_definitions.parse_next(input)?;
        let elements = elements.parse_next(input)?;

        ws_comments0.parse_next(input)?;
        any.verify(|token: &PositionedToken<'_>| matches!(token.token, Token::RightBrace))
            .parse_next(input)?;

        Ok(types::Element::Diagram(types::Diagram {
            kind,
            attributes,
            type_definitions,
            elements,
        }))
    })
}

/// Utility function to convert winnow errors to our custom error format
///
/// Extracts position information from error context (StartOffset) and calculates
/// precise error spans using the token array.
fn convert_error(
    error: ErrMode<ContextError<Context>>,
    tokens: &[PositionedToken],
    current_remaining: usize,
) -> Diagnostic {
    // Extract start offset from error context if available
    let start_remaining = match &error {
        ErrMode::Backtrack(e) | ErrMode::Cut(e) => e.context().find_map(|ctx| match ctx {
            Context::StartOffset(n) => Some(*n),
            _ => None,
        }),
        _ => None,
    };

    // Calculate offsets from remaining token counts
    let end_offset = tokens.len() - current_remaining;
    let start_offset = start_remaining.map(|r| tokens.len() - r).unwrap_or(0);

    match error {
        ErrMode::Backtrack(e) | ErrMode::Cut(e) => {
            // Extract context information for better error messages
            let contexts: Vec<String> = e
                .context()
                .filter_map(|ctx| match ctx {
                    Context::Label(label) => Some(format!("expected {label}")),
                    _ => None,
                })
                .collect();

            let message = if contexts.is_empty() {
                "unexpected token or end of input".to_string()
            } else {
                contexts.join(" → ")
            };

            // Calculate error span from token positions
            // Determine the range to examine based on error type
            let error_span = {
                let examine_range = if start_offset < end_offset {
                    // Parser consumed tokens - examine that range
                    start_offset..end_offset
                } else if end_offset < tokens.len() {
                    if matches!(
                        tokens[end_offset].token,
                        Token::RightBrace | Token::RightBracket
                    ) {
                        // At delimiter without consuming - examine everything before it
                        // (e.g., missing semicolon before })
                        0..end_offset
                    } else {
                        // At specific non-delimiter token - examine just that token
                        end_offset..end_offset + 1
                    }
                } else {
                    // EOF - examine all tokens
                    0..tokens.len()
                };

                // Extract meaningful spans from the range and union them
                let slice = &tokens[examine_range];
                let first = slice
                    .iter()
                    .find(|t| !matches!(t.token, Token::Whitespace | Token::Newline))
                    .map(|t| t.span)
                    .unwrap_or(slice[0].span);
                let last = slice
                    .iter()
                    .rev()
                    .find(|t| !matches!(t.token, Token::Whitespace | Token::Newline))
                    .map(|t| t.span)
                    .unwrap_or(slice[slice.len() - 1].span);
                first.union(last)
            };

            Diagnostic::error(format!("unexpected token: {message}"))
                .with_code(ErrorCode::E100)
                .with_label(error_span, "unexpected token")
                .with_help("check syntax and token positioning")
        }
        ErrMode::Incomplete(_) => {
            // This should not happen as we are not supporting streaming input.
            let error_span = if end_offset < tokens.len() {
                tokens[end_offset].span
            } else {
                tokens
                    .iter()
                    .rev()
                    .find(|t| !matches!(t.token, Token::Whitespace | Token::Newline))
                    .map(|t| t.span)
                    .unwrap_or(tokens[tokens.len() - 1].span)
            };

            Diagnostic::error("incomplete input, more tokens expected")
                .with_code(ErrorCode::E101)
                .with_label(error_span, "incomplete")
                .with_help("ensure input is complete")
        }
    }
}

/// Build a diagram from tokens
pub fn build_diagram<'src>(
    tokens: &'src [PositionedToken<'src>],
) -> Result<Spanned<types::Element<'src>>, Diagnostic> {
    let mut token_slice = TokenSlice::new(tokens);

    match diagram.parse_next(&mut token_slice) {
        Ok(diagram) => {
            let total_span = tokens
                .first()
                .and_then(|f| tokens.last().map(|l| l.span.union(f.span)))
                .unwrap_or_default();

            Ok(make_spanned(diagram, total_span))
        }
        Err(e) => {
            let current_remaining = token_slice.eof_offset();
            Err(convert_error(e, tokens, current_remaining))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

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

        first.union(last)
    }

    /// Helper to create a token at a specific position
    fn make_token<'a>(token: Token<'a>, offset: usize, length: usize) -> PositionedToken<'a> {
        PositionedToken {
            token,
            span: Span::new(offset..offset + length),
        }
    }

    #[test]
    fn test_raw_identifier() {
        // Test that raw_identifier returns &str wrapped in Spanned
        let tokens = parse_tokens("test_name");
        let mut slice = TokenSlice::new(&tokens);
        let result = raw_identifier.parse_next(&mut slice);
        assert!(result.is_ok());
        let spanned_str = result.unwrap();
        assert_eq!(*spanned_str.inner(), "test_name");
        assert!(!spanned_str.span().is_empty());

        // Test with keyword used as identifier
        let tokens = parse_tokens("Component");
        let mut slice = TokenSlice::new(&tokens);
        let result = raw_identifier.parse_next(&mut slice);
        assert!(result.is_ok());
        let spanned_str = result.unwrap();
        assert_eq!(*spanned_str.inner(), "Component");
    }

    #[test]
    fn test_identifier() {
        let tokens = parse_tokens("test_id");
        let mut slice = TokenSlice::new(&tokens);
        let result = identifier.parse_next(&mut slice);
        assert!(result.is_ok());
        let spanned_id = result.unwrap();
        assert_eq!(*spanned_id.inner(), "test_id");
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
        let spanned_id = result.unwrap();
        assert_eq!(*spanned_id.inner(), "parent::child::grandchild");
    }

    #[test]
    fn test_simple_diagram() {
        let input = r#"diagram component;
        app: Rectangle;"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_relation() {
        let input = r#"diagram component;
        frontend -> backend;"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_relation_with_named_type() {
        let input = "a -> @Arrow b;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let result = relation(&mut slice);
        assert!(result.is_ok(), "Relation with @TypeName should parse");

        match result.unwrap() {
            types::Element::Relation { type_spec, .. } => {
                assert!(type_spec.type_name.is_some());
                assert_eq!(*type_spec.type_name.unwrap().inner(), "Arrow");
                assert!(type_spec.attributes.is_empty());
            }
            _ => panic!("Expected Relation element"),
        }
    }

    #[test]
    fn test_relation_with_named_type_and_attributes() {
        let input = "a -> @Arrow[color=\"red\", width=2] b;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let result = relation(&mut slice);
        assert!(
            result.is_ok(),
            "Relation with @TypeName[attrs] should parse"
        );

        match result.unwrap() {
            types::Element::Relation { type_spec, .. } => {
                assert!(type_spec.type_name.is_some());
                assert_eq!(*type_spec.type_name.unwrap().inner(), "Arrow");
                assert_eq!(type_spec.attributes.len(), 2);
                assert_eq!(*type_spec.attributes[0].name.inner(), "color");
                assert_eq!(*type_spec.attributes[1].name.inner(), "width");
            }
            _ => panic!("Expected Relation element"),
        }
    }

    #[test]
    fn test_relation_with_anonymous_attributes() {
        let input = "a -> [style=\"dashed\", width=3] b;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let result = relation(&mut slice);
        assert!(result.is_ok(), "Relation with [attrs] should parse");

        match result.unwrap() {
            types::Element::Relation { type_spec, .. } => {
                assert!(type_spec.type_name.is_none(), "Anonymous type has no name");
                assert_eq!(type_spec.attributes.len(), 2);
                assert_eq!(*type_spec.attributes[0].name.inner(), "style");
                assert_eq!(*type_spec.attributes[1].name.inner(), "width");
            }
            _ => panic!("Expected Relation element"),
        }
    }

    #[test]
    fn test_relation_with_sugar_syntax() {
        let input = "a -> b;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let result = relation(&mut slice);
        assert!(result.is_ok(), "Relation with sugar syntax should parse");

        match result.unwrap() {
            types::Element::Relation { type_spec, .. } => {
                assert!(type_spec.type_name.is_none());
                assert!(type_spec.attributes.is_empty());
            }
            _ => panic!("Expected Relation element"),
        }
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
            types::Element::Activate { component, .. } => {
                assert_eq!(*component.inner(), "user");
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
                assert_eq!(*component.inner(), "server");
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
            types::Element::Activate { component, .. } => {
                assert_eq!(*component.inner(), "user");
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
                assert_eq!(*component.inner(), "user");
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
            types::Element::Activate { component, .. } => {
                assert_eq!(*component.inner(), "parent::child");
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
    fn test_activate_statement_with_type_spec() {
        let input = "activate @MyType user;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = activate_statement.parse_next(&mut slice);
        assert!(elem.is_ok(), "activate with type_spec should parse");

        match elem.unwrap() {
            types::Element::Activate {
                component,
                type_spec,
            } => {
                assert_eq!(*component.inner(), "user");
                assert_eq!(
                    type_spec.type_name.as_ref().map(|id| *id.inner()),
                    Some(Id::new("MyType"))
                );
                assert!(type_spec.attributes.is_empty());
            }
            other => panic!("expected Activate element, got {:?}", other),
        }
    }

    #[test]
    fn test_activate_statement_with_type_spec_and_attributes() {
        let input = "activate @MyType[color=\"red\"] user;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = activate_statement.parse_next(&mut slice);
        assert!(
            elem.is_ok(),
            "activate with type_spec and attributes should parse"
        );

        match elem.unwrap() {
            types::Element::Activate {
                component,
                type_spec,
            } => {
                assert_eq!(*component.inner(), "user");
                assert_eq!(
                    type_spec.type_name.as_ref().map(|id| *id.inner()),
                    Some(Id::new("MyType"))
                );
                assert_eq!(type_spec.attributes.len(), 1);
                assert_eq!(*type_spec.attributes[0].name.inner(), "color");
            }
            other => panic!("expected Activate element, got {:?}", other),
        }
    }

    #[test]
    fn test_activate_statement_without_type_spec() {
        let input = "activate user;";
        let tokens = parse_tokens(input);
        let mut slice = TokenSlice::new(&tokens);

        let elem = activate_statement.parse_next(&mut slice);
        assert!(elem.is_ok(), "activate without type_spec should parse");

        match elem.unwrap() {
            types::Element::Activate {
                component,
                type_spec,
            } => {
                assert_eq!(*component.inner(), "user");
                assert_eq!(type_spec.type_name, None);
                assert!(type_spec.attributes.is_empty());
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
                assert_eq!(*component.inner(), "a::b::c");
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
            type_spec: _type_spec,
        } = element
        {
            assert_eq!(*component.inner(), "user");
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
        let input = r#"diagram component [layout="basic"];
        type CustomBox = Rectangle [color="blue"];

        frontend: CustomBox [label="Frontend"];
        backend: Rectangle;

        frontend -> backend: "API calls";"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens);
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
                Token::Identifier(name) => {
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
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_ok());

        let tokens = tokenize("\t").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_ok());

        // Test comment parsing
        let tokens = tokenize("// this is a comment").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_ok());

        // Test failure cases
        let tokens = tokenize("identifier").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(ws_comment(&mut input).is_err());
    }

    #[test]
    fn test_semicolon_function() {
        // Test basic semicolon
        let tokens = tokenize(";").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(semicolon(&mut input).is_ok());

        // Test semicolon with leading whitespace
        let tokens = tokenize("  ;").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(semicolon(&mut input).is_ok());

        // Test failure cases
        let tokens = tokenize(":").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(semicolon(&mut input).is_err());
    }

    #[test]
    fn test_identifier_function() {
        // Test basic identifiers with span validation
        let tokens = tokenize("hello").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = identifier(&mut input);
        assert!(result.is_ok());
        let spanned_id = result.unwrap();
        assert_eq!(*spanned_id.inner(), "hello");
        assert!(!spanned_id.span().is_empty());

        // Test keywords as identifiers
        let tokens = tokenize("Component").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = identifier(&mut input);
        assert!(result.is_ok());
        let spanned_id = result.unwrap();
        assert_eq!(*spanned_id.inner(), "Component");

        // Test failure cases
        let tokens = tokenize("->").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(identifier(&mut input).is_err());
    }

    #[test]
    fn test_nested_identifier_function() {
        // Test simple identifier
        let tokens = tokenize("simple").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = nested_identifier(&mut input);
        assert!(result.is_ok());
        let spanned_id = result.unwrap();
        assert_eq!(*spanned_id.inner(), "simple");
        assert!(!spanned_id.span().is_empty());

        // Test nested identifiers
        let tokens = tokenize("parent::child").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = nested_identifier(&mut input);
        assert!(result.is_ok());
        let spanned_id = result.unwrap();
        assert_eq!(*spanned_id.inner(), "parent::child");
    }

    #[test]
    fn test_string_literal_function() {
        // Test basic string literals
        let tokens = tokenize("\"hello\"").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = string_literal(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().inner(), "hello");

        // Test strings with escape sequences
        let tokens = tokenize("\"hello\\nworld\"").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = string_literal(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().inner(), "hello\nworld");

        // Test failure cases
        let tokens = tokenize("identifier").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(string_literal(&mut input).is_err());
    }

    #[test]
    fn test_attribute_function() {
        // Test basic attribute parsing
        let tokens = tokenize("color=\"red\"").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "color");
        assert!(matches!(&attr.value, types::AttributeValue::String(s) if s.inner() == "red"));

        // Test that unquoted identifiers are now valid as TypeSpec names
        let tokens = tokenize("stroke=RedStroke").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "stroke");
        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            assert_eq!(*type_spec.type_name.as_ref().unwrap().inner(), "RedStroke");
            assert_eq!(type_spec.attributes.len(), 0);
        } else {
            panic!("Expected TypeSpec for stroke=RedStroke");
        }

        // Test float attribute parsing
        let tokens = tokenize("width=2.5").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");

        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");
        // Empty brackets [] are parsed as Empty variant
        if let types::AttributeValue::Empty = &attr.value {
            // Verify it can be interpreted as empty attributes
            assert_eq!(attr.value.as_type_spec().unwrap().attributes.len(), 0);
        } else {
            panic!("Expected Empty attribute value for text=[]");
        }

        // Test single nested attribute
        let tokens = tokenize("text=[font_size=16]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test missing equals in nested attribute
        let tokens = tokenize("text=[font_size 12]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test missing value in nested attribute
        let tokens = tokenize("text=[font_size=]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test invalid comma usage in nested attributes
        let tokens = tokenize("text=[,font_size=12]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test trailing comma in nested attributes (should fail)
        let tokens = tokenize("text=[font_size=12,]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_err());

        // Test nested brackets - now supported as nested TypeSpec
        let tokens = tokenize("text=[style=[curved=true]]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_attribute_parsing() {
        // Test complete text attribute group
        let tokens = tokenize(
            "text=[font_size=16, font_family=\"Arial\", background_color=\"white\", padding=8.0]",
        )
        .expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");

        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        assert_eq!(*attr.name.inner(), "text");
        // Empty brackets [] are parsed as Empty variant
        if let types::AttributeValue::Empty = &attr.value {
            // Verify it can be interpreted as empty attributes
            assert_eq!(attr.value.as_type_spec().unwrap().attributes.len(), 0);
        } else {
            panic!("Expected Empty attribute value for text=[]");
        }

        // Test single text attribute
        let tokens = tokenize("text=[font_size=20]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();
        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = attribute(&mut input);
        assert!(result.is_ok());
        let attr = result.unwrap();

        if let types::AttributeValue::TypeSpec(type_spec) = &attr.value {
            let nested_attrs = &type_spec.attributes;
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = wrapped_attributes(&mut input);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test single attribute in brackets
        let tokens = tokenize("[color=\"red\"]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
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
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = type_definition(&mut input);
        assert!(result.is_ok());
        let typedef = result.unwrap();
        assert_eq!(*typedef.name.inner(), "MyType");
        assert_eq!(
            *typedef.type_spec.type_name.as_ref().unwrap().inner(),
            "Rectangle"
        );

        // Test failure cases
        let tokens = tokenize("MyType = Rectangle;").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(type_definition(&mut input).is_err());
    }

    #[test]
    fn test_type_spec_function() {
        // Test: TypeName only
        let tokens = tokenize("Rectangle").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = type_spec(&mut input);
        assert!(result.is_ok(), "TypeName should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Rectangle");
        assert!(spec.attributes.is_empty());

        // Test: TypeName[single attribute]
        let tokens = tokenize("Rectangle[fill_color=\"blue\"]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = type_spec(&mut input);
        assert!(result.is_ok(), "TypeName[single attribute] should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Rectangle");
        assert_eq!(spec.attributes.len(), 1);
        assert_eq!(*spec.attributes[0].name.inner(), "fill_color");

        // Test: TypeName[multiple attributes]
        let tokens =
            tokenize("Service[fill=\"blue\", size=100, active=1]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = type_spec(&mut input);
        assert!(result.is_ok(), "TypeName[multiple attributes] should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Service");
        assert_eq!(spec.attributes.len(), 3);
        assert_eq!(*spec.attributes[0].name.inner(), "fill");
        assert_eq!(*spec.attributes[1].name.inner(), "size");
        assert_eq!(*spec.attributes[2].name.inner(), "active");

        // Test: TypeName[nested attributes]
        let tokens =
            tokenize("Rectangle[text=[font=\"Arial\", size=12]]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = type_spec(&mut input);
        assert!(result.is_ok(), "TypeName[nested attributes] should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Rectangle");
        assert_eq!(spec.attributes.len(), 1);
        assert_eq!(*spec.attributes[0].name.inner(), "text");
        match &spec.attributes[0].value {
            types::AttributeValue::TypeSpec(type_spec) => {
                let nested = &type_spec.attributes;
                assert_eq!(nested.len(), 2);
                assert_eq!(*nested[0].name.inner(), "font");
                assert_eq!(*nested[1].name.inner(), "size");
            }
            _ => panic!("Expected nested attributes"),
        }

        // Test: TypeName[] (empty attributes)
        let tokens = tokenize("Rectangle[]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = type_spec(&mut input);
        assert!(result.is_ok(), "TypeName[] should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Rectangle");
        assert!(spec.attributes.is_empty());

        // Test: Missing TypeName should FAIL (TypeName is required)
        let tokens = tokenize("[fill_color=\"blue\"]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = type_spec(&mut input);
        assert!(
            result.is_err(),
            "type_spec requires TypeName - [attributes] alone should fail"
        );
    }

    #[test]
    fn test_invocation_type_spec_function() {
        // Test: @TypeName
        let tokens = tokenize("@Arrow").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(result.is_ok(), "@TypeName should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Arrow");
        assert!(spec.attributes.is_empty());

        // Test: @TypeName[single attribute]
        let tokens = tokenize("@Arrow[color=\"red\"]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(result.is_ok(), "@TypeName[single attribute] should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Arrow");
        assert_eq!(spec.attributes.len(), 1);
        assert_eq!(*spec.attributes[0].name.inner(), "color");

        // Test: @TypeName[multiple attributes]
        let tokens = tokenize("@Arrow[color=\"red\", width=2, style=\"dashed\"]")
            .expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(
            result.is_ok(),
            "@TypeName[multiple attributes] should parse"
        );
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Arrow");
        assert_eq!(spec.attributes.len(), 3);
        assert_eq!(*spec.attributes[0].name.inner(), "color");
        assert_eq!(*spec.attributes[1].name.inner(), "width");
        assert_eq!(*spec.attributes[2].name.inner(), "style");

        // Test: @TypeName[nested attributes]
        let tokens =
            tokenize("@Arrow[stroke=[color=\"blue\", width=3]]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(result.is_ok(), "@TypeName[nested attributes] should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(*spec.type_name.unwrap().inner(), "Arrow");
        assert_eq!(spec.attributes.len(), 1);
        assert_eq!(*spec.attributes[0].name.inner(), "stroke");
        match &spec.attributes[0].value {
            types::AttributeValue::TypeSpec(type_spec) => {
                let nested = &type_spec.attributes;
                assert_eq!(nested.len(), 2);
                assert_eq!(*nested[0].name.inner(), "color");
                assert_eq!(*nested[1].name.inner(), "width");
            }
            _ => panic!("Expected nested attributes"),
        }

        // Test: [single attribute] without @ (anonymous)
        let tokens = tokenize("[color=\"red\"]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(
            result.is_ok(),
            "[single attribute] without @ should parse (anonymous)"
        );
        let spec = result.unwrap();
        assert!(spec.type_name.is_none());
        assert_eq!(spec.attributes.len(), 1);
        assert_eq!(*spec.attributes[0].name.inner(), "color");

        // Test: [multiple attributes] without @ (anonymous)
        let tokens =
            tokenize("[style=\"dashed\", width=2, color=\"blue\"]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(
            result.is_ok(),
            "[multiple attributes] without @ should parse"
        );
        let spec = result.unwrap();
        assert!(spec.type_name.is_none());
        assert_eq!(spec.attributes.len(), 3);
        assert_eq!(*spec.attributes[0].name.inner(), "style");
        assert_eq!(*spec.attributes[1].name.inner(), "width");
        assert_eq!(*spec.attributes[2].name.inner(), "color");

        // Test: [nested attributes] without @ (anonymous)
        let tokens = tokenize("[stroke=[color=\"green\", width=1.5]]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(result.is_ok(), "[nested attributes] should parse");
        let spec = result.unwrap();
        assert!(spec.type_name.is_none());
        assert_eq!(spec.attributes.len(), 1);
        match &spec.attributes[0].value {
            types::AttributeValue::TypeSpec(type_spec) => {
                let nested = &type_spec.attributes;
                assert_eq!(nested.len(), 2);
            }
            _ => panic!("Expected nested attributes"),
        }

        // Test: [] empty brackets (anonymous with no attributes)
        let tokens = tokenize("[]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(result.is_ok(), "[] should parse as empty anonymous type");
        let spec = result.unwrap();
        assert!(spec.type_name.is_none());
        assert!(spec.attributes.is_empty());

        // Test: Nothing (sugar syntax) - returns empty TypeSpec
        let tokens = tokenize("server").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(result.is_ok(), "Nothing should return empty TypeSpec");
        let spec = result.unwrap();
        assert!(spec.type_name.is_none());
        assert!(spec.attributes.is_empty());

        // Test: @[attributes] SHOULD FAIL - TypeName required after @
        let tokens = tokenize("@[color=\"red\"]").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = invocation_type_spec(&mut input);
        assert!(
            result.is_err(),
            "@[attributes] should fail - TypeName required after @"
        );
    }

    #[test]
    fn test_relation_type_function() {
        // Test all relation types
        let tokens = tokenize("->").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = relation_type(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "->");

        let tokens = tokenize("<-").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        let result = relation_type(&mut input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "<-");

        // Test failure cases
        let tokens = tokenize("=").expect("Failed to tokenize");
        let mut input = OrreryTokenSlice::new(&tokens);
        assert!(relation_type(&mut input).is_err());
    }

    #[test]
    fn test_standalone_keywords_still_work() {
        // Ensure that standalone keywords are still recognized as keywords
        let test_cases = vec![
            ("component", Token::Component),
            ("type", Token::Type),
            ("diagram", Token::Diagram),
            ("sequence", Token::Sequence),
            ("embed", Token::Embed),
            ("as", Token::As),
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
            type_spec: _type_spec,
        } = element
        {
            assert_eq!(*component.inner(), "user");
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
            type_spec: _type_spec,
        } = element
        {
            assert_eq!(*component.inner(), "user");
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
            type_spec: _type_spec,
        } = element
        {
            assert_eq!(*component.inner(), "user");
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
            type_spec,
        } = element
        {
            assert!(keyword_span.start() < keyword_span.end());
            assert_eq!(
                section.title.as_ref().unwrap().inner(),
                "user authenticated"
            );
            assert_eq!(type_spec.attributes.len(), 0);
            assert_eq!(section.elements.len(), 1);
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
        if let types::Element::OptBlock { type_spec, .. } = element {
            assert_eq!(type_spec.attributes.len(), 2);
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
            assert_eq!(note.type_spec.attributes.len(), 0);
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
            assert_eq!(note.type_spec.attributes.len(), 1);
            assert_eq!(*note.type_spec.attributes[0].name.inner(), "align");
            assert_eq!(note.content.inner(), "Note with attributes");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_note_element_with_named_type() {
        let input = r#"note @WarningNote: "Alert message";"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = note_element(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse note with @TypeName");

        let element = result.unwrap();
        if let types::Element::Note(note) = element {
            assert!(note.type_spec.type_name.is_some());
            assert_eq!(*note.type_spec.type_name.unwrap().inner(), "WarningNote");
            assert!(note.type_spec.attributes.is_empty());
            assert_eq!(note.content.inner(), "Alert message");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_note_element_with_named_type_and_attributes() {
        let input = r#"note @InfoNote[color="blue", size=12]: "Information";"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = note_element(&mut token_slice);
        assert!(result.is_ok(), "Failed to parse note with @TypeName[attrs]");

        let element = result.unwrap();
        if let types::Element::Note(note) = element {
            assert!(note.type_spec.type_name.is_some());
            assert_eq!(*note.type_spec.type_name.unwrap().inner(), "InfoNote");
            assert_eq!(note.type_spec.attributes.len(), 2);
            assert_eq!(*note.type_spec.attributes[0].name.inner(), "color");
            assert_eq!(*note.type_spec.attributes[1].name.inner(), "size");
            assert_eq!(note.content.inner(), "Information");
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
        if let types::Element::Note(note) = &element {
            assert_eq!(note.type_spec.attributes.len(), 1);
            assert_eq!(*note.type_spec.attributes[0].name.inner(), "align");
            assert_eq!(note.content.inner(), "Content with spacing");
        } else {
            panic!("Expected Note element");
        }
    }

    #[test]
    fn test_convert_error_consumed_tokens() {
        // Test Case: Parser consumed tokens before failing
        // Tokens: ["a", "->", "b"]
        // Consumed: all three (indices 0..3)
        // Expected: span covering "a -> b"
        let tokens = vec![
            make_token(Token::Identifier("a"), 0, 1),
            make_token(Token::Arrow_, 2, 2),
            make_token(Token::Identifier("b"), 5, 1),
        ];

        let mut err = ContextError::new();
        err.push(Context::StartOffset(3)); // Started with 3 tokens remaining
        err.push(Context::Label("semicolon"));

        let result = convert_error(
            ErrMode::Cut(err),
            &tokens,
            0, // No tokens remaining
        );

        // Should span from first token to last token
        // Span union: (0..1) union (5..6) = (0..6), which has length 6
        let debug = format!("{:?}", result);
        assert!(debug.contains("unexpected token: expected semicolon"));
    }

    #[test]
    fn test_convert_error_delimiter_at_position() {
        // Test Case: Error at delimiter without consuming tokens
        // Tokens: ["a", ";", "}"]
        // At index 2 ("}"), should point to previous meaningful token
        let tokens = vec![
            make_token(Token::Identifier("a"), 0, 1),
            make_token(Token::Semicolon, 1, 1),
            make_token(Token::RightBrace, 3, 1),
        ];

        let mut err = ContextError::new();
        err.push(Context::StartOffset(1)); // Started at the "}" position
        err.push(Context::Label("component"));

        let result = convert_error(
            ErrMode::Cut(err),
            &tokens,
            1, // One token remaining (the "}")
        );

        // Should point to previous tokens (everything before "}"), which is "a" and ";"
        // Span union: (0..1) union (1..2) = (0..2), which has length 2
        let debug = format!("{:?}", result);
        assert!(debug.contains("start: 0"));
        assert!(debug.contains("end: 2"));
    }

    #[test]
    fn test_convert_error_non_delimiter_at_position() {
        // Test Case: Error at non-delimiter token
        // Tokens: ["diagram", "component", ";", ":"]
        // At index 3 (":"), should point directly at it
        let tokens = vec![
            make_token(Token::Diagram, 0, 7),
            make_token(Token::Whitespace, 7, 1),
            make_token(Token::Component, 8, 9),
            make_token(Token::Semicolon, 17, 1),
            make_token(Token::Colon, 19, 1),
        ];

        let mut err = ContextError::new();
        err.push(Context::StartOffset(1)); // Started at current position

        let result = convert_error(
            ErrMode::Cut(err),
            &tokens,
            1, // One token remaining (the ":")
        );

        // Should point directly at the ":" token
        let debug = format!("{:?}", result);
        assert!(debug.contains("start: 19"));
        assert!(debug.contains("end: 20"));
    }

    #[test]
    fn test_convert_error_eof() {
        // Test Case: Error at EOF
        // Should use last meaningful token
        let tokens = vec![
            make_token(Token::Identifier("a"), 0, 1),
            make_token(Token::Arrow_, 2, 2),
            make_token(Token::Whitespace, 4, 1),
        ];

        let mut err = ContextError::new();
        err.push(Context::StartOffset(0)); // Started at EOF
        err.push(Context::Label("semicolon"));

        let result = convert_error(
            ErrMode::Cut(err),
            &tokens,
            0, // No tokens remaining (EOF)
        );

        // Should span from first meaningful ("a") to last meaningful (arrow)
        // Span union: (0..1) union (2..4) = (0..4), which has length 4
        let debug = format!("{:?}", result);
        assert!(debug.contains("start: 0"));
        assert!(debug.contains("end: 4"));
    }

    #[test]
    fn test_convert_error_skip_whitespace() {
        // Test Case: Consumed range with whitespace
        // Should skip whitespace and union first/last meaningful tokens
        let tokens = vec![
            make_token(Token::Whitespace, 0, 2),
            make_token(Token::Identifier("a"), 2, 1),
            make_token(Token::Whitespace, 3, 1),
            make_token(Token::Arrow_, 4, 2),
            make_token(Token::Whitespace, 6, 1),
            make_token(Token::Identifier("b"), 7, 1),
            make_token(Token::Newline, 8, 1),
        ];

        let mut err = ContextError::new();
        err.push(Context::StartOffset(7)); // Started at beginning
        err.push(Context::Label("semicolon"));

        let result = convert_error(
            ErrMode::Cut(err),
            &tokens,
            0, // Consumed all tokens
        );

        // Should span from first meaningful ("a" at 2) to last meaningful ("b" at 7)
        // Span union: (2..3) union (7..8) = (2..8), which has length 6
        let debug = format!("{:?}", result);
        assert!(debug.contains("start: 2"));
        assert!(debug.contains("end: 8"));
    }

    #[test]
    fn test_convert_error_no_start_offset_fallback() {
        // Test Case: Error without StartOffset context (fallback to 0)
        let tokens = vec![
            make_token(Token::Identifier("a"), 0, 1),
            make_token(Token::Arrow_, 2, 2),
        ];

        let err = ContextError::new(); // No StartOffset!

        let result = convert_error(
            ErrMode::Cut(err),
            &tokens,
            0, // No tokens remaining
        );

        // Should fallback to start_offset=0 and span from beginning
        let debug = format!("{:?}", result);
        assert!(debug.contains("start: 0"));
        assert!(debug.contains("end: 4")); // (0..1) union (2..4) = (0..4)
    }

    #[test]
    fn test_convert_error_multiple_context_labels() {
        // Test Case: Multiple context labels should be joined with " → "
        let tokens = vec![make_token(Token::Identifier("a"), 0, 1)];

        let mut err = ContextError::new();
        err.push(Context::StartOffset(1));
        err.push(Context::Label("semicolon"));
        err.push(Context::Label("component"));

        let result = convert_error(ErrMode::Cut(err), &tokens, 0);

        // Should join labels with arrow separator
        let debug = format!("{:?}", result);
        assert!(debug.contains("expected semicolon → expected component"));
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
                    note.type_spec.attributes.len(),
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
                    note.type_spec
                        .attributes
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
        assert_eq!(*ids[0].inner(), "Component");
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
        assert_eq!(*ids[0].inner(), "client");
        assert_eq!(*ids[1].inner(), "server");
        assert_eq!(*ids[2].inner(), "database");
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
        assert_eq!(*ids[0].inner(), "frontend::app");
        assert_eq!(*ids[1].inner(), "backend::api");
    }

    #[test]
    fn test_identifiers_with_whitespace() {
        let input = r#"[ client , server ]"#;
        let tokens = parse_tokens(input);
        let mut token_slice = TokenSlice::new(&tokens);

        let result = identifiers(&mut token_slice);
        assert!(
            result.is_ok(),
            "Failed to parse identifiers with whitespace"
        );

        let ids = result.unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(*ids[0].inner(), "client");
        assert_eq!(*ids[1].inner(), "server");
    }

    #[test]
    fn test_embedded_diagram_empty() {
        let input = r#"diagram component;
type Service = Rectangle;
auth_service: Service embed diagram sequence {};"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens);
        assert!(
            result.is_ok(),
            "Failed to parse embedded diagram: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_embedded_diagram_with_elements() {
        let input = r#"diagram component;
auth_service: Rectangle embed diagram sequence {
    client: Rectangle;
    server: Rectangle;
    client -> server;
};"#;
        let tokens = parse_tokens(input);
        let result = build_diagram(&tokens);
        assert!(
            result.is_ok(),
            "Failed to parse embedded diagram: {:?}",
            result.err()
        );
    }
}
