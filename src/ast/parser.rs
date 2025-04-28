use crate::ast::error::ParserError;
use crate::error::FilamentError;
use log::{debug, trace};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha1, alphanumeric1, char, multispace1, not_line_ending},
    combinator::{all_consuming, cut, map, not, opt, peek, recognize, value},
    error::context,
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    IResult, Parser,
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;
type ParseResult<'a, T> = IResult<Span<'a>, T, ParserError>;

#[derive(Debug)]
pub struct Attribute<'a> {
    pub name: Span<'a>,
    pub value: Span<'a>,
}

#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: Span<'a>,
    pub base_type: Span<'a>,
    pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug)]
pub struct Diagram<'a> {
    pub kind: Span<'a>,
    pub type_definitions: Vec<TypeDefinition<'a>>,
    pub elements: Vec<Element<'a>>,
}

#[derive(Debug)]
pub enum Element<'a> {
    Component {
        name: Span<'a>,
        type_name: Span<'a>, // TODO
        attributes: Vec<Attribute<'a>>,
        nested_elements: Vec<Element<'a>>,
    },
    Relation {
        source: Span<'a>,
        target: Span<'a>,
        relation_type: Span<'a>, // e.g., "->" or "<->". Could be an enum
        attributes: Vec<Attribute<'a>>,
    },
    Diagram(Diagram<'a>),
}

fn semicolon(input: Span) -> ParseResult<()> {
    cut(value(
        (),
        pair(ws_comments0, context("semicolon", char(';'))),
    ))
    .parse(input)
}

// Parses Rust-style line comments and whitespace
fn ws_comment(input: Span) -> ParseResult<()> {
    value(
        (),
        alt((
            // Match whitespace
            multispace1,
            // Match Rust style comments
            recognize(pair(tag("//"), not_line_ending)),
        )),
    )
    .parse(input)
}

fn ws_comments0(input: Span) -> ParseResult<()> {
    value((), many0(ws_comment)).parse(input)
}

fn ws_comments1(input: Span) -> ParseResult<()> {
    value((), many1(ws_comment)).parse(input)
}

// Define a parser for a standard identifier (starts with alpha, can contain alphanum or underscore)
fn parse_identifier(input: Span) -> ParseResult<Span> {
    context(
        "identifier",
        recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_")))))),
    )
    .parse(input)
    // NOTE: Why it is not working with char('_')?
}

fn parse_nested_identifier(input: Span) -> ParseResult<Span> {
    context(
        "nested_identifier",
        recognize(separated_list1(tag("::"), parse_identifier)),
    )
    .parse(input)
}

fn parse_string_literal(input: Span) -> ParseResult<Span> {
    context(
        "string_literal",
        delimited(char('"'), take_while1(|c: char| c != '"'), cut(char('"'))),
    )
    .parse(input)
}

fn parse_attribute(input: Span) -> ParseResult<Attribute> {
    context(
        "attribute",
        map(
            // FIXME: Why I cannot add cut() here?
            separated_pair(
                parse_identifier,
                delimited(ws_comments0, char('='), ws_comments0),
                parse_string_literal,
            ),
            |(name, value)| Attribute { name, value },
        ),
    )
    .parse(input)
}

fn parse_attributes(input: Span) -> ParseResult<Vec<Attribute>> {
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
    .parse(input)
}

fn parse_type_definition(input: Span) -> ParseResult<TypeDefinition> {
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
            |(_, (name, _, base_type, attributes))| TypeDefinition {
                name,
                base_type,
                attributes: attributes.unwrap_or_default(),
            },
        ),
    )
    .parse(input)
}

fn parse_component(input: Span) -> ParseResult<Element> {
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
            |(name, _, _, (type_name, attributes, nested_elements))| Element::Component {
                name,
                type_name,
                attributes: attributes.unwrap_or_default(),
                nested_elements: nested_elements.unwrap_or_default(),
            },
        ),
    )
    .parse(input)
}

// Parse a relation type like -> or <- or <-> or -
fn parse_relation_type(input: Span) -> ParseResult<Span> {
    context(
        "relation_type",
        alt((tag("<->"), tag("<-"), tag("->"), tag("-"))),
    )
    .parse(input)
}

fn parse_relation(input: Span) -> ParseResult<Element> {
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
            |(source, relation_type, (attributes, target, _))| Element::Relation {
                source,
                target,
                attributes: attributes.unwrap_or_default(),
                relation_type,
            },
        ),
    )
    .parse(input)
}

fn parse_element(input: Span) -> ParseResult<Element> {
    delimited(
        ws_comments0,
        alt((parse_component, parse_relation)),
        ws_comments0,
    )
    .parse(input)
}

fn parse_elements(input: Span) -> ParseResult<Vec<Element>> {
    many0(parse_element).parse(input)
}

fn parse_diagram_header(input: Span) -> ParseResult<Span> {
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

fn parse_diagram(input: Span) -> ParseResult<Element> {
    map(
        all_consuming(delimited(
            ws_comments0,
            (
                parse_diagram_header,
                many0(parse_type_definition),
                parse_elements,
            ),
            ws_comments0,
        )),
        |(kind, type_definitions, elements)| {
            Element::Diagram(Diagram {
                kind,
                type_definitions,
                elements,
            })
        },
    )
    .parse(input)
}

pub fn build_diagram(input: &str) -> Result<Element, FilamentError> {
    debug!("Starting diagram parsing, input length: {}", input.len());

    // Create a span with the full input
    let input_span = Span::new(input);

    // Pass the full input to our parser
    match parse_diagram(input_span) {
        Ok((remaining, diagram)) => {
            if !remaining.is_empty() {
                // Create a proper error with location information
                let mut err = ParserError::new(remaining, nom::error::ErrorKind::NonEmpty);
                err.src = input.to_string();

                return Err(err.into());
            }
            debug!("Diagram parsed successfully");
            trace!("Parsed diagram: {:?}", diagram);
            Ok(diagram)
        }
        Err(nom::Err::Error(mut err) | nom::Err::Failure(mut err)) => {
            trace!("ParserError: {:?}", err);

            // Make sure the error has the full source
            err.src = input.to_string();
            Err(err.into())
        }
        Err(err) => {
            trace!("Other parser error");
            Err(FilamentError::Parse(err.to_string()))
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
        assert_eq!(*value.fragment(), "hello");

        let input = Span::new("\"hello world\"");
        let (rest, value) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*value.fragment(), "hello world");

        // Test with content after the string
        let input = Span::new("\"hello\" world");
        let (rest, value) = parse_string_literal(input).unwrap();
        assert_eq!(*rest.fragment(), " world");
        assert_eq!(*value.fragment(), "hello");

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
        assert_eq!(*attr.name.fragment(), "color");
        assert_eq!(*attr.value.fragment(), "blue");

        // Test with whitespace
        let input = Span::new("color = \"blue\"");
        let (rest, attr) = parse_attribute(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*attr.name.fragment(), "color");
        assert_eq!(*attr.value.fragment(), "blue");

        // Test with comments
        let input = Span::new("color = // comment\n \"blue\"");
        let (rest, attr) = parse_attribute(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*attr.name.fragment(), "color");
        assert_eq!(*attr.value.fragment(), "blue");

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
        assert_eq!(*attrs[0].name.fragment(), "color");
        assert_eq!(*attrs[0].value.fragment(), "blue");

        // Test multiple attributes
        let input = Span::new("[color=\"blue\", size=\"large\"]");
        let (rest, attrs) = parse_attributes(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(attrs.len(), 2);
        assert_eq!(*attrs[0].name.fragment(), "color");
        assert_eq!(*attrs[0].value.fragment(), "blue");
        assert_eq!(*attrs[1].name.fragment(), "size");
        assert_eq!(*attrs[1].value.fragment(), "large");

        // Test with whitespace and comments
        let input = Span::new("[color=\"blue\" // comment\n, size=\"large\"]");
        let (rest, attrs) = parse_attributes(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(attrs.len(), 2);
        assert_eq!(*attrs[0].name.fragment(), "color");
        assert_eq!(*attrs[0].value.fragment(), "blue");
        assert_eq!(*attrs[1].name.fragment(), "size");
        assert_eq!(*attrs[1].value.fragment(), "large");

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
        assert_eq!(*type_def.name.fragment(), "Database");
        assert_eq!(*type_def.base_type.fragment(), "Rectangle");
        assert_eq!(type_def.attributes.len(), 0);

        // Test type definition with attributes - notice the space before the attributes
        let input = Span::new("type Database = Rectangle [fill_color=\"blue\"];");
        let (rest, type_def) = parse_type_definition(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*type_def.name.fragment(), "Database");
        assert_eq!(*type_def.base_type.fragment(), "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);
        assert_eq!(*type_def.attributes[0].name.fragment(), "fill_color");
        assert_eq!(*type_def.attributes[0].value.fragment(), "blue");

        // Test with whitespace and comments
        // FIXME: Fix the code to pass this test.
        let input = Span::new("type Database = Rectangle // comment\n [fill_color=\"blue\"];");
        let (rest, type_def) = parse_type_definition(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*type_def.name.fragment(), "Database");
        assert_eq!(*type_def.base_type.fragment(), "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);
        assert_eq!(*type_def.attributes[0].name.fragment(), "fill_color");
        assert_eq!(*type_def.attributes[0].value.fragment(), "blue");

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
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name.fragment(), "database");
                assert_eq!(*type_name.fragment(), "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with attributes - note the space before attributes
        let input = Span::new("database: Rectangle [fill_color=\"blue\"];");
        let (rest, component) = parse_component(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name.fragment(), "database");
                assert_eq!(*type_name.fragment(), "Rectangle");
                assert_eq!(attributes.len(), 1);
                assert_eq!(*attributes[0].name.fragment(), "fill_color");
                assert_eq!(*attributes[0].value.fragment(), "blue");
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with attributes - note there is no space before attributes
        let input = Span::new("server: Oval[fill_color=\"green\", line_color=\"black\"];");
        let (rest, component) = parse_component(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name.fragment(), "server");
                assert_eq!(*type_name.fragment(), "Oval");
                assert_eq!(attributes.len(), 2);
                assert_eq!(*attributes[0].name.fragment(), "fill_color");
                assert_eq!(*attributes[0].value.fragment(), "green");
                assert_eq!(*attributes[1].name.fragment(), "line_color");
                assert_eq!(*attributes[1].value.fragment(), "black");
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with nested elements
        let input = Span::new("system: Rectangle { db: Database; };");
        let (rest, component) = parse_component(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(*name.fragment(), "system");
                assert_eq!(*type_name.fragment(), "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 1);
                match &nested_elements[0] {
                    Element::Component {
                        name, type_name, ..
                    } => {
                        assert_eq!(*name.fragment(), "db");
                        assert_eq!(*type_name.fragment(), "Database");
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
        match relation {
            Element::Relation {
                source,
                target,
                relation_type,
                attributes,
            } => {
                assert_eq!(*source.fragment(), "a");
                assert_eq!(*target.fragment(), "b");
                assert_eq!(*relation_type.fragment(), "->");
                assert_eq!(attributes.len(), 0);
            }
            _ => panic!("Expected Relation"),
        }

        let input = Span::new("a <- b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation {
            Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type.fragment(), "<-");
            }
            _ => panic!("Expected Relation"),
        }

        let input = Span::new("a <-> b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation {
            Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type.fragment(), "<->");
            }
            _ => panic!("Expected Relation"),
        }

        let input = Span::new("a - b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation {
            Element::Relation { relation_type, .. } => {
                assert_eq!(*relation_type.fragment(), "-");
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with attributes
        let input = Span::new("a -> [color=\"red\"] b;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation {
            Element::Relation {
                source,
                target,
                relation_type,
                attributes,
            } => {
                assert_eq!(*source.fragment(), "a");
                assert_eq!(*target.fragment(), "b");
                assert_eq!(*relation_type.fragment(), "->");
                assert_eq!(attributes.len(), 1);
                assert_eq!(*attributes[0].name.fragment(), "color");
                assert_eq!(*attributes[0].value.fragment(), "red");
            }
            _ => panic!("Expected Relation"),
        }

        // Test nested identifiers in relations
        let input = Span::new("parent::child -> service;");
        let (_rest, relation) = parse_relation(input).unwrap();
        match relation {
            Element::Relation { source, target, .. } => {
                assert_eq!(*source.fragment(), "parent::child");
                assert_eq!(*target.fragment(), "service");
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
        assert_eq!(*kind.fragment(), "component");

        let input = Span::new("diagram sequence;");
        let (rest, kind) = parse_diagram_header(input).unwrap();
        assert_eq!(*rest.fragment(), "");
        assert_eq!(*kind.fragment(), "sequence");

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
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(*d.kind.fragment(), "component");
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
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(*d.kind.fragment(), "component");
                assert_eq!(d.type_definitions.len(), 1);
                assert_eq!(*d.type_definitions[0].name.fragment(), "Database");
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
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(*d.kind.fragment(), "component");
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
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(*d.kind.fragment(), "component");
                assert_eq!(d.elements.len(), 3);
                match &d.elements[2] {
                    Element::Relation { source, target, .. } => {
                        assert_eq!(*source.fragment(), "app");
                        assert_eq!(*target.fragment(), "db");
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
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(*d.kind.fragment(), "component");
                assert_eq!(d.elements.len(), 1);
                match &d.elements[0] {
                    Element::Component {
                        name,
                        nested_elements,
                        ..
                    } => {
                        assert_eq!(*name.fragment(), "system");
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
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(*d.kind.fragment(), "component");
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
