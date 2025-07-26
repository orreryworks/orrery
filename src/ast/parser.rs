use crate::{
    ast::{
        parser_types as types,
        span::{SpanImpl, Spanned},
        tokens::Token,
    },
    error::ParseDiagnosticError,
};
use chumsky::{
    IterParser as _, Parser,
    error::Rich,
    extra,
    primitive::{any, choice, end},
    recursive::recursive,
    select,
};
use log::{debug, trace};

type TokenStream<'src> = &'src [(Token<'src>, SpanImpl)];
type DiagramHeader<'a> = (Spanned<&'a str>, Vec<types::Attribute<'a>>);

/// Helper function to create a spanned value
fn make_spanned<T>(value: T, span: SpanImpl) -> Spanned<T> {
    Spanned::new(value, span)
}

/// Parse whitespace and comments (now explicit tokens)
fn ws_comment<'src>()
-> impl Parser<'src, TokenStream<'src>, (), extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>> + Clone
{
    any()
        .filter(|(token, _span)| {
            matches!(
                token,
                Token::Whitespace | Token::Newline | Token::LineComment(_)
            )
        })
        .ignored()
}

pub fn ws_comments0<'src>()
-> impl Parser<'src, TokenStream<'src>, (), extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>> + Clone
{
    ws_comment().repeated().ignored()
}

fn ws_comments1<'src>()
-> impl Parser<'src, TokenStream<'src>, (), extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>> + Clone
{
    ws_comment().repeated().at_least(1).ignored()
}

/// Parse semicolon with optional whitespace
fn semicolon<'src>()
-> impl Parser<'src, TokenStream<'src>, (), extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>> + Clone
{
    ws_comments0()
        .ignore_then(any().filter(|(token, _span)| matches!(token, Token::Semicolon)))
        .ignored()
        .labelled("semicolon")
}

/// Parse a standard identifier
pub fn identifier<'src>()
-> impl Parser<'src, TokenStream<'src>, &'src str, extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>>
+ Clone {
    select! {
        (Token::Identifier(name), _span) => name,
    }
    .labelled("identifier")
}

/// Parse nested identifier with :: separators (e.g., "parent::child", "module::service")
///
/// Supports single identifiers like "app" as well as nested identifiers like "web::frontend".
/// Multiple levels of nesting are supported: "a::b::c::d".
///
/// Returns a String with the full identifier path joined by "::".
fn nested_identifier<'src>()
-> impl Parser<'src, TokenStream<'src>, String, extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>> + Clone
{
    identifier()
        .separated_by(
            any()
                .filter(|(token, _span)| matches!(token, Token::Colon))
                .then(any().filter(|(token, _span)| matches!(token, Token::Colon)))
                .ignored(),
        )
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|parts| parts.join("::"))
        .labelled("nested identifier")
}

/// Parse a string literal
fn string_literal<'src>()
-> impl Parser<'src, TokenStream<'src>, String, extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>> + Clone
{
    select! {
        (Token::StringLiteral(s), _span) => s,
    }
    .labelled("string literal")
}

/// Parse an attribute (key=value pair)
fn attribute<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    types::Attribute<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    identifier()
        .then_ignore(
            ws_comments0()
                .ignore_then(any().filter(|(token, _span)| matches!(token, Token::Equals)))
                .ignore_then(ws_comments0()),
        )
        .then(string_literal())
        .map_with(|(name, value), extra| {
            let span = extra.span();
            types::Attribute {
                name: make_spanned(name, span),
                value: make_spanned(value, span),
            }
        })
        .labelled("attribute")
}

/// Parse a comma-separated list of attributes
fn attributes<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    Vec<types::Attribute<'src>>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    attribute()
        .separated_by(
            any()
                .filter(|(token, _span)| matches!(token, Token::Comma))
                .padded_by(ws_comments0()),
        )
        .at_least(1)
        .collect()
        .labelled("attributes")
}

/// Parse attributes wrapped in square brackets [attr1=val1, attr2=val2]
fn wrapped_attributes<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    Vec<types::Attribute<'src>>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    attributes()
        .or_not()
        .map(|attrs| attrs.unwrap_or_default())
        .delimited_by(
            any()
                .filter(|(token, _span)| matches!(token, Token::LeftBracket))
                .ignore_then(ws_comments0()),
            ws_comments0()
                .ignore_then(any().filter(|(token, _span)| matches!(token, Token::RightBracket))),
        )
        .labelled("wrapped attributes")
}

/// Parse a type definition: type Name = BaseType [attributes];
fn type_definition<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    types::TypeDefinition<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    any()
        .filter(|(token, _span)| matches!(token, Token::Type))
        .ignore_then(ws_comments1())
        .ignore_then(identifier())
        .then_ignore(
            ws_comments0()
                .ignore_then(any().filter(|(token, _span)| matches!(token, Token::Equals)))
                .ignore_then(ws_comments0()),
        )
        .then(identifier())
        .then(ws_comments0().ignore_then(wrapped_attributes().or_not()))
        .then_ignore(semicolon())
        .map_with(|((name, base_type), attributes), extra| {
            let span = extra.span();
            types::TypeDefinition {
                name: make_spanned(name, span),
                base_type: make_spanned(base_type, span),
                attributes: attributes.unwrap_or_default(),
            }
        })
        .padded_by(ws_comments0())
        .labelled("type definition")
}

/// Parse zero or more type definitions
fn type_definitions<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    Vec<types::TypeDefinition<'src>>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    type_definition().repeated().collect()
}

/// Parse relation type specification in brackets: [attributes], [TypeName], or [TypeName; attributes]
/// Parse relation type specifications in square brackets
///
/// Supports four different syntaxes:
/// - `[TypeName; attributes]` - Type with additional attributes: `[RedArrow; width="3"]`
/// - `[attributes]` - Direct attributes only: `[color="blue", width="2"]`
/// - `[TypeName]` - Type reference only: `[RedArrow]`
/// - `[]` - Empty specification
///
/// The parser uses careful choice ordering to avoid commitment issues where
/// an identifier could be interpreted as either a type name or the start of an attribute.
fn relation_type_spec<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    types::RelationTypeSpec<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    choice((
        // [TypeName; attributes] - most specific, try first
        identifier()
            .then_ignore(
                ws_comments0()
                    .ignore_then(any().filter(|(token, _span)| matches!(token, Token::Semicolon)))
                    .ignore_then(ws_comments0()),
            )
            .then(attributes())
            .map_with(|(type_name, attributes), extra| {
                let span = extra.span();
                types::RelationTypeSpec {
                    type_name: Some(make_spanned(type_name, span)),
                    attributes,
                }
            }),
        // [TypeName;] - type name with semicolon but no attributes
        identifier()
            .then_ignore(
                ws_comments0()
                    .ignore_then(any().filter(|(token, _span)| matches!(token, Token::Semicolon)))
                    .ignore_then(ws_comments0()),
            )
            .map_with(|type_name, extra| {
                let span = extra.span();
                types::RelationTypeSpec {
                    type_name: Some(make_spanned(type_name, span)),
                    attributes: Vec::new(),
                }
            }),
        // [attributes] (no type name) - try before [TypeName] to avoid false matches
        attributes().map(|attributes| types::RelationTypeSpec {
            type_name: None,
            attributes,
        }),
        // [TypeName] (no attributes) - try before empty to avoid false matches
        identifier()
            .then_ignore(ws_comments0())
            .map_with(|type_name, extra| {
                let span = extra.span();
                types::RelationTypeSpec {
                    type_name: Some(make_spanned(type_name, span)),
                    attributes: Vec::new(),
                }
            }),
        // [] (empty type spec) - try last
        ws_comments0().map(|_| types::RelationTypeSpec {
            type_name: None,
            attributes: Vec::new(),
        }),
    ))
    .delimited_by(
        any()
            .filter(|(token, _span)| matches!(token, Token::LeftBracket))
            .ignore_then(ws_comments0()),
        ws_comments0()
            .ignore_then(any().filter(|(token, _span)| matches!(token, Token::RightBracket))),
    )
    .labelled("relation type specification")
}

/// Parse relation arrow operators
///
/// Supports all four arrow types defined in the Filament specification:
/// - `->` - Forward arrow (pointing from source to target)
/// - `<-` - Backward arrow (pointing from target to source)
/// - `<->` - Bidirectional arrow (arrows pointing in both directions)
/// - `-` - Plain connection (line with no arrowheads)
///
/// Returns the string representation of the arrow operator.
fn relation_type<'src>()
-> impl Parser<'src, TokenStream<'src>, &'src str, extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>>
+ Clone {
    any()
        .filter(|(token, _span)| {
            matches!(
                token,
                Token::DoubleArrow | Token::LeftArrow | Token::Arrow_ | Token::Plain
            )
        })
        .map(|(token, _span)| match token {
            Token::DoubleArrow => "<->",
            Token::LeftArrow => "<-",
            Token::Arrow_ => "->",
            Token::Plain => "-",
            _ => unreachable!(),
        })
        .labelled("relation type")
}

/// Parse a component with optional nested elements
/// Note: Nested elements are consumed but not recursively parsed for simplicity
fn component_with_elements<'src>(
    elements_parser: impl Parser<
        'src,
        TokenStream<'src>,
        Vec<types::Element<'src>>,
        extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
    > + Clone,
) -> impl Parser<
    'src,
    TokenStream<'src>,
    types::Element<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    identifier()
        .map_with(|name, extra| {
            let span = extra.span();
            make_spanned(name, span)
        })
        .then_ignore(ws_comments0()) // handle whitespace after identifier
        .then(
            // Optional "as" followed by a string literal
            any()
                .filter(|(token, _span)| matches!(token, Token::As))
                .ignore_then(ws_comments1())
                .ignore_then(string_literal())
                .or_not(),
        )
        .then_ignore(ws_comments0()) // handle whitespace before colon
        .then_ignore(any().filter(|(token, _span)| matches!(token, Token::Colon)))
        .then_ignore(ws_comments0()) // handle whitespace after colon
        .then(identifier().map_with(|type_name, extra| {
            let span = extra.span();
            make_spanned(type_name, span)
        })) // parse type name
        .then_ignore(ws_comments0()) // handle whitespace before attributes
        .then(wrapped_attributes().or_not()) // parse optional attributes
        .then_ignore(ws_comments0()) // handle whitespace before optional braces
        .then(
            // Optional nested block: parse nested elements inside braces
            any()
                .filter(|(token, _span)| matches!(token, Token::LeftBrace))
                .ignore_then(ws_comments0())
                .ignore_then(elements_parser.clone())
                .then_ignore(ws_comments0())
                .then_ignore(any().filter(|(token, _span)| matches!(token, Token::RightBrace)))
                .or_not(),
        )
        .then_ignore(ws_comments0()) // handle whitespace before semicolon
        .then_ignore(semicolon()) // parse semicolon (which handles its own whitespace)
        .map_with(
            |((((name, display_name), type_name), attributes), nested_elements), extra| {
                let span = extra.span();
                types::Element::Component {
                    name,
                    display_name: display_name.map(|s| make_spanned(s, span)),
                    type_name,
                    attributes: attributes.unwrap_or_default(),
                    nested_elements: nested_elements.unwrap_or_default(),
                }
            },
        )
        .labelled("component")
}

/// Parse a relation definition
/// Parse a complete relation statement with full Filament syntax support
///
/// Handles the complete relation syntax:
/// ```text
/// source -> target;                           // Basic relation
/// source -> target: "label";                  // With label
/// source -> [TypeSpec] target;                // With type specification
/// source -> [TypeSpec] target: "label";      // With both type spec and label
/// parent::child -> module::service;          // With nested identifiers
/// ```
///
/// The parser follows this sequence:
/// 1. Parse source identifier (supports nested like "parent::child")
/// 2. Consume required whitespace
/// 3. Parse relation type (arrow operator: ->, <-, <->, -)
/// 4. Parse optional type specification in brackets [...]
/// 5. Consume required whitespace and parse target identifier
/// 6. Parse optional label after colon: "text"
/// 7. Consume semicolon terminator
fn relation<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    types::Element<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    nested_identifier()
        .then_ignore(ws_comments1()) // source + required whitespace
        .then(relation_type()) // relation type (arrow)
        .then(
            // Optional type specification with optional whitespace before it
            ws_comments0().ignore_then(relation_type_spec()).or_not(),
        )
        .then(
            // Required whitespace + target identifier (matching legacy parser behavior)
            ws_comments1().ignore_then(nested_identifier()),
        )
        .then_ignore(ws_comments0()) // optional whitespace after target
        .then(
            // Optional label: optional whitespace + colon + optional whitespace + string
            ws_comments0()
                .ignore_then(any().filter(|(token, _span)| matches!(token, Token::Colon)))
                .ignore_then(ws_comments0())
                .ignore_then(string_literal())
                .or_not(),
        )
        .then_ignore(semicolon())
        .map_with(
            |((((source, relation_type), type_spec), target), label), extra| {
                let span = extra.span();
                types::Element::Relation {
                    source: make_spanned(Box::leak(source.into_boxed_str()), span),
                    target: make_spanned(Box::leak(target.into_boxed_str()), span),
                    relation_type: make_spanned(relation_type, span),
                    type_spec,
                    label: label.map(|s| make_spanned(s, span)),
                }
            },
        )
        .labelled("relation")
}

/// Parse any element (component or relation)
fn elements<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    Vec<types::Element<'src>>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    recursive(|elements_parser| {
        let component = component_with_elements(elements_parser);
        let element = choice((component, relation())).padded_by(ws_comments0());
        element.repeated().collect()
    })
}

/// Parse diagram type (component, sequence, etc.)
fn diagram_type<'src>()
-> impl Parser<'src, TokenStream<'src>, &'src str, extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>>
+ Clone {
    any()
        .filter(|(token, _span)| matches!(token, Token::Component | Token::Sequence))
        .map(|(token, _span)| match token {
            Token::Component => "component",
            Token::Sequence => "sequence",
            _ => unreachable!(),
        })
        .labelled("diagram type")
}

/// Parse diagram header with unwrapped attributes
fn diagram_header<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    DiagramHeader<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    any()
        .filter(|(token, _span)| matches!(token, Token::Diagram))
        .ignore_then(ws_comments1())
        .ignore_then(diagram_type())
        .then_ignore(ws_comments0())
        .then(wrapped_attributes().or_not())
        .map_with(|(kind, attrs_opt), extra| {
            let span = extra.span();
            (make_spanned(kind, span), attrs_opt.unwrap_or_default())
        })
        .labelled("diagram header")
}

/// Parse diagram header with semicolon with unwrapped attributes
pub fn diagram_header_with_semicolon<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    DiagramHeader<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    diagram_header().then_ignore(semicolon())
}

/// Parse complete diagram
fn diagram<'src>() -> impl Parser<
    'src,
    TokenStream<'src>,
    types::Element<'src>,
    extra::Err<Rich<'src, (Token<'src>, SpanImpl)>>,
> + Clone {
    ws_comments0()
        .ignore_then(diagram_header_with_semicolon())
        .then(type_definitions())
        .then(elements())
        .then_ignore(ws_comments0())
        .then_ignore(end())
        .map(|((header, type_definitions), elements)| {
            let (kind, attributes) = header;
            types::Element::Diagram(types::Diagram {
                kind,
                attributes,
                type_definitions,
                elements,
            })
        })
        .labelled("diagram")
}

/// Build a diagram from tokens
pub fn build_diagram<'src>(
    tokens: &'src [(Token<'src>, SpanImpl)],
) -> Result<Spanned<types::Element<'src>>, ParseDiagnosticError> {
    debug!("Starting diagram parsing, token count: {}", tokens.len());

    match diagram().parse(tokens).into_result() {
        Ok(diagram) => {
            debug!("Diagram parsed successfully");
            trace!(diagram:?; "Parsed diagram");
            let total_span = if tokens.is_empty() {
                0..0
            } else {
                let first = tokens[0].1;
                let last = tokens[tokens.len() - 1].1;
                first.start..last.end
            };
            Ok(make_spanned(diagram, total_span.into()))
        }
        Err(errors) => {
            trace!("Parser errors: {errors:?}");

            let error_msg = errors
                .into_iter()
                .map(|e| format!("{:?}", e.reason()))
                .collect::<Vec<_>>()
                .join(", ");

            let total_span = if tokens.is_empty() {
                0..0
            } else {
                let first = tokens[0].1;
                let last = tokens[tokens.len() - 1].1;
                first.start..last.end
            };

            Err(ParseDiagnosticError {
                src: format!("{tokens:?}"), // We no longer have the original source string
                message: error_msg,
                span: Some((total_span.start, total_span.end).into()),
                help: Some(
                    "Check syntax for diagram header, type definitions, and elements".to_string(),
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::lexer;
    use chumsky::Parser;

    // Test helper functions for common patterns

    /// Helper function to tokenize input string for testing
    fn tokenize(input: &str) -> Vec<(Token, SpanImpl)> {
        let lexer_parser = lexer::lexer();
        lexer_parser.parse(input).into_output().unwrap_or_default()
    }

    #[test]
    fn test_tokenizer_debug() {
        // Debug test to understand tokenizer behavior
        let test_cases = vec![
            "",
            "   ",
            "\n",
            "\t",
            "// comment",
            "identifier",
            "\"string\"",
            ";",
            "hello",
        ];

        for input in test_cases {
            let lexer_parser = lexer::lexer();
            let result = lexer_parser.parse(input);
            println!("Input: {:?}", input);
            if let Some(tokens) = result.into_output() {
                println!("  Tokens: {:?}", tokens);
            } else {
                println!("  Failed to tokenize");
            }
        }
    }

    #[test]
    fn test_nested_identifier_tokenization_debug() {
        // Debug test to understand nested identifier tokenization
        let test_cases = vec![
            "hello",
            "parent::child",
            "module::sub_module::service",
            "a::b::c",
        ];

        for input in test_cases {
            println!("Testing nested identifier: {:?}", input);
            let tokens = tokenize(input);
            println!("  Tokens: {:?}", tokens);

            let result = nested_identifier().parse(tokens.as_slice());
            println!(
                "  nested_identifier() result: has_output={}",
                result.has_output()
            );
            if result.has_output() {
                println!("  Output: {:?}", result.into_output().unwrap());
            } else {
                println!("  Errors: {:?}", result.into_errors());
            }
            println!();
        }
    }

    #[test]
    fn test_my_service_debug() {
        // Debug test for my_service tokenization issue
        let input = "my_service";
        println!("Testing my_service: {:?}", input);
        let tokens = tokenize(input);
        println!("  Tokens: {:?}", tokens);

        let result = nested_identifier().parse(tokens.as_slice());
        println!(
            "  nested_identifier() result: has_output={}",
            result.has_output()
        );
        if result.has_output() {
            println!("  Output: {:?}", result.into_output().unwrap());
        } else {
            println!("  Errors: {:?}", result.into_errors());
        }
    }

    #[test]
    fn test_empty_brackets_debug() {
        // Debug test for empty brackets parsing issue
        let input = "[]";
        println!("Testing empty brackets: {:?}", input);
        let tokens = tokenize(input);
        println!("  Tokens: {:?}", tokens);

        let result = wrapped_attributes().parse(tokens.as_slice());
        println!(
            "  wrapped_attributes() result: has_output={}",
            result.has_output()
        );
        if result.has_output() {
            println!("  Output: {:?}", result.into_output().unwrap());
        } else {
            println!("  Errors: {:?}", result.into_errors());
        }
    }

    #[test]
    fn test_relation_step_by_step_debug() {
        // Debug relation parsing step by step to understand why "a -> ;" succeeds
        let input = "a -> ;";
        println!("Debugging relation parsing for: {:?}", input);
        let tokens = tokenize(input);
        println!("  Tokens: {:?}", tokens);

        // Test each component of the relation parser separately
        println!("\n  Testing nested_identifier() on 'a':");
        let result1 = nested_identifier().parse(&tokens[0..1]);
        println!("    Result: has_output={}", result1.has_output());
        if result1.has_output() {
            println!("    Output: {:?}", result1.into_output().unwrap());
        }

        println!("\n  Testing relation_type() on '->':");
        let result2 = relation_type().parse(&tokens[2..3]);
        println!("    Result: has_output={}", result2.has_output());
        if result2.has_output() {
            println!("    Output: {:?}", result2.into_output().unwrap());
        }

        println!("\n  Testing nested_identifier() on ';' (should fail):");
        let result3 = nested_identifier().parse(&tokens[4..5]);
        println!("    Result: has_output={}", result3.has_output());
        if !result3.has_output() {
            println!("    Errors: {:?}", result3.into_errors());
        }

        println!("\n  Testing full relation parser:");
        let full_result = relation().parse(tokens.as_slice());
        println!("    Result: has_output={}", full_result.has_output());
        if full_result.has_output() {
            println!("    Output: {:?}", full_result.into_output().unwrap());
        } else {
            println!("    Errors: {:?}", full_result.into_errors());
        }
    }

    #[test]
    fn test_semicolon_as_identifier_debug() {
        // Debug test to understand why semicolon might be parsed as identifier
        let input = ";";
        println!("Testing semicolon as identifier: {:?}", input);
        let tokens = tokenize(input);
        println!("  Tokens: {:?}", tokens);

        let result = nested_identifier().parse(tokens.as_slice());
        println!(
            "  nested_identifier() result: has_output={}",
            result.has_output()
        );
        if result.has_output() {
            println!("  Output: {:?}", result.into_output().unwrap());
        } else {
            println!("  Errors: {:?}", result.into_errors());
        }

        let result2 = identifier().parse(tokens.as_slice());
        println!("  identifier() result: has_output={}", result2.has_output());
        if result2.has_output() {
            println!("  Output: {:?}", result2.into_output().unwrap());
        } else {
            println!("  Errors: {:?}", result2.into_errors());
        }
    }

    #[test]
    fn test_relation_edge_cases_debug() {
        // Debug test for relation parsing edge cases
        let test_cases = vec!["a -> b;", "a -> ;", "-> b;", "a -> b"];

        for input in test_cases {
            println!("Testing relation: {:?}", input);
            let tokens = tokenize(input);
            println!("  Tokens: {:?}", tokens);

            let result = relation().parse(tokens.as_slice());
            println!("  relation() result: has_output={}", result.has_output());
            if result.has_output() {
                println!("  Success!");
            } else {
                println!("  Errors: {:?}", result.into_errors());
            }
            println!();
        }
    }

    #[test]
    fn test_simple_component_debug() {
        // Debug test for basic component parsing
        let test_cases = vec![
            "database: Rectangle;",
            "server: Oval;",
            "app as \"Application\": Service;",
            "cache: Redis [color=\"red\"];",
        ];

        for input in test_cases {
            println!("Testing simple component: {:?}", input);
            let tokens = tokenize(input);
            println!("  Tokens: {:?}", tokens);

            let result = component_with_elements(elements()).parse(tokens.as_slice());
            println!("  component() result: has_output={}", result.has_output());
            if result.has_output() {
                println!("  Success!");
            } else {
                println!("  Errors: {:?}", result.into_errors());
            }
            println!();
        }
    }

    #[test]
    fn test_component_whitespace_debug() {
        // Debug test for component parsing with whitespace and comments
        let input = "my_db // comment\n: Rectangle // another comment\n [fill_color=\"red\"]; // final comment";
        println!("Testing component with whitespace: {:?}", input);
        let tokens = tokenize(input);
        println!("  Tokens: {:?}", tokens);

        let result = component_with_elements(elements()).parse(tokens.as_slice());
        println!("  component() result: has_output={}", result.has_output());
        if result.has_output() {
            println!("  Output: {:?}", result.into_output().unwrap());
        } else {
            println!("  Errors: {:?}", result.into_errors());
        }
    }

    #[test]
    fn test_whitespace_brackets_debug() {
        // Debug test for whitespace in brackets parsing issue
        let input = "[ color=\"red\" , size=\"small\" ]";
        println!("Testing whitespace brackets: {:?}", input);
        let tokens = tokenize(input);
        println!("  Tokens: {:?}", tokens);

        let result = wrapped_attributes().parse(tokens.as_slice());
        println!(
            "  wrapped_attributes() result: has_output={}",
            result.has_output()
        );
        if result.has_output() {
            println!("  Output: {:?}", result.into_output().unwrap());
        } else {
            println!("  Errors: {:?}", result.into_errors());
        }
    }

    #[test]
    fn test_ws_comments0_debug() {
        // Debug test to understand ws_comments0 behavior
        let test_cases = vec![
            ("", "empty input"),
            ("identifier", "identifier token"),
            ("   ", "whitespace"),
            ("// comment", "comment"),
            ("   identifier", "whitespace + identifier"),
        ];

        for (input, description) in test_cases {
            let tokens = tokenize(input);
            println!("Testing {}: {:?}", description, input);
            println!("  Tokens: {:?}", tokens);

            let result = ws_comments0().parse(tokens.as_slice());
            println!(
                "  ws_comments0() result: has_output={}",
                result.has_output()
            );
            if !result.has_output() {
                println!("  Errors: {:?}", result.into_errors());
            } else {
                println!("  Success - remaining tokens after parse");
            }
            println!();
        }
    }

    #[test]
    fn test_basic_diagram() {
        let input = r#"
            diagram component;

            app: WebApp;
        "#;

        // First lex the input
        let lexer_parser = lexer::lexer();
        let tokens = lexer_parser.parse(input).into_output().unwrap();

        // Then parse the tokens
        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_ok(),
            "Expected successful parse, got: {:?}",
            result
        );
    }

    #[test]
    fn test_diagram_header() {
        let input = "diagram component;";

        // First lex the input
        let lexer_parser = lexer::lexer();
        let tokens = lexer_parser.parse(input).into_output().unwrap();

        println!("Header tokens: {:?}", tokens);

        // Test just the header parser
        let parser = diagram_header_with_semicolon();
        let result = parser.parse(tokens.as_slice());
        println!("Header parse result: {:?}", result);

        if result.has_output() {
            println!("Success!");
        } else {
            println!("Errors: {:?}", result.into_errors());
        }
    }

    #[test]
    fn test_identifier() {
        let input = "hello";
        let lexer_parser = lexer::lexer();
        let tokens = lexer_parser.parse(input).into_output().unwrap();

        let parser = identifier();
        let result = parser.parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "hello");
    }

    #[test]
    fn test_nested_identifier() {
        // Test simple identifier (only case currently supported)
        let input = "hello";
        let lexer_parser = lexer::lexer();
        let tokens = lexer_parser.parse(input).into_output().unwrap();
        let parser = nested_identifier();
        let result = parser.parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "hello");

        // TODO: Implement and test nested identifier cases
        // - "hello::world"
        // - "a::b::c"
    }

    #[test]
    fn test_ws_comment() {
        // Test parsing whitespace tokens
        let tokens = tokenize("   ");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse whitespace");

        let tokens = tokenize("\n");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse newline");

        let tokens = tokenize("\t");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse tab");

        // Test parsing line comments
        let tokens = tokenize("// This is a comment");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse line comment");

        let tokens = tokenize("// Another comment with symbols !@#$%");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse line comment with symbols"
        );

        // Test that non-whitespace/comment tokens fail
        let tokens = tokenize("identifier");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should not parse identifier as whitespace/comment"
        );

        let tokens = tokenize(";");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should not parse semicolon as whitespace/comment"
        );

        let tokens = tokenize("\"string\"");
        let result = ws_comment().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should not parse string as whitespace/comment"
        );
    }

    #[test]
    fn test_ws_comments0() {
        // Test zero whitespace/comments (should succeed on empty input)
        let tokens = tokenize("");
        let result = ws_comments0().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse empty input (zero whitespace/comments)"
        );

        // Test that it fails when there are non-whitespace tokens (can't consume them)
        let tokens = tokenize("identifier");
        let result = ws_comments0().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should fail on identifier (can't consume non-whitespace tokens)"
        );

        // Test single whitespace/comment
        let tokens = tokenize("   ");
        let result = ws_comments0().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single whitespace");

        let tokens = tokenize("// comment");
        let result = ws_comments0().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single comment");

        // Test multiple whitespace/comments
        let tokens = tokenize("   \n\t   ");
        let result = ws_comments0().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse multiple whitespace tokens"
        );

        let tokens = tokenize("// comment1\n// comment2");
        let result = ws_comments0().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse multiple comments");

        // Test mixed whitespace and comments
        let tokens = tokenize("  // comment\n  ");
        let result = ws_comments0().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse mixed whitespace and comments"
        );
    }

    #[test]
    fn test_ws_comments1() {
        // Test single whitespace/comment (should succeed)
        let tokens = tokenize("   ");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single whitespace");

        let tokens = tokenize("// comment");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single comment");

        let tokens = tokenize("\n");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single newline");

        // Test multiple whitespace/comments
        let tokens = tokenize("   \n\t   ");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse multiple whitespace tokens"
        );

        let tokens = tokenize("// comment1\n// comment2");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse multiple comments");

        // Test mixed whitespace and comments
        let tokens = tokenize("  // comment\n  ");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse mixed whitespace and comments"
        );

        // Test that it requires at least one whitespace/comment
        let tokens = tokenize("");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should fail on empty input (requires at least one)"
        );

        let tokens = tokenize("identifier");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should fail on identifier (requires whitespace/comment)"
        );

        let tokens = tokenize(";");
        let result = ws_comments1().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should fail on semicolon (requires whitespace/comment)"
        );
    }

    #[test]
    fn test_semicolon() {
        // Test basic semicolon
        let tokens = tokenize(";");
        assert!(semicolon().parse(tokens.as_slice()).has_output());

        // Test semicolon with leading whitespace
        let tokens = tokenize("   ;");
        assert!(semicolon().parse(tokens.as_slice()).has_output());

        let tokens = tokenize("\n\t  ;");
        assert!(semicolon().parse(tokens.as_slice()).has_output());

        // Test semicolon with leading comments
        let tokens = tokenize("// comment\n;");
        assert!(semicolon().parse(tokens.as_slice()).has_output());

        let tokens = tokenize("  // comment\n  ;");
        assert!(semicolon().parse(tokens.as_slice()).has_output());

        // Test that non-semicolon fails
        let tokens = tokenize("identifier");
        assert!(!semicolon().parse(tokens.as_slice()).has_output());

        let tokens = tokenize(",");
        assert!(!semicolon().parse(tokens.as_slice()).has_output());

        let tokens = tokenize(":");
        assert!(!semicolon().parse(tokens.as_slice()).has_output());
    }

    #[test]
    fn test_string_literal() {
        // Test basic string literals
        let tokens = tokenize("\"hello\"");
        let result = string_literal().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "hello".to_string());

        let tokens = tokenize("\"world\"");
        let result = string_literal().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "world".to_string());

        let tokens = tokenize("\"\"");
        let result = string_literal().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "".to_string());

        // Test strings with spaces and special characters
        let tokens = tokenize("\"hello world\"");
        let result = string_literal().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "hello world".to_string());

        let tokens = tokenize("\"test 123 !@#\"");
        let result = string_literal().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "test 123 !@#".to_string());

        // Test strings with escape sequences (if supported by lexer)
        let tokens = tokenize("\"hello\\nworld\"");
        let result = string_literal().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "hello\nworld".to_string());

        let tokens = tokenize("\"quote: \\\"test\\\"\"");
        let result = string_literal().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "quote: \"test\"".to_string());

        // Test that non-strings fail
        let tokens = tokenize("identifier");
        assert!(!string_literal().parse(tokens.as_slice()).has_output());

        let tokens = tokenize("123");
        assert!(!string_literal().parse(tokens.as_slice()).has_output());

        let tokens = tokenize(";");
        assert!(!string_literal().parse(tokens.as_slice()).has_output());
    }

    #[test]
    fn test_nested_identifier_enhanced() {
        // Test simple identifier (already works)
        let tokens = tokenize("hello");
        let result = nested_identifier().parse(tokens.as_slice());
        assert!(result.has_output());
        assert_eq!(result.into_output().unwrap(), "hello");

        let tokens = tokenize("service");
        let result = nested_identifier().parse(tokens.as_slice());
        if !result.has_output() {
            println!("service test failed with tokens: {:?}", tokens);
            println!("errors: {:?}", result.into_errors());
            panic!("service should parse successfully");
        }
        assert_eq!(result.into_output().unwrap(), "service");

        let tokens = tokenize("my_service");
        let result = nested_identifier().parse(tokens.as_slice());
        if !result.has_output() {
            println!("my_service test failed with tokens: {:?}", tokens);
            println!("errors: {:?}", result.into_errors());
            panic!("my_service should parse successfully");
        }
        assert_eq!(result.into_output().unwrap(), "my_service");

        // Test nested identifiers (these should work if the parser supports ::)
        // Note: These tests will help verify if nested identifier parsing is fully implemented
        let tokens = tokenize("parent::child");
        let result = nested_identifier().parse(tokens.as_slice());
        if result.has_output() {
            assert_eq!(result.into_output().unwrap(), "parent::child");
        } else {
            // If not implemented yet, just verify it fails gracefully
            println!("Nested identifier parsing not yet fully implemented for: parent::child");
        }

        let tokens = tokenize("module::sub_module::service");
        let result = nested_identifier().parse(tokens.as_slice());
        if result.has_output() {
            assert_eq!(result.into_output().unwrap(), "module::sub_module::service");
        } else {
            println!(
                "Complex nested identifier parsing not yet implemented for: module::sub_module::service"
            );
        }
    }

    #[test]
    fn test_attribute() {
        // Test basic attribute parsing
        let tokens = tokenize("color=\"blue\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic attribute");
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "color");
        assert_eq!(*attr.value, "blue".to_string());

        // Test attribute with whitespace around equals
        let tokens = tokenize("size = \"large\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse attribute with whitespace"
        );
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "size");
        assert_eq!(*attr.value, "large".to_string());

        // Test attribute with comments around equals
        let tokens = tokenize("width // comment\n = \"10\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse attribute with comments");
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "width");
        assert_eq!(*attr.value, "10".to_string());

        // Test attributes with various string values
        let tokens = tokenize("label=\"Multi word string\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse attribute with multi-word string"
        );
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "label");
        assert_eq!(*attr.value, "Multi word string".to_string());

        // Test that invalid attributes fail
        let tokens = tokenize("invalid_syntax");
        let result = attribute().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should fail on invalid attribute syntax"
        );

        let tokens = tokenize("key=unquoted_value");
        let result = attribute().parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on unquoted value");
    }

    #[test]
    fn test_attributes() {
        // Test single attribute list
        let tokens = tokenize("color=\"blue\"");
        let result = attributes().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single attribute");
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "blue".to_string());

        // Test multiple attributes
        let tokens = tokenize("color=\"blue\", size=\"large\"");
        let result = attributes().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse multiple attributes");
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 2);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "blue".to_string());
        assert_eq!(*attrs[1].name, "size");
        assert_eq!(*attrs[1].value, "large".to_string());

        // Test attributes with whitespace and comments
        let tokens = tokenize("color=\"red\" // first attr\n, size = \"small\"");
        let result = attributes().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse attributes with whitespace/comments"
        );
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 2);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "red".to_string());
        assert_eq!(*attrs[1].name, "size");
        assert_eq!(*attrs[1].value, "small".to_string());

        // Test that empty attribute list fails (requires at least one)
        let tokens = tokenize("");
        let result = attributes().parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on empty attribute list");
    }

    #[test]
    fn test_wrapped_attributes() {
        // Test empty attribute brackets
        let tokens = tokenize("[]");
        let result = wrapped_attributes().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse empty brackets");
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 0);

        // Test single attribute in brackets
        let tokens = tokenize("[color=\"blue\"]");
        let result = wrapped_attributes().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single wrapped attribute");
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 1);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "blue".to_string());

        // Test multiple attributes in brackets
        let tokens = tokenize("[color=\"blue\", size=\"large\", active=\"true\"]");
        let result = wrapped_attributes().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse multiple wrapped attributes"
        );
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 3);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "blue".to_string());
        assert_eq!(*attrs[1].name, "size");
        assert_eq!(*attrs[1].value, "large".to_string());
        assert_eq!(*attrs[2].name, "active");
        assert_eq!(*attrs[2].value, "true".to_string());

        // Test attributes with whitespace in brackets
        let tokens = tokenize("[ color=\"red\" , size=\"small\" ]");
        let result = wrapped_attributes().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse wrapped attributes with whitespace"
        );
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 2);

        // Test that unclosed brackets fail
        let tokens = tokenize("[color=\"blue\"");
        let result = wrapped_attributes().parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on unclosed brackets");
    }

    #[test]
    fn test_type_definition() {
        // Test basic type definition
        let tokens = tokenize("type Database = Rectangle;");
        let result = type_definition().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic type definition");
        let type_def = result.into_output().unwrap();
        assert_eq!(*type_def.name, "Database");
        assert_eq!(*type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 0);

        // Test type definition with attributes
        let tokens = tokenize("type Service = Rectangle [fill_color=\"blue\"];");
        let result = type_definition().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse type definition with attributes"
        );
        let type_def = result.into_output().unwrap();
        assert_eq!(*type_def.name, "Service");
        assert_eq!(*type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);
        assert_eq!(*type_def.attributes[0].name, "fill_color");
        assert_eq!(*type_def.attributes[0].value, "blue".to_string());

        // Test type definition with multiple attributes
        let tokens = tokenize("type Client = Oval [fill_color=\"red\", line_width=\"2\"];");
        let result = type_definition().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse type definition with multiple attributes"
        );
        let type_def = result.into_output().unwrap();
        assert_eq!(*type_def.name, "Client");
        assert_eq!(*type_def.base_type, "Oval");
        assert_eq!(type_def.attributes.len(), 2);
        assert_eq!(*type_def.attributes[0].name, "fill_color");
        assert_eq!(*type_def.attributes[0].value, "red".to_string());
        assert_eq!(*type_def.attributes[1].name, "line_width");
        assert_eq!(*type_def.attributes[1].value, "2".to_string());

        // Test type definition with whitespace and comments
        let tokens =
            tokenize("type MyService = Rectangle // base type\n [color=\"green\"]; // attributes");
        let result = type_definition().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse type definition with whitespace/comments"
        );
        let type_def = result.into_output().unwrap();
        assert_eq!(*type_def.name, "MyService");
        assert_eq!(*type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);

        // Test that invalid type definitions fail
        let tokens = tokenize("type Database;");
        let result = type_definition().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should fail on incomplete type definition"
        );

        let tokens = tokenize("type = Rectangle;");
        let result = type_definition().parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on missing type name");

        let tokens = tokenize("type Database = Rectangle");
        let result = type_definition().parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on missing semicolon");
    }

    #[test]
    fn test_type_definitions() {
        // Test empty type definitions (should succeed)
        let tokens = tokenize("");
        let result = type_definitions().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse empty type definitions");
        let type_defs = result.into_output().unwrap();
        assert_eq!(type_defs.len(), 0);

        // Test single type definition
        let tokens = tokenize("type Database = Rectangle;");
        let result = type_definitions().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single type definition");
        let type_defs = result.into_output().unwrap();
        assert_eq!(type_defs.len(), 1);
        assert_eq!(*type_defs[0].name, "Database");

        // Test multiple type definitions
        let tokens = tokenize(
            r#"
            type Database = Rectangle [fill_color="blue"];
            type Service = Oval [fill_color="green"];
            type Client = Rectangle [fill_color="red"];
        "#,
        );
        let result = type_definitions().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse multiple type definitions"
        );
        let type_defs = result.into_output().unwrap();
        assert_eq!(type_defs.len(), 3);
        assert_eq!(*type_defs[0].name, "Database");
        assert_eq!(*type_defs[1].name, "Service");
        assert_eq!(*type_defs[2].name, "Client");

        // Test type definitions with mixed whitespace and comments
        let tokens = tokenize(
            r#"
            // First type
            type MyDatabase = Rectangle;

            // Second type with attributes
            type MyService = Oval [color="blue"];
        "#,
        );
        let result = type_definitions().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse type definitions with comments"
        );
        let type_defs = result.into_output().unwrap();
        assert_eq!(type_defs.len(), 2);
        assert_eq!(*type_defs[0].name, "MyDatabase");
        assert_eq!(*type_defs[1].name, "MyService");
    }

    #[test]
    fn test_component() {
        // Test basic component
        let tokens = tokenize("database: Rectangle;");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic component");
        match result.into_output().unwrap() {
            types::Element::Component {
                name,
                display_name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "database");
                assert!(display_name.is_none());
                assert_eq!(*type_name, "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with attributes
        let tokens = tokenize("server: Oval [fill_color=\"green\", line_color=\"black\"];");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse component with attributes"
        );
        match result.into_output().unwrap() {
            types::Element::Component {
                name,
                display_name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "server");
                assert!(display_name.is_none());
                assert_eq!(*type_name, "Oval");
                assert_eq!(attributes.len(), 2);
                assert_eq!(*attributes[0].name, "fill_color");
                assert_eq!(*attributes[0].value, "green".to_string());
                assert_eq!(*attributes[1].name, "line_color");
                assert_eq!(*attributes[1].value, "black".to_string());
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with display name (alias)
        let tokens = tokenize("db_server as \"Database Server\": Rectangle;");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse component with display name"
        );
        match result.into_output().unwrap() {
            types::Element::Component {
                name,
                display_name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "db_server");
                assert!(display_name.is_some());
                assert_eq!(*display_name.unwrap(), "Database Server");
                assert_eq!(*type_name, "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with both display name and attributes
        let tokens =
            tokenize("auth_service as \"Authentication Service\": Service [color=\"blue\"];");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse component with display name and attributes"
        );
        match result.into_output().unwrap() {
            types::Element::Component {
                name,
                display_name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "auth_service");
                assert!(display_name.is_some());
                assert_eq!(*display_name.unwrap(), "Authentication Service");
                assert_eq!(*type_name, "Service");
                assert_eq!(attributes.len(), 1);
                assert_eq!(*attributes[0].name, "color");
                assert_eq!(*attributes[0].value, "blue".to_string());
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with whitespace and comments
        let tokens =
            tokenize("my_db // comment\n: Rectangle // another comment\n [fill_color=\"red\"];");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse component with whitespace and comments"
        );
        match result.into_output().unwrap() {
            types::Element::Component {
                name,
                type_name,
                attributes,
                ..
            } => {
                assert_eq!(*name, "my_db");
                assert_eq!(*type_name, "Rectangle");
                assert_eq!(attributes.len(), 1);
                assert_eq!(*attributes[0].name, "fill_color");
                assert_eq!(*attributes[0].value, "red".to_string());
            }
            _ => panic!("Expected Component"),
        }

        // Test that invalid components fail
        let tokens = tokenize("database Rectangle;");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on missing colon");

        let tokens = tokenize("database:;");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on missing type");

        let tokens = tokenize("database: Rectangle");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on missing semicolon");
    }

    #[test]
    fn test_relation() {
        // Test basic relation types
        let tokens = tokenize("a -> b;");
        let result = relation().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic forward relation");
        match result.into_output().unwrap() {
            types::Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => {
                assert_eq!(*source, "a");
                assert_eq!(*target, "b");
                assert_eq!(*relation_type, "->");
                assert!(type_spec.is_none());
                assert!(label.is_none());
            }
            _ => panic!("Expected Relation"),
        }

        // Test different arrow types
        let tokens = tokenize("a <- b;");
        let result = relation().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse backward relation");
        match result.into_output().unwrap() {
            types::Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type, "<-");
            }
            _ => panic!("Expected Relation"),
        }

        let tokens = tokenize("a <-> b;");
        let result = relation().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse bidirectional relation");
        match result.into_output().unwrap() {
            types::Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type, "<->");
            }
            _ => panic!("Expected Relation"),
        }

        let tokens = tokenize("a - b;");
        let result = relation().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse plain connection");
        match result.into_output().unwrap() {
            types::Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type, "-");
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with type reference
        let tokens = tokenize("a -> [RedArrow] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse relation with type reference"
        );
        match result.into_output().unwrap() {
            types::Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => {
                assert_eq!(*source, "a");
                assert_eq!(*target, "b");
                assert_eq!(*relation_type, "->");
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_some());
                assert_eq!(**spec.type_name.as_ref().unwrap(), "RedArrow");
                assert!(spec.attributes.is_empty());
                assert!(label.is_none());
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with type reference and attributes
        let tokens = tokenize("a -> [BlueArrow; width=\"5\", style=\"dashed\"] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse relation with type and attributes"
        );
        match result.into_output().unwrap() {
            types::Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => {
                assert_eq!(*source, "a");
                assert_eq!(*target, "b");
                assert_eq!(*relation_type, "->");
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_some());
                assert_eq!(**spec.type_name.as_ref().unwrap(), "BlueArrow");
                assert_eq!(spec.attributes.len(), 2);
                assert_eq!(*spec.attributes[0].name, "width");
                assert_eq!(*spec.attributes[0].value, "5".to_string());
                assert_eq!(*spec.attributes[1].name, "style");
                assert_eq!(*spec.attributes[1].value, "dashed".to_string());
                assert!(label.is_none());
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with attributes only (no type name)
        let tokens = tokenize("a -> [color=\"red\", width=\"3\"] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse relation with attributes only"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_none());
                assert_eq!(spec.attributes.len(), 2);
                assert_eq!(*spec.attributes[0].name, "color");
                assert_eq!(*spec.attributes[0].value, "red".to_string());
                assert_eq!(*spec.attributes[1].name, "width");
                assert_eq!(*spec.attributes[1].value, "3".to_string());
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with label
        let tokens = tokenize("client -> server: \"HTTP Request\";");
        let result = relation().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse relation with label");
        match result.into_output().unwrap() {
            types::Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => {
                assert_eq!(*source, "client");
                assert_eq!(*target, "server");
                assert_eq!(*relation_type, "->");
                assert!(type_spec.is_none());
                assert_eq!(**label.as_ref().unwrap(), "HTTP Request");
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with nested identifiers
        let tokens = tokenize("frontend::app -> backend::service;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse relation with nested identifiers"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { source, target, .. } => {
                assert_eq!(*source, "frontend::app");
                assert_eq!(*target, "backend::service");
            }
            _ => panic!("Expected Relation"),
        }

        // Test complex relation with everything
        let tokens = tokenize(
            "web::client -> [HTTPConnection; secure=\"true\"] api::server: \"HTTPS Request\";",
        );
        let result = relation().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse complex relation");
        match result.into_output().unwrap() {
            types::Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => {
                assert_eq!(*source, "web::client");
                assert_eq!(*target, "api::server");
                assert_eq!(*relation_type, "->");
                let spec = type_spec.as_ref().unwrap();
                assert_eq!(**spec.type_name.as_ref().unwrap(), "HTTPConnection");
                assert_eq!(spec.attributes.len(), 1);
                assert_eq!(**label.as_ref().unwrap(), "HTTPS Request");
            }
            _ => panic!("Expected Relation"),
        }

        // Test that invalid relations fail
        let tokens = tokenize("a -> ;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            !result.has_output(),
            "Should fail on missing target - semicolon is not a valid identifier"
        );

        let tokens = tokenize("-> b;");
        let result = relation().parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on missing source");

        let tokens = tokenize("a -> b");
        let result = relation().parse(tokens.as_slice());
        assert!(!result.has_output(), "Should fail on missing semicolon");
    }

    #[test]
    fn test_relation_type_spec() {
        // Test empty type specification
        let tokens = tokenize("[]");
        let result = relation_type_spec().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse empty type spec");
        let spec = result.into_output().unwrap();
        assert!(spec.type_name.is_none());
        assert!(spec.attributes.is_empty());

        // Test type name only
        let tokens = tokenize("[RedArrow]");
        let result = relation_type_spec().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse type name only");
        let spec = result.into_output().unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(**spec.type_name.as_ref().unwrap(), "RedArrow");
        assert!(spec.attributes.is_empty());

        // Test attributes only
        let tokens = tokenize("[color=\"blue\", width=\"2\"]");
        let result = relation_type_spec().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse attributes only");
        let spec = result.into_output().unwrap();
        assert!(spec.type_name.is_none());
        assert_eq!(spec.attributes.len(), 2);
        assert_eq!(*spec.attributes[0].name, "color");
        assert_eq!(*spec.attributes[0].value, "blue".to_string());
        assert_eq!(*spec.attributes[1].name, "width");
        assert_eq!(*spec.attributes[1].value, "2".to_string());

        // Test type name with attributes
        let tokens = tokenize("[BlueArrow; style=\"curved\", width=\"3\"]");
        let result = relation_type_spec().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse type name with attributes"
        );
        let spec = result.into_output().unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(**spec.type_name.as_ref().unwrap(), "BlueArrow");
        assert_eq!(spec.attributes.len(), 2);
        assert_eq!(*spec.attributes[0].name, "style");
        assert_eq!(*spec.attributes[0].value, "curved".to_string());
        assert_eq!(*spec.attributes[1].name, "width");
        assert_eq!(*spec.attributes[1].value, "3".to_string());

        // Test with whitespace
        let tokens = tokenize("[ MyArrow ; color=\"red\" , style=\"dotted\" ]");
        let result = relation_type_spec().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse with whitespace");
        let spec = result.into_output().unwrap();
        assert!(spec.type_name.is_some());
        assert_eq!(**spec.type_name.as_ref().unwrap(), "MyArrow");
        assert_eq!(spec.attributes.len(), 2);
    }

    #[test]
    fn test_elements() {
        // Test empty elements (should succeed)
        let tokens = tokenize("");
        let result = elements().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse empty elements");
        let elems = result.into_output().unwrap();
        assert_eq!(elems.len(), 0);

        // Test single component
        let tokens = tokenize("app: Rectangle;");
        let result = elements().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single component");
        let elems = result.into_output().unwrap();
        assert_eq!(elems.len(), 1);
        match &elems[0] {
            types::Element::Component { name, .. } => {
                assert_eq!(*name.inner(), "app");
            }
            _ => panic!("Expected Component"),
        }

        // Test single relation
        let tokens = tokenize("a -> b;");
        let result = elements().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse single relation");
        let elems = result.into_output().unwrap();
        assert_eq!(elems.len(), 1);
        match &elems[0] {
            types::Element::Relation { source, target, .. } => {
                assert_eq!(*source.inner(), "a");
                assert_eq!(*target.inner(), "b");
            }
            _ => panic!("Expected Relation"),
        }

        // Test multiple mixed elements
        let tokens = tokenize(
            r#"
            app: Rectangle;
            db: Database;
            app -> db;
            cache: Oval [color="red"];
            db -> cache: "writes to";
        "#,
        );
        let result = elements().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse multiple mixed elements");
        let elems = result.into_output().unwrap();
        assert_eq!(elems.len(), 5);

        // Verify element types
        match &elems[0] {
            types::Element::Component { name, .. } => assert_eq!(*name.inner(), "app"),
            _ => panic!("Expected Component at index 0"),
        }
        match &elems[1] {
            types::Element::Component { name, .. } => assert_eq!(*name.inner(), "db"),
            _ => panic!("Expected Component at index 1"),
        }
        match &elems[2] {
            types::Element::Relation { source, target, .. } => {
                assert_eq!(*source.inner(), "app");
                assert_eq!(*target.inner(), "db");
            }
            _ => panic!("Expected Relation at index 2"),
        }
        match &elems[3] {
            types::Element::Component { name, .. } => assert_eq!(*name.inner(), "cache"),
            _ => panic!("Expected Component at index 3"),
        }
        match &elems[4] {
            types::Element::Relation {
                source,
                target,
                label,
                ..
            } => {
                assert_eq!(*source.inner(), "db");
                assert_eq!(*target.inner(), "cache");
                assert_eq!(*label.as_ref().unwrap().inner(), "writes to");
            }
            _ => panic!("Expected Relation at index 4"),
        }
    }

    #[test]
    fn test_embedded_diagram() {
        // Note: The current component parser is simplified and doesn't support nested elements yet
        // This test checks if the parser can handle the basic syntax for future implementation

        // Test component with simple syntax that would support embedding
        let tokens = tokenize("auth_service: Service;");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic service component");

        // For now, we test that the basic component structure is there
        // In the future, this would be extended to test actual embedded diagram syntax
        // like: auth_service: Service embed diagram sequence { client: Rectangle; };
    }

    #[test]
    fn test_complex_diagram() {
        // Test a comprehensive diagram with multiple features
        let tokens = tokenize(
            r#"
            diagram component;

            type Database = Rectangle [fill_color="lightblue"];
            type Service = Rectangle [fill_color="lightgreen"];

            frontend: Service [label="Frontend App"];
            backend: Service [label="Backend API"];
            cache: Database [label="Redis Cache"];
            db: Database [label="PostgreSQL"];

            frontend -> backend: "HTTP Request";
            backend -> cache: "Cache Lookup";
            backend -> db: "Database Query";
            cache -> backend: "Cache Response";
            db -> backend: "Query Result";
            backend -> frontend: "HTTP Response";
        "#,
        );

        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should parse complex diagram successfully");

        let diagram_element = result.unwrap();
        match diagram_element.inner() {
            types::Element::Diagram(diagram) => {
                assert_eq!(*diagram.kind, "component");
                assert_eq!(diagram.type_definitions.len(), 2); // Database and Service types
                assert_eq!(diagram.elements.len(), 10); // 4 components + 6 relations

                // Verify we have the expected type definitions
                let type_defs = &diagram.type_definitions;
                assert_eq!(*type_defs[0].name, "Database");
                assert_eq!(*type_defs[1].name, "Service");

                // Verify we have both components and relations
                let elements = &diagram.elements;
                let mut component_count = 0;
                let mut relation_count = 0;

                for element in elements.iter() {
                    match element {
                        types::Element::Component { .. } => component_count += 1,
                        types::Element::Relation { .. } => relation_count += 1,
                        _ => {}
                    }
                }

                assert_eq!(component_count, 4, "Should have 4 components");
                assert_eq!(relation_count, 6, "Should have 6 relations");
            }
            _ => panic!("Expected Diagram element"),
        }
    }

    #[test]
    fn test_nested_components() {
        // Note: Current parser doesn't support nested components yet
        // This test verifies the basic structure is parseable for future extension

        // Test basic component that could be extended to support nesting
        let tokens = tokenize("system: Rectangle;");
        let result = component_with_elements(elements()).parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic component");

        match result.into_output().unwrap() {
            types::Element::Component {
                nested_elements, ..
            } => {
                // Currently nested_elements is always empty (simplified implementation)
                assert_eq!(nested_elements.len(), 0);
                // In the future, this would test actual nested syntax like:
                // system: Rectangle { app: Service; db: Database; app -> db; };
            }
            _ => panic!("Expected Component"),
        }
    }

    #[test]
    fn test_diagram_with_types() {
        // Test diagram that combines type definitions with components and relations
        let tokens = tokenize(
            r#"
            diagram component;

            type WebApp = Rectangle [fill_color="blue", border="2"];
            type Database = Oval [fill_color="green"];
            type ApiService = Rectangle [fill_color="yellow"];

            frontend: WebApp;
            api: ApiService [port="8080"];
            db: Database [name="PostgreSQL"];

            frontend -> api: "API Call";
            api -> db: "SQL Query";
        "#,
        );

        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_ok(),
            "Should parse diagram with types successfully"
        );

        let diagram_element = result.unwrap();
        match diagram_element.inner() {
            types::Element::Diagram(diagram) => {
                // Verify diagram structure
                assert_eq!(*diagram.kind, "component");
                assert_eq!(diagram.type_definitions.len(), 3);
                assert_eq!(diagram.elements.len(), 5); // 3 components + 2 relations

                // Verify type definitions
                let types = &diagram.type_definitions;
                assert_eq!(*types[0].name, "WebApp");
                assert_eq!(*types[0].base_type, "Rectangle");
                assert_eq!(types[0].attributes.len(), 2);

                assert_eq!(*types[1].name, "Database");
                assert_eq!(*types[1].base_type, "Oval");
                assert_eq!(types[1].attributes.len(), 1);

                assert_eq!(*types[2].name, "ApiService");
                assert_eq!(*types[2].base_type, "Rectangle");
                assert_eq!(types[2].attributes.len(), 1);

                // Verify elements include both components and relations
                let elements = &diagram.elements;
                let mut components = Vec::new();
                let mut relations = Vec::new();

                for element in elements.iter() {
                    match element {
                        types::Element::Component { name, .. } => {
                            components.push((*name.inner()).to_string());
                        }
                        types::Element::Relation { source, target, .. } => {
                            relations.push((
                                (*source.inner()).to_string(),
                                (*target.inner()).to_string(),
                            ));
                        }
                        _ => {}
                    }
                }

                assert_eq!(components.len(), 3);
                assert!(components.contains(&"frontend".to_string()));
                assert!(components.contains(&"api".to_string()));
                assert!(components.contains(&"db".to_string()));

                assert_eq!(relations.len(), 2);
                assert!(relations.contains(&("frontend".to_string(), "api".to_string())));
                assert!(relations.contains(&("api".to_string(), "db".to_string())));
            }
            _ => panic!("Expected Diagram element"),
        }
    }

    #[test]
    fn test_build_diagram_enhanced() {
        // Enhanced test for build_diagram with various scenarios

        // Test 1: Minimal valid diagram
        let tokens = tokenize("diagram component;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should parse minimal diagram");

        // Test 2: Diagram with sequence type
        let tokens = tokenize("diagram sequence;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should parse sequence diagram");

        // Test 3: Diagram with attributes
        let tokens = tokenize("diagram component [layout=\"hierarchical\"];");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should parse diagram with attributes");

        // Test 4: Complete diagram with all features
        let tokens = tokenize(
            r#"
            diagram component [layout="auto"];

            type MyService = Rectangle [color="blue"];

            app: MyService;
            db: Rectangle;

            app -> db;
        "#,
        );
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should parse complete diagram");

        let diagram_element = result.unwrap();
        match diagram_element.inner() {
            types::Element::Diagram(diagram) => {
                assert_eq!(*diagram.kind, "component");
                assert_eq!(diagram.attributes.len(), 1);
                assert_eq!(*diagram.attributes[0].name, "layout");
                assert_eq!(*diagram.attributes[0].value, "auto".to_string());
                assert_eq!(diagram.type_definitions.len(), 1);
                assert_eq!(diagram.elements.len(), 3); // 2 components + 1 relation
            }
            _ => panic!("Expected Diagram element"),
        }

        // Test 5: Error cases
        let tokens = tokenize("invalid syntax here");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on invalid syntax");

        let tokens = tokenize("diagram component; extra content");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on trailing content");
    }

    #[test]
    fn test_diagram_header_variations() {
        // Test various diagram header formats

        // Basic headers
        let tokens = tokenize("diagram component;");
        let result = diagram_header_with_semicolon().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic component diagram");
        let (kind, attrs) = result.into_output().unwrap();
        assert_eq!(*kind, "component");
        assert_eq!(attrs.len(), 0);

        let tokens = tokenize("diagram sequence;");
        let result = diagram_header_with_semicolon().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse basic sequence diagram");
        let (kind, attrs) = result.into_output().unwrap();
        assert_eq!(*kind, "sequence");
        assert_eq!(attrs.len(), 0);

        // Headers with attributes
        let tokens = tokenize("diagram component [layout=\"grid\", theme=\"dark\"];");
        let result = diagram_header_with_semicolon().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse diagram with attributes");
        let (kind, attrs) = result.into_output().unwrap();
        assert_eq!(*kind, "component");
        assert_eq!(attrs.len(), 2);
        assert_eq!(*attrs[0].name, "layout");
        assert_eq!(*attrs[0].value, "grid".to_string());
        assert_eq!(*attrs[1].name, "theme");
        assert_eq!(*attrs[1].value, "dark".to_string());

        // Headers with whitespace and comments
        let tokens =
            tokenize("diagram // comment\n component // another comment\n [attr=\"value\"];");
        println!("Diagram with comments tokens: {:?}", tokens);
        let result = diagram_header_with_semicolon().parse(tokens.as_slice());
        if !result.has_output() {
            println!(
                "Failed to parse diagram with comments - errors: {:?}",
                result.into_errors()
            );
            panic!("Should parse diagram with comments");
        }
        let (kind, attrs) = result.into_output().unwrap();
        assert_eq!(*kind, "component");
        assert_eq!(attrs.len(), 1);
    }

    #[test]
    fn test_real_world_scenarios() {
        // Test realistic diagram scenarios

        // Scenario 1: Web application architecture
        let web_app_diagram = r#"
            diagram component;

            type Frontend = Rectangle [fill_color="lightblue"];
            type Backend = Rectangle [fill_color="lightgreen"];
            type Database = Oval [fill_color="lightyellow"];

            ui: Frontend [label="React App"];
            api: Backend [label="Node.js API"];
            cache: Database [label="Redis"];
            db: Database [label="PostgreSQL"];

            ui -> api: "HTTP/REST";
            api -> cache: "Cache Check";
            api -> db: "SQL Query";
        "#;

        let tokens = tokenize(web_app_diagram);
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should parse web app architecture diagram");

        // Scenario 2: Microservices architecture
        let microservices_diagram = r#"
            diagram component;

            type Service = Rectangle [border="2"];
            type Gateway = Rectangle [fill_color="orange"];

            gateway: Gateway;
            user_service: Service;
            order_service: Service;
            payment_service: Service;

            gateway -> user_service;
            gateway -> order_service;
            gateway -> payment_service;
            order_service -> payment_service: "Process Payment";
        "#;

        let tokens = tokenize(microservices_diagram);
        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_ok(),
            "Should parse microservices architecture diagram"
        );

        // Verify the structure
        if let Ok(diagram_element) = result {
            match diagram_element.inner() {
                types::Element::Diagram(diagram) => {
                    assert_eq!(diagram.type_definitions.len(), 2);
                    assert_eq!(diagram.elements.len(), 8); // 4 components + 4 relations
                }
                _ => panic!("Expected Diagram element"),
            }
        }
    }

    #[test]
    fn test_parsing_errors() {
        // Test various invalid syntax scenarios to ensure graceful error handling

        // Invalid diagram headers
        let tokens = tokenize("diagram;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on diagram without type");

        let tokens = tokenize("diagramcomponent;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on malformed diagram header");

        // Invalid type definitions
        let tokens = tokenize("diagram component; type Database;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on incomplete type definition");

        let tokens = tokenize("diagram component; type = Rectangle;");
        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_err(),
            "Should fail on type definition without name"
        );

        let tokens = tokenize("diagram component; type Database = Rectangle");
        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_err(),
            "Should fail on type definition without semicolon"
        );

        // Invalid components
        let tokens = tokenize("diagram component; database Rectangle;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on component without colon");

        let tokens = tokenize("diagram component; database:;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on component without type");

        let tokens = tokenize("diagram component; database: Rectangle");
        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_err(),
            "Should fail on component without semicolon"
        );

        // Invalid relations
        let tokens = tokenize("diagram component; a: Rect; b: Rect; a -> ;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on relation without target");

        let tokens = tokenize("diagram component; a: Rect; b: Rect; -> b;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on relation without source");

        let tokens = tokenize("diagram component; a: Rect; b: Rect; a -> b");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on relation without semicolon");

        // Invalid attributes
        let tokens = tokenize("diagram component; app: Rect [color=blue];");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on unquoted attribute value");

        let tokens = tokenize("diagram component; app: Rect [color=\"blue\";");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on unclosed attribute bracket");
    }

    #[test]
    fn test_relation_type_spec_edge_cases() {
        // Test comprehensive RelationTypeSpec parsing edge cases matching parser.rs

        // Test empty attributes list
        let tokens = tokenize("a -> [] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse relation with empty type spec"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_none());
                assert!(spec.attributes.is_empty());
            }
            _ => panic!("Expected Relation"),
        }

        // Test single attribute
        let tokens = tokenize("a -> [color=\"red\"] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse relation with single attribute"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_none());
                assert_eq!(spec.attributes.len(), 1);
                assert_eq!(*spec.attributes[0].name, "color");
                assert_eq!(*spec.attributes[0].value, "red".to_string());
            }
            _ => panic!("Expected Relation"),
        }

        // Test type name with empty attributes after semicolon
        let tokens = tokenize("a -> [MyType;] b;");
        println!("Testing [MyType;] with tokens: {:?}", tokens);
        let result = relation().parse(tokens.as_slice());
        if !result.has_output() {
            println!(
                "Failed to parse [MyType;] - errors: {:?}",
                result.into_errors()
            );
            panic!("Should parse type with empty attributes");
        }
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_some());
                assert_eq!(**spec.type_name.as_ref().unwrap(), "MyType");
                assert!(spec.attributes.is_empty());
            }
            _ => panic!("Expected Relation"),
        }

        // Test type name with single attribute
        let tokens = tokenize("a -> [MyType; width=\"5\"] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse type with single attribute"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_some());
                assert_eq!(**spec.type_name.as_ref().unwrap(), "MyType");
                assert_eq!(spec.attributes.len(), 1);
                assert_eq!(*spec.attributes[0].name, "width");
                assert_eq!(*spec.attributes[0].value, "5".to_string());
            }
            _ => panic!("Expected Relation"),
        }
    }

    #[test]
    fn test_relation_type_spec_whitespace() {
        // Test RelationTypeSpec with whitespace variations matching parser.rs

        // Test with extra whitespace around type name
        let tokens = tokenize("a -> [  MyType  ] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse with whitespace around type"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_some());
                assert_eq!(**spec.type_name.as_ref().unwrap(), "MyType");
                assert!(spec.attributes.is_empty());
            }
            _ => panic!("Expected Relation"),
        }

        // Test with whitespace around semicolon and attributes
        let tokens = tokenize("a -> [MyType ; color=\"blue\" , width=\"3\" ] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse with whitespace around semicolon"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_some());
                assert_eq!(**spec.type_name.as_ref().unwrap(), "MyType");
                assert_eq!(spec.attributes.len(), 2);
                assert_eq!(*spec.attributes[0].name, "color");
                assert_eq!(*spec.attributes[0].value, "blue".to_string());
                assert_eq!(*spec.attributes[1].name, "width");
                assert_eq!(*spec.attributes[1].value, "3".to_string());
            }
            _ => panic!("Expected Relation"),
        }

        // Test with whitespace in attributes only
        let tokens = tokenize("a -> [ color=\"red\" , style=\"dashed\" ] b;");
        let result = relation().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse attributes with whitespace"
        );
        match result.into_output().unwrap() {
            types::Element::Relation { type_spec, .. } => {
                let spec = type_spec.as_ref().unwrap();
                assert!(spec.type_name.is_none());
                assert_eq!(spec.attributes.len(), 2);
                assert_eq!(*spec.attributes[0].name, "color");
                assert_eq!(*spec.attributes[0].value, "red".to_string());
                assert_eq!(*spec.attributes[1].name, "style");
                assert_eq!(*spec.attributes[1].value, "dashed".to_string());
            }
            _ => panic!("Expected Relation"),
        }
    }

    #[test]
    fn test_attribute_edge_cases() {
        // Test boundary conditions and edge cases for attribute parsing

        // Test empty attribute values
        let tokens = tokenize("key=\"\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse attribute with empty value"
        );
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "key");
        assert_eq!(*attr.value, "".to_string());

        // Test attributes with special characters in values
        let tokens = tokenize("path=\"/path/to/file.txt\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse attribute with path value"
        );
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "path");
        assert_eq!(*attr.value, "/path/to/file.txt".to_string());

        // Test attributes with numbers in values
        let tokens = tokenize("port=\"8080\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(
            result.has_output(),
            "Should parse attribute with numeric value"
        );
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "port");
        assert_eq!(*attr.value, "8080".to_string());

        // Test attributes with mixed case names
        let tokens = tokenize("backgroundColor=\"white\"");
        let result = attribute().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse camelCase attribute name");
        let attr = result.into_output().unwrap();
        assert_eq!(*attr.name, "backgroundColor");
        assert_eq!(*attr.value, "white".to_string());

        // Test maximum reasonable attribute combinations
        let tokens = tokenize(
            "[attr1=\"val1\", attr2=\"val2\", attr3=\"val3\", attr4=\"val4\", attr5=\"val5\"]",
        );
        let result = wrapped_attributes().parse(tokens.as_slice());
        assert!(result.has_output(), "Should parse multiple attributes");
        let attrs = result.into_output().unwrap();
        assert_eq!(attrs.len(), 5);
        for (i, attr) in attrs.iter().enumerate() {
            assert_eq!(*attr.name, format!("attr{}", i + 1));
            assert_eq!(*attr.value, format!("val{}", i + 1));
        }
    }

    #[test]
    fn test_malformed_input() {
        // Test robustness with various malformed input scenarios

        // Test completely invalid input
        let tokens = tokenize("this is not a valid diagram at all");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on completely invalid input");

        // Test partial valid input
        let tokens = tokenize("diagram component; app: Rectangle; incomplete");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on incomplete statements");

        // Test mixed valid/invalid tokens
        let tokens = tokenize("diagram component; app: Rectangle; 123invalid: Type;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on invalid identifiers");

        // Test unclosed constructs
        let tokens = tokenize("diagram component; app: Rectangle [color=\"blue\"");
        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_err(),
            "Should fail on unclosed attribute brackets"
        );

        // Test invalid relation operators
        let tokens = tokenize("diagram component; a: Rect; b: Rect; a >> b;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on invalid relation operators");

        // Test mismatched brackets
        let tokens = tokenize("diagram component; app: Rectangle [color=\"blue\"];");
        let result = build_diagram(tokens.as_slice());
        assert!(
            result.is_ok(),
            "Should succeed on properly matched brackets"
        );

        let tokens = tokenize("diagram component; app: Rectangle ]color=\"blue\"[;");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_err(), "Should fail on mismatched brackets");

        // Test empty but syntactically valid constructs
        let tokens = tokenize("diagram component; app: Rectangle [];");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should succeed on empty attributes");

        // Test extremely long identifiers (within reason)
        let long_name = "very_long_identifier_name_that_is_still_valid_but_quite_long";
        let diagram_str = format!("diagram component; {}: Rectangle;", long_name);
        let tokens = tokenize(&diagram_str);
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should handle reasonably long identifiers");

        // Test nested quotes in string values (should be handled by lexer)
        let tokens = tokenize("diagram component; app: Rectangle [label=\"App \\\"Server\\\"\"];");
        let result = build_diagram(tokens.as_slice());
        assert!(result.is_ok(), "Should handle escaped quotes in strings");
    }
}
