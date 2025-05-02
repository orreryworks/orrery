use super::parser_types as types;
use crate::ast::span::Spanned;
use crate::error::{ParseDiagnosticError, SlimParserError};
use log::{debug, trace};
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha1, alphanumeric1, char, multispace1, not_line_ending},
    combinator::{all_consuming, cut, map, not, opt, peek, recognize, value},
    error::context,
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;
type PResult<'a, T> = IResult<Span<'a>, Spanned<T>, SlimParserError>;

fn to_spanned<'a, T>(input: Span, result: IResult<Span<'a>, T, SlimParserError>) -> PResult<'a, T> {
    match result {
        Ok((span, val)) => Ok((
            span,
            Spanned::new(
                val,
                input.location_offset(),
                span.location_offset() - input.location_offset(),
            ),
        )),

        Err(err) => Err(err),
    }
}

fn semicolon(input: Span) -> PResult<()> {
    to_spanned(
        input,
        cut(value(
            (),
            pair(ws_comments0, context("semicolon", char(';'))),
        ))
        .parse(input),
    )
}

// Parses Rust-style line comments and whitespace
fn ws_comment(input: Span) -> PResult<()> {
    to_spanned(
        input,
        value(
            (),
            alt((
                // Match whitespace
                multispace1,
                // Match Rust style comments
                recognize(pair(tag("//"), not_line_ending)),
            )),
        )
        .parse(input),
    )
}

fn ws_comments0(input: Span) -> PResult<()> {
    to_spanned(input, value((), many0(ws_comment)).parse(input))
}

fn ws_comments1(input: Span) -> PResult<()> {
    to_spanned(input, value((), many1(ws_comment)).parse(input))
}

// Define a parser for a standard identifier (starts with alpha, can contain alphanum or underscore)
fn parse_identifier(input: Span) -> PResult<&str> {
    to_spanned(
        input,
        context(
            "identifier",
            map(
                recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_")))))),
                |v: LocatedSpan<&str>| v.into_fragment(),
            ),
        )
        .parse(input),
    )
    // NOTE: Why it is not working with char('_')?
}

fn parse_nested_identifier(input: Span) -> PResult<&str> {
    to_spanned(
        input,
        context(
            "nested_identifier",
            map(
                recognize(separated_list1(tag("::"), parse_identifier)),
                |v: LocatedSpan<&str>| v.into_fragment(),
            ),
        )
        .parse(input),
    )
}

fn parse_string_literal(input: Span) -> PResult<&str> {
    to_spanned(
        input,
        context(
            "string_literal",
            map(
                delimited(char('"'), take_while1(|c: char| c != '"'), cut(char('"'))),
                |v: LocatedSpan<&str>| v.into_fragment(),
            ),
        )
        .parse(input),
    )
}

fn parse_attribute(input: Span) -> PResult<types::Attribute> {
    to_spanned(
        input,
        context(
            "attribute",
            map(
                // FIXME: Why I cannot add cut() here?
                separated_pair(
                    parse_identifier,
                    delimited(ws_comments0, char('='), ws_comments0),
                    parse_string_literal,
                ),
                |(name, value)| types::Attribute { name, value },
            ),
        )
        .parse(input),
    )
}

fn parse_attributes(input: Span) -> PResult<Vec<Spanned<types::Attribute>>> {
    to_spanned(
        input,
        context(
            "attributes",
            delimited(
                char('['),
                separated_list0(
                    char(','),
                    delimited(ws_comments0, parse_attribute, ws_comments0),
                ),
                cut(char(']')),
            ),
        )
        .parse(input),
    )
}

fn parse_type_definition(input: Span) -> PResult<types::TypeDefinition> {
    to_spanned(
        input,
        context(
            "type_definition",
            map(
                delimited(
                    ws_comments0,
                    (
                        pair(tag("type"), ws_comments1),
                        cut((
                            parse_identifier,
                            delimited(ws_comments0, char('='), ws_comments0),
                            parse_identifier,
                            preceded(ws_comments0, opt(parse_attributes)), // Allow 0 or more spaces before attributes
                        )),
                    ),
                    semicolon,
                ),
                |(_, (name, _, base_type, attributes))| types::TypeDefinition {
                    name,
                    base_type,
                    attributes: attributes.unwrap_or_default(),
                },
            ),
        )
        .parse(input),
    )
}

fn parse_type_definitions(input: Span) -> PResult<Vec<Spanned<types::TypeDefinition>>> {
    to_spanned(input, many0(parse_type_definition).parse(input))
}

fn parse_component(input: Span) -> PResult<types::Element> {
    to_spanned(
        input,
        context(
            "component",
            map(
                terminated(
                    (
                        terminated(parse_identifier, ws_comments0),
                        char(':'),
                        peek(not(char(':'))),
                        cut((
                            delimited(ws_comments0, parse_identifier, ws_comments0),
                            opt(parse_attributes),
                            opt(delimited(
                                preceded(ws_comments0, char('{')),
                                parse_elements,
                                preceded(ws_comments0, char('}')),
                            )),
                        )),
                    ),
                    semicolon,
                ),
                |(name, _, _, (type_name, attributes, nested_elements))| {
                    types::Element::Component {
                        name,
                        type_name,
                        attributes: attributes.unwrap_or_default(),
                        nested_elements: nested_elements.unwrap_or_default(),
                    }
                },
            ),
        )
        .parse(input),
    )
}

// Parse a relation type like -> or <- or <-> or -
fn parse_relation_type(input: Span) -> PResult<&str> {
    to_spanned(
        input,
        context(
            "relation_type",
            map(
                alt((tag("<->"), tag("<-"), tag("->"), tag("-"))),
                |v: LocatedSpan<&str>| v.into_fragment(),
            ),
        )
        .parse(input),
    )
}

fn parse_relation(input: Span) -> PResult<types::Element> {
    to_spanned(
        input,
        context(
            "relation",
            map(
                terminated(
                    (
                        terminated(parse_nested_identifier, ws_comments1),
                        parse_relation_type,
                        cut((
                            opt(preceded(ws_comments0, parse_attributes)), // Optional attributes
                            delimited(
                                ws_comments1, // Require at least one space after relation type
                                parse_nested_identifier,
                                ws_comments0, // Target identifier with possible leading space
                            ),
                            opt((preceded(ws_comments0, char(':')), parse_string_literal)),
                        )),
                    ),
                    semicolon,
                ),
                |(source, relation_type, (attributes, target, _))| types::Element::Relation {
                    source,
                    target,
                    attributes: attributes.unwrap_or_default(),
                    relation_type,
                },
            ),
        )
        .parse(input),
    )
}

fn parse_element(input: Span) -> PResult<types::Element> {
    delimited(
        ws_comments0,
        alt((parse_component, parse_relation)),
        ws_comments0,
    )
    .parse(input)
}

fn parse_elements(input: Span) -> PResult<Vec<Spanned<types::Element>>> {
    to_spanned(input, many0(parse_element).parse(input))
}

fn parse_diagram_header(input: Span) -> PResult<&str> {
    context(
        "header",
        cut(delimited(
            pair(context("diagram_keyword", tag("diagram")), multispace1),
            context("diagram_type", parse_identifier),
            semicolon,
        )),
    )
    .parse(input)
}

fn parse_diagram(input: Span) -> PResult<types::Element> {
    to_spanned(
        input,
        map(
            all_consuming(delimited(
                ws_comments0,
                (parse_diagram_header, parse_type_definitions, parse_elements),
                ws_comments0,
            )),
            |(kind, type_definitions, elements)| {
                types::Element::Diagram(types::Diagram {
                    kind,
                    type_definitions,
                    elements,
                })
            },
        )
        .parse(input),
    )
}

pub fn build_diagram(input: &str) -> Result<Spanned<types::Element>, ParseDiagnosticError> {
    debug!("Starting diagram parsing, input length: {}", input.len());

    // Create a span with the full input
    let input_span = Span::new(input);

    // Pass the full input to our parser
    match parse_diagram(input_span) {
        Ok((remaining, diagram)) => {
            if !remaining.is_empty() {
                // Create a slim error for the remaining content
                let err = SlimParserError::new(remaining, nom::error::ErrorKind::NonEmpty);

                // Convert to full error with the complete source
                return Err(err.move_to_full_error(input));
            }
            debug!("Diagram parsed successfully");
            trace!("Parsed diagram: {:?}", diagram);
            Ok(diagram)
        }
        Err(nom::Err::Error(err) | nom::Err::Failure(err)) => {
            trace!("Parser error: kind={:?}, offset={}", err.kind, err.offset);

            // Convert the slim error to a full error with context
            Err(err.move_to_full_error(input))
        }
        Err(err) => {
            trace!("Other parser error");
            Err(ParseDiagnosticError {
                src: input.to_string(),
                message: err.to_string(),
                span: Some((0, input_span.len()).into()),
                help: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semicolon() {
        // Test semicolon
        let input = Span::new(";");
        let (rest, _) = semicolon(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test with whitespace
        let input = Span::new("   \t\r\n  ;");
        let (rest, _) = semicolon(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test without semicolon
        let input = Span::new("content");
        assert!(semicolon(input).is_err());
    }

    #[test]
    fn test_ws_comment() {
        // Test whitespace
        let input = Span::new("   \n\t   ");
        let (rest, _) = ws_comment(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test comments
        let input = Span::new("// This is a comment");
        let (rest, _) = ws_comment(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test whitespace followed by comments
        let input = Span::new("  \n  // Comment");
        let (rest, _) = ws_comment(input).unwrap();
        assert_eq!(*rest.fragment(), "// Comment");

        // Test whitespace followed by content
        let input = Span::new("  \r\t  content");
        let (rest, _) = ws_comment(input).unwrap();
        assert_eq!(*rest.fragment(), "content");

        // Test comment followed by whitespace
        let input = Span::new("// Comment\n");
        let (rest, _) = ws_comment(input).unwrap();
        assert_eq!(*rest.fragment(), "\n");
    }

    #[test]
    fn test_ws_comments0() {
        // Test whitespace
        let input = Span::new("   \n\t   ");
        let (rest, _) = ws_comments0(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test comments
        let input = Span::new("// This is a comment\n// Another comment");
        let (rest, _) = ws_comments0(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test mixed whitespace and comments
        let input = Span::new("  // Comment\n  // Another\n  ");
        let (rest, _) = ws_comments0(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test with content after
        let input = Span::new("  // Comment\n  content");
        let (rest, _) = ws_comments0(input).unwrap();
        assert_eq!(*rest.fragment(), "content");

        // Test without content or whitespace
        let input = Span::new("content");
        let (rest, _) = ws_comments0(input).unwrap();
        assert_eq!(*rest.fragment(), "content");
    }

    #[test]
    fn test_ws_comments1() {
        // Test whitespace
        let input = Span::new("   \n\t   ");
        let (rest, _) = ws_comments1(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test comments
        let input = Span::new("// This is a comment\n// Another comment");
        let (rest, _) = ws_comments1(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test mixed whitespace and comments
        let input = Span::new("  // Comment\n  // Another\n  ");
        let (rest, _) = ws_comments1(input).unwrap();
        assert_eq!(*rest.fragment(), "");

        // Test with content after
        let input = Span::new("  // Comment\n  content");
        let (rest, _) = ws_comments1(input).unwrap();
        assert_eq!(*rest.fragment(), "content");

        // Test without content or whitespace
        let input = Span::new("content");
        assert!(ws_comments1(input).is_err());
    }

    #[test]
    fn test_parse_identifier() {
        // Test basic identifiers
        assert!(parse_identifier(Span::new("simple")).is_ok());
        assert!(parse_identifier(Span::new("snake_case")).is_ok());
        assert!(parse_identifier(Span::new("camelCase")).is_ok());
        assert!(parse_identifier(Span::new("PascalCase")).is_ok());

        // Test invalid identifiers
        assert!(parse_identifier(Span::new("123invalid")).is_err());
        assert!(parse_identifier(Span::new("_invalid")).is_err());
        assert!(parse_identifier(Span::new("")).is_err());
    }

    #[test]
    fn test_parse_nested_identifier() {
        // Test basic identifiers
        assert!(parse_nested_identifier(Span::new("simple")).is_ok());

        // Test nested identifiers
        assert!(parse_nested_identifier(Span::new("parent::child")).is_ok());
        assert!(parse_nested_identifier(Span::new("module::sub_module::element")).is_ok());

        // Test invalid identifiers
        assert!(parse_nested_identifier(Span::new("_invalid")).is_err());
        assert!(parse_nested_identifier(Span::new("")).is_err());
    }

    #[test]
    fn test_parse_string_literal() {
        // Test valid string literals
        let input = Span::new("\"hello\"");
        let (rest, value) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*value, "hello");

        let input = Span::new("\"hello world\"");
        let (rest, value) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*value, "hello world");

        // Test with content after the string
        let input = Span::new("\"hello\" world");
        let (rest, value) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), " world");
        assert_eq!(*value, "hello");

        // Test invalid string literals
        assert!(parse_string_literal(Span::new("hello")).is_err());
        assert!(parse_string_literal(Span::new("\"unclosed")).is_err());
        assert!(parse_string_literal(Span::new("\"\"")).is_err()); // Empty strings are invalid per the implementation
    }

    #[test]
    fn test_parse_attribute() {
        // Test valid attributes
        let input = Span::new("color=\"blue\"");
        let (rest, attr) = parse_attribute(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*attr.name, "color");
        assert_eq!(*attr.value, "blue");

        // Test with whitespace
        let input = Span::new("color = \"blue\"");
        let (rest, attr) = parse_attribute(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*attr.name, "color");
        assert_eq!(*attr.value, "blue");

        // Test with comments
        let input = Span::new("color = // comment\n \"blue\"");
        let (rest, attr) = parse_attribute(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*attr.name, "color");
        assert_eq!(*attr.value, "blue");

        // Test invalid attributes
        assert!(parse_attribute(Span::new("color=blue")).is_err());
        assert!(parse_attribute(Span::new("123=\"blue\"")).is_err());
        assert!(parse_attribute(Span::new("color=\"")).is_err());
    }

    #[test]
    fn test_parse_attributes() {
        // Test empty attributes
        let input = Span::new("[]");
        let (rest, attrs) = parse_attributes(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(attrs.len(), 0);

        // Test single attribute
        let input = Span::new("[color=\"blue\"]");
        let (rest, attrs) = parse_attributes(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(attrs.len(), 1);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "blue");

        // Test multiple attributes
        let input = Span::new("[color=\"blue\", size=\"large\"]");
        let (rest, attrs) = parse_attributes(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(attrs.len(), 2);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "blue");
        assert_eq!(*attrs[1].name, "size");
        assert_eq!(*attrs[1].value, "large");

        // Test with whitespace and comments
        let input = Span::new("[color=\"blue\" // comment\n, size=\"large\"]");
        let (rest, attrs) = parse_attributes(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(attrs.len(), 2);
        assert_eq!(*attrs[0].name, "color");
        assert_eq!(*attrs[0].value, "blue");
        assert_eq!(*attrs[1].name, "size");
        assert_eq!(*attrs[1].value, "large");

        // Test invalid attributes
        assert!(parse_attributes(Span::new("[color=blue]")).is_err());
        assert!(parse_attributes(Span::new("[color=\"blue\"")).is_err());
        assert!(parse_attributes(Span::new("[color=\"blue\", ]")).is_err());
    }

    #[test]
    fn test_parse_type_definition() {
        // Test basic type definition
        let input = Span::new("type Database = Rectangle;");
        let (rest, type_def) = parse_type_definition(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*type_def.name, "Database");
        assert_eq!(*type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 0);

        // Test type definition with attributes - notice the space before the attributes
        let input = Span::new("type Database = Rectangle [fill_color=\"blue\"];");
        let (rest, type_def) = parse_type_definition(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*type_def.name, "Database");
        assert_eq!(*type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);
        assert_eq!(*type_def.attributes[0].name, "fill_color");
        assert_eq!(*type_def.attributes[0].value, "blue");

        // Test with whitespace and comments
        // FIXME: Fix the code to pass this test.
        let input = Span::new("type Database = Rectangle // comment\n [fill_color=\"blue\"];");
        let (rest, type_def) = parse_type_definition(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*type_def.name, "Database");
        assert_eq!(*type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);
        assert_eq!(*type_def.attributes[0].name, "fill_color");
        assert_eq!(*type_def.attributes[0].value, "blue");

        // Test invalid type definitions
        assert!(parse_type_definition(Span::new("type Database;")).is_err());
        assert!(parse_type_definition(Span::new("type = Rectangle;")).is_err());
        assert!(parse_type_definition(Span::new("type Database = Rectangle")).is_err());
    }

    #[test]
    fn test_parse_component() {
        // Test basic component
        let input = Span::new("database: Rectangle;");
        let (rest, component) = parse_component(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match component.into_inner() {
            types::Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "database");
                assert_eq!(*type_name, "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with attributes - note the space before attributes
        let input = Span::new("database: Rectangle [fill_color=\"blue\"];");
        let (rest, component) = parse_component(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match component.into_inner() {
            types::Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "database");
                assert_eq!(*type_name, "Rectangle");
                assert_eq!(attributes.len(), 1);
                assert_eq!(*attributes[0].name, "fill_color");
                assert_eq!(*attributes[0].value, "blue");
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with attributes - note there is no space before attributes
        let input = Span::new("server: Oval[fill_color=\"green\", line_color=\"black\"];");
        let (rest, component) = parse_component(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match component.into_inner() {
            types::Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "server");
                assert_eq!(*type_name, "Oval");
                assert_eq!(attributes.len(), 2);
                assert_eq!(*attributes[0].name, "fill_color");
                assert_eq!(*attributes[0].value, "green");
                assert_eq!(*attributes[1].name, "line_color");
                assert_eq!(*attributes[1].value, "black");
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with nested elements
        let input = Span::new("system: Rectangle { db: Database; };");
        let (rest, component) = parse_component(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match component.into_inner() {
            types::Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name, "system");
                assert_eq!(*type_name, "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 1);
                match nested_elements.into_inner().pop().unwrap().into_inner() {
                    types::Element::Component {
                        name, type_name, ..
                    } => {
                        assert_eq!(*name, "db");
                        assert_eq!(*type_name, "Database");
                    }
                    _ => panic!("Expected Component"),
                }
            }
            _ => panic!("Expected Component"),
        }

        // Test invalid components
        assert!(parse_component(Span::new("database Rectangle;")).is_err());
        assert!(parse_component(Span::new("database:;")).is_err());
        assert!(parse_component(Span::new("database: Rectangle")).is_err());
    }

    #[test]
    fn test_parse_relation() {
        // Test basic relation types
        let input = Span::new("a -> b;");
        let (rest, relation) = parse_relation(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match relation.into_inner() {
            types::Element::Relation {
                source,
                target,
                relation_type,
                attributes,
            } => {
                assert_eq!(*source, "a");
                assert_eq!(*target, "b");
                assert_eq!(*relation_type, "->");
                assert_eq!(attributes.len(), 0);
            }
            _ => panic!("Expected Relation"),
        }

        let input = Span::new("a <- b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation.into_inner() {
            types::Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type, "<-");
            }
            _ => panic!("Expected Relation"),
        }

        let input = Span::new("a <-> b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation.into_inner() {
            types::Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type, "<->");
            }
            _ => panic!("Expected Relation"),
        }

        let input = Span::new("a - b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation.into_inner() {
            types::Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type, "-");
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with attributes
        let input = Span::new("a -> [color=\"red\"] b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation.into_inner() {
            types::Element::Relation {
                source,
                target,
                relation_type,
                attributes,
            } => {
                assert_eq!(*source, "a");
                assert_eq!(*target, "b");
                assert_eq!(*relation_type, "->");
                assert_eq!(attributes.len(), 1);
                assert_eq!(*attributes[0].name, "color");
                assert_eq!(*attributes[0].value, "red");
            }
            _ => panic!("Expected Relation"),
        }

        // Test nested identifiers in relations
        let input = Span::new("parent::child -> service;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation.into_inner() {
            types::Element::Relation { source, target, .. } => {
                assert_eq!(*source, "parent::child");
                assert_eq!(*target, "service");
            }
            _ => panic!("Expected Relation"),
        }

        // Test invalid relations
        assert!(parse_relation(Span::new("a -> ;")).is_err());
        assert!(parse_relation(Span::new("a >>b;")).is_err());
        assert!(parse_relation(Span::new("a -> b")).is_err());
    }

    #[test]
    fn test_parse_diagram_header() {
        // Test basic diagram header
        let input = Span::new("diagram component;");
        let (rest, kind) = parse_diagram_header(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*kind, "component");

        let input = Span::new("diagram sequence;");
        let (rest, kind) = parse_diagram_header(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*kind, "sequence");

        // Test invalid diagram headers
        assert!(parse_diagram_header(Span::new("diagram;")).is_err());
        assert!(parse_diagram_header(Span::new("diagram component")).is_err());
        assert!(parse_diagram_header(Span::new("diagramcomponent;")).is_err());
    }

    #[test]
    fn test_parse_diagram() {
        // Test minimal diagram
        let diagram_str = Span::new("diagram component;");
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        assert_eq!(*_rest.fragment(), "");
        match diagram.into_inner() {
            types::Element::Diagram(d) => {
                assert_eq!(*d.kind, "component");
                assert_eq!(d.type_definitions.len(), 0);
                assert_eq!(d.elements.len(), 0);
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with type definition
        let diagram_str = Span::new(
            "
            diagram component;
            type Database = Rectangle [fill_color=\"blue\"];
        ",
        );
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram.into_inner() {
            types::Element::Diagram(d) => {
                assert_eq!(*d.kind, "component");
                assert_eq!(d.type_definitions.len(), 1);
                assert_eq!(*d.type_definitions[0].name, "Database");
                assert_eq!(d.elements.len(), 0);
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with components
        let diagram_str = Span::new(
            "
            diagram component;
            app: Rectangle;
            db: Rectangle;
        ",
        );
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram.into_inner() {
            types::Element::Diagram(d) => {
                assert_eq!(*d.kind, "component");
                assert_eq!(d.type_definitions.len(), 0);
                assert_eq!(d.elements.len(), 2);
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with relations
        let diagram_str = Span::new(
            "
            diagram component;
            app: Rectangle;
            db: Rectangle;
            app -> db;
        ",
        );
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram.into_inner() {
            types::Element::Diagram(d) => {
                assert_eq!(*d.kind, "component");
                assert_eq!(d.elements.len(), 3);
                match d.elements.into_inner().remove(2).into_inner() {
                    types::Element::Relation { source, target, .. } => {
                        assert_eq!(*source, "app");
                        assert_eq!(*target, "db");
                    }
                    _ => panic!("Expected Relation"),
                }
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with nested components
        let diagram_str = Span::new(
            "
            diagram component;
            system: Rectangle {
                app: Rectangle;
                db: Rectangle;
                app -> db;
            };
        ",
        );
        let (rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram.into_inner() {
            types::Element::Diagram(d) => {
                assert_eq!(*d.kind, "component");
                assert_eq!(d.elements.len(), 1);
                match d.elements.into_inner().pop().unwrap().into_inner() {
                    types::Element::Component {
                        name,
                        nested_elements,
                        ..
                    } => {
                        assert_eq!(*name, "system");
                        assert_eq!(nested_elements.len(), 3);
                    }
                    _ => panic!("Expected Component"),
                }
            }
            _ => panic!("Expected Diagram"),
        }
        assert_eq!(*rest.fragment(), "");

        // Test complex diagram (from spec example)
        let diagram_str = Span::new(
            "
            diagram component;

            type Database = Rectangle [fill_color=\"lightblue\", rounded=\"10\"];
            type Service = Rectangle [fill_color=\"#e6f3ff\"];
            type Client = Oval [fill_color=\"#ffe6e6\"];

            end_user: Client;
            backend_system: Service {
                auth_service: Service;
                user_db: Database;
                auth_service -> user_db;
            };
            api_gateway: Service;

            end_user -> api_gateway;
            api_gateway -> backend_system;
        ",
        );
        let result = parse_diagram(diagram_str);
        assert!(result.is_ok());
        let (rest, diagram) = result.unwrap();
        match diagram.into_inner() {
            types::Element::Diagram(d) => {
                assert_eq!(*d.kind, "component");
                assert_eq!(d.type_definitions.len(), 3);
                assert_eq!(d.elements.len(), 5); // 3 components + 2 relations
            }
            _ => panic!("Expected Diagram"),
        }
        assert_eq!(*rest.fragment(), "");
    }

    #[test]
    fn test_build_diagram() {
        // Test successful parsing
        let diagram_str = "diagram component; app: Rectangle; db: Rectangle; app -> db;";
        let result = build_diagram(diagram_str);
        assert!(result.is_ok());

        // Test with trailing content
        let diagram_str =
            "diagram component; app: Rectangle; db: Rectangle; app -> db; extra stuff";
        let result = build_diagram(diagram_str);
        assert!(result.is_err());

        // Test with syntax error
        let diagram_str = "diagram component; app: Rectangle; db: ; app -> db;";
        let result = build_diagram(diagram_str);
        assert!(result.is_err());
    }
}
