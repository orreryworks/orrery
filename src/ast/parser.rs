use crate::error::FilamentError;
use log::{debug, trace};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha1, alphanumeric1, char, multispace1, not_line_ending},
    combinator::{map, opt, recognize, value},
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    IResult, Parser,
};

#[derive(Debug)]
pub struct Attribute<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: &'a str,
    pub base_type: &'a str,
    pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug)]
pub struct Diagram<'a> {
    pub kind: &'a str,
    pub type_definitions: Vec<TypeDefinition<'a>>,
    pub elements: Vec<Element<'a>>,
}

#[derive(Debug)]
pub enum Element<'a> {
    Component {
        name: &'a str,
        type_name: &'a str, // TODO
        attributes: Vec<Attribute<'a>>,
        nested_elements: Vec<Element<'a>>,
    },
    Relation {
        source: &'a str,
        target: &'a str,
        relation_type: &'a str, // e.g., "->" or "<->". Could be an enum
        attributes: Vec<Attribute<'a>>,
    },
    Diagram(Diagram<'a>),
}

// Parses Rust-style line comments and whitespace
fn ws_comment(input: &str) -> IResult<&str, ()> {
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

fn ws_comments0(input: &str) -> IResult<&str, ()> {
    value((), many0(ws_comment)).parse(input)
}

fn ws_comments1(input: &str) -> IResult<&str, ()> {
    value((), many1(ws_comment)).parse(input)
}

// Define a parser for a standard identifier (starts with alpha, can contain alphanum or underscore)
fn parse_identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_")))))).parse(input)
    // NOTE: Why it is not working with char('_')?
}

fn parse_nested_identifier(input: &str) -> IResult<&str, &str> {
    recognize(separated_list1(tag("::"), parse_identifier)).parse(input)
}

fn parse_string_literal(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), take_while1(|c: char| c != '"'), char('"')).parse(input)
}

fn parse_attribute(input: &str) -> IResult<&str, Attribute> {
    map(
        separated_pair(
            parse_identifier,
            delimited(ws_comments0, char('='), ws_comments0),
            parse_string_literal,
        ),
        |(name, value)| Attribute { name, value },
    )
    .parse(input)
}

fn parse_attributes(input: &str) -> IResult<&str, Vec<Attribute>> {
    delimited(
        char('['),
        separated_list0(
            char(','),
            delimited(ws_comments0, parse_attribute, ws_comments0),
        ),
        char(']'),
    )
    .parse(input)
}

fn parse_type_definition(input: &str) -> IResult<&str, TypeDefinition> {
    map(
        delimited(
            ws_comments0,
            (
                pair(tag("type"), ws_comments1),
                parse_identifier,
                delimited(ws_comments0, char('='), ws_comments0),
                parse_identifier,
                preceded(ws_comments0, opt(parse_attributes)), // Allow 0 or more spaces before attributes
            ),
            pair(ws_comments0, char(';')),
        ),
        |(_, name, _, base_type, attributes)| TypeDefinition {
            name,
            base_type,
            attributes: attributes.unwrap_or_default(),
        },
    )
    .parse(input)
}

fn parse_component(input: &str) -> IResult<&str, Element> {
    map(
        terminated(
            (
                terminated(parse_identifier, ws_comments0),
                char(':'),
                delimited(ws_comments0, parse_identifier, ws_comments0),
                opt(parse_attributes),
                opt(delimited(
                    preceded(ws_comments0, char('{')),
                    parse_elements,
                    preceded(ws_comments0, char('}')),
                )),
            ),
            pair(ws_comments0, char(';')),
        ),
        |(name, _, type_name, attributes, nested_elements)| Element::Component {
            name,
            type_name,
            attributes: attributes.unwrap_or_default(),
            nested_elements: nested_elements.unwrap_or_default(),
        },
    )
    .parse(input)
}

// Parse a relation type like -> or <- or <-> or -
fn parse_relation_type(input: &str) -> IResult<&str, &str> {
    alt((tag("<->"), tag("<-"), tag("->"), tag("-"))).parse(input)
}

fn parse_relation(input: &str) -> IResult<&str, Element> {
    map(
        terminated(
            (
                terminated(parse_nested_identifier, ws_comments1),
                parse_relation_type,
                opt(preceded(ws_comments0, parse_attributes)), // Optional attributes
                delimited(
                    ws_comments1, // Require at least one space after relation type
                    parse_nested_identifier,
                    ws_comments0, // Target identifier with possible leading space
                ),
                opt((preceded(ws_comments0, char(':')), parse_string_literal)),
            ),
            pair(ws_comments0, char(';')),
        ),
        |(source, relation_type, attributes, target, _)| Element::Relation {
            source,
            target,
            attributes: attributes.unwrap_or_default(),
            relation_type,
        },
    )
    .parse(input)
}

fn parse_element(input: &str) -> IResult<&str, Element> {
    delimited(
        ws_comments0,
        alt((parse_component, parse_relation)),
        ws_comments0,
    )
    .parse(input)
}

fn parse_elements(input: &str) -> IResult<&str, Vec<Element>> {
    many0(parse_element).parse(input)
}

fn parse_diagram_header(input: &str) -> IResult<&str, &str> {
    delimited(
        pair(tag("diagram"), multispace1),
        parse_identifier,
        pair(ws_comments0, char(';')),
    )
    .parse(input)
}

fn parse_diagram(input: &str) -> IResult<&str, Element> {
    map(
        delimited(
            ws_comments0,
            (
                parse_diagram_header,
                many0(parse_type_definition),
                parse_elements,
            ),
            ws_comments0,
        ),
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
    match parse_diagram(input) {
        Ok((remaining, diagram)) => {
            if !remaining.is_empty() {
                return Err(FilamentError::Parse(format!(
                    "Unexpected trailing characters: {remaining}"
                )));
            }
            debug!("Diagram parsed successfully");
            trace!("Parsed diagram: {:?}", diagram);
            Ok(diagram)
        }
        Err(err) => Err(FilamentError::Parse(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_comment() {
        // Test whitespace
        let (rest, _) = ws_comment("   \n\t   ").unwrap();
        assert_eq!(rest, "");

        // Test comments
        let (rest, _) = ws_comment("// This is a comment").unwrap();
        assert_eq!(rest, "");

        // Test whitespace followed by comments
        let (rest, _) = ws_comment("  \n  // Comment").unwrap();
        assert_eq!(rest, "// Comment");

        // Test whitespace followed by content
        let (rest, _) = ws_comment("  \r\t  content").unwrap();
        assert_eq!(rest, "content");

        // Test comment followed by whitespace
        let (rest, _) = ws_comment("// Comment\n").unwrap();
        assert_eq!(rest, "\n");
    }

    #[test]
    fn test_ws_comments0() {
        // Test whitespace
        let (rest, _) = ws_comments0("   \n\t   ").unwrap();
        assert_eq!(rest, "");

        // Test comments
        let (rest, _) = ws_comments0("// This is a comment\n// Another comment").unwrap();
        assert_eq!(rest, "");

        // Test mixed whitespace and comments
        let (rest, _) = ws_comments0("  // Comment\n  // Another\n  ").unwrap();
        assert_eq!(rest, "");

        // Test with content after
        let (rest, _) = ws_comments0("  // Comment\n  content").unwrap();
        assert_eq!(rest, "content");

        // Test without content or whitespace
        let (rest, _) = ws_comments0("content").unwrap();
        assert_eq!(rest, "content");
    }

    #[test]
    fn test_ws_comments1() {
        // Test whitespace
        let (rest, _) = ws_comments1("   \n\t   ").unwrap();
        assert_eq!(rest, "");

        // Test comments
        let (rest, _) = ws_comments1("// This is a comment\n// Another comment").unwrap();
        assert_eq!(rest, "");

        // Test mixed whitespace and comments
        let (rest, _) = ws_comments1("  // Comment\n  // Another\n  ").unwrap();
        assert_eq!(rest, "");

        // Test with content after
        let (rest, _) = ws_comments1("  // Comment\n  content").unwrap();
        assert_eq!(rest, "content");

        // Test without content or whitespace
        assert!(ws_comments1("content").is_err());
    }

    #[test]
    fn test_parse_identifier() {
        // Test basic identifiers
        assert!(parse_identifier("simple").is_ok());
        assert!(parse_identifier("snake_case").is_ok());
        assert!(parse_identifier("camelCase").is_ok());
        assert!(parse_identifier("PascalCase").is_ok());

        // Test invalid identifiers
        assert!(parse_identifier("123invalid").is_err());
        assert!(parse_identifier("_invalid").is_err());
        assert!(parse_identifier("").is_err());
    }

    #[test]
    fn test_parse_nested_identifier() {
        // Test basic identifiers
        assert!(parse_nested_identifier("simple").is_ok());

        // Test nested identifiers
        assert!(parse_nested_identifier("parent::child").is_ok());
        assert!(parse_nested_identifier("module::sub_module::element").is_ok());

        // Test invalid identifiers
        assert!(parse_nested_identifier("_invalid").is_err());
        assert!(parse_nested_identifier("").is_err());
    }

    #[test]
    fn test_parse_string_literal() {
        // Test valid string literals
        let (rest, value) = parse_string_literal("\"hello\"").unwrap();
        assert_eq!(rest, "");
        assert_eq!(value, "hello");

        let (rest, value) = parse_string_literal("\"hello world\"").unwrap();
        assert_eq!(rest, "");
        assert_eq!(value, "hello world");

        // Test with content after the string
        let (rest, value) = parse_string_literal("\"hello\" world").unwrap();
        assert_eq!(rest, " world");
        assert_eq!(value, "hello");

        // Test invalid string literals
        assert!(parse_string_literal("hello").is_err());
        assert!(parse_string_literal("\"unclosed").is_err());
        assert!(parse_string_literal("\"\"").is_err()); // Empty strings are invalid per the implementation
    }

    #[test]
    fn test_parse_attribute() {
        // Test valid attributes
        let (rest, attr) = parse_attribute("color=\"blue\"").unwrap();
        assert_eq!(rest, "");
        assert_eq!(attr.name, "color");
        assert_eq!(attr.value, "blue");

        // Test with whitespace
        let (rest, attr) = parse_attribute("color = \"blue\"").unwrap();
        assert_eq!(rest, "");
        assert_eq!(attr.name, "color");
        assert_eq!(attr.value, "blue");

        // Test with comments
        let (rest, attr) = parse_attribute("color = // comment\n \"blue\"").unwrap();
        assert_eq!(rest, "");
        assert_eq!(attr.name, "color");
        assert_eq!(attr.value, "blue");

        // Test invalid attributes
        assert!(parse_attribute("color=blue").is_err());
        assert!(parse_attribute("123=\"blue\"").is_err());
        assert!(parse_attribute("color=\"").is_err());
    }

    #[test]
    fn test_parse_attributes() {
        // Test empty attributes
        let (rest, attrs) = parse_attributes("[]").unwrap();
        assert_eq!(rest, "");
        assert_eq!(attrs.len(), 0);

        // Test single attribute
        let (rest, attrs) = parse_attributes("[color=\"blue\"]").unwrap();
        assert_eq!(rest, "");
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].name, "color");
        assert_eq!(attrs[0].value, "blue");

        // Test multiple attributes
        let (rest, attrs) = parse_attributes("[color=\"blue\", size=\"large\"]").unwrap();
        assert_eq!(rest, "");
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].name, "color");
        assert_eq!(attrs[0].value, "blue");
        assert_eq!(attrs[1].name, "size");
        assert_eq!(attrs[1].value, "large");

        // Test with whitespace and comments
        let (rest, attrs) =
            parse_attributes("[color=\"blue\" // comment\n, size=\"large\"]").unwrap();
        assert_eq!(rest, "");
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].name, "color");
        assert_eq!(attrs[0].value, "blue");
        assert_eq!(attrs[1].name, "size");
        assert_eq!(attrs[1].value, "large");

        // Test invalid attributes
        assert!(parse_attributes("[color=blue]").is_err());
        assert!(parse_attributes("[color=\"blue\"").is_err());
        assert!(parse_attributes("[color=\"blue\", ]").is_err());
    }

    #[test]
    fn test_parse_type_definition() {
        // Test basic type definition
        let (rest, type_def) = parse_type_definition("type Database = Rectangle;").unwrap();
        assert_eq!(rest, "");
        assert_eq!(type_def.name, "Database");
        assert_eq!(type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 0);

        // Test type definition with attributes - notice the space before the attributes
        let (rest, type_def) =
            parse_type_definition("type Database = Rectangle [fill_color=\"blue\"];").unwrap();
        assert_eq!(rest, "");
        assert_eq!(type_def.name, "Database");
        assert_eq!(type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);
        assert_eq!(type_def.attributes[0].name, "fill_color");
        assert_eq!(type_def.attributes[0].value, "blue");

        // Test with whitespace and comments
        // FIXME: Fix the code to pass this test.
        let (rest, type_def) =
            parse_type_definition("type Database = Rectangle // comment\n [fill_color=\"blue\"];")
                .unwrap();
        assert_eq!(rest, "");
        assert_eq!(type_def.name, "Database");
        assert_eq!(type_def.base_type, "Rectangle");
        assert_eq!(type_def.attributes.len(), 1);
        assert_eq!(type_def.attributes[0].name, "fill_color");
        assert_eq!(type_def.attributes[0].value, "blue");

        // Test invalid type definitions
        assert!(parse_type_definition("type Database;").is_err());
        assert!(parse_type_definition("type = Rectangle;").is_err());
        assert!(parse_type_definition("type Database = Rectangle").is_err());
    }

    #[test]
    fn test_parse_component() {
        // Test basic component
        let (rest, component) = parse_component("database: Rectangle;").unwrap();
        assert_eq!(rest, "");
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(name, "database");
                assert_eq!(type_name, "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with attributes - note the space before attributes
        let (rest, component) =
            parse_component("database: Rectangle [fill_color=\"blue\"];").unwrap();
        assert_eq!(rest, "");
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(name, "database");
                assert_eq!(type_name, "Rectangle");
                assert_eq!(attributes.len(), 1);
                assert_eq!(attributes[0].name, "fill_color");
                assert_eq!(attributes[0].value, "blue");
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with attributes - note there is no space before attributes
        let (rest, component) =
            parse_component("server: Oval[fill_color=\"green\", line_color=\"black\"];").unwrap();
        assert_eq!(rest, "");
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(name, "server");
                assert_eq!(type_name, "Oval");
                assert_eq!(attributes.len(), 2);
                assert_eq!(attributes[0].name, "fill_color");
                assert_eq!(attributes[0].value, "green");
                assert_eq!(attributes[1].name, "line_color");
                assert_eq!(attributes[1].value, "black");
                assert_eq!(nested_elements.len(), 0);
            }
            _ => panic!("Expected Component"),
        }

        // Test component with nested elements
        let (rest, component) = parse_component("system: Rectangle { db: Database; };").unwrap();
        assert_eq!(rest, "");
        match component {
            Element::Component {
                name,
                type_name,
                attributes,
                nested_elements,
            } => {
                assert_eq!(name, "system");
                assert_eq!(type_name, "Rectangle");
                assert_eq!(attributes.len(), 0);
                assert_eq!(nested_elements.len(), 1);
                match &nested_elements[0] {
                    Element::Component {
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
        assert!(parse_component("database Rectangle;").is_err());
        assert!(parse_component("database:;").is_err());
        assert!(parse_component("database: Rectangle").is_err());
    }

    #[test]
    fn test_parse_relation() {
        // Test basic relation types
        let (rest, relation) = parse_relation("a -> b;").unwrap();
        assert_eq!(rest, "");
        match relation {
            Element::Relation {
                source,
                target,
                relation_type,
                attributes,
            } => {
                assert_eq!(source, "a");
                assert_eq!(target, "b");
                assert_eq!(relation_type, "->");
                assert_eq!(attributes.len(), 0);
            }
            _ => panic!("Expected Relation"),
        }

        let (_rest, relation) = parse_relation("a <- b;").unwrap();
        match relation {
            Element::Relation { relation_type, .. } => {
                assert_eq!(relation_type, "<-");
            }
            _ => panic!("Expected Relation"),
        }

        let (_rest, relation) = parse_relation("a <-> b;").unwrap();
        match relation {
            Element::Relation { relation_type, .. } => {
                assert_eq!(relation_type, "<->");
            }
            _ => panic!("Expected Relation"),
        }

        let (_rest, relation) = parse_relation("a - b;").unwrap();
        match relation {
            Element::Relation { relation_type, .. } => {
                assert_eq!(relation_type, "-");
            }
            _ => panic!("Expected Relation"),
        }

        // Test relation with attributes
        let (_rest, relation) = parse_relation("a -> [color=\"red\"] b;").unwrap();
        match relation {
            Element::Relation {
                source,
                target,
                relation_type,
                attributes,
            } => {
                assert_eq!(source, "a");
                assert_eq!(target, "b");
                assert_eq!(relation_type, "->");
                assert_eq!(attributes.len(), 1);
                assert_eq!(attributes[0].name, "color");
                assert_eq!(attributes[0].value, "red");
            }
            _ => panic!("Expected Relation"),
        }

        // Test nested identifiers in relations
        let (_rest, relation) = parse_relation("parent::child -> service;").unwrap();
        match relation {
            Element::Relation { source, target, .. } => {
                assert_eq!(source, "parent::child");
                assert_eq!(target, "service");
            }
            _ => panic!("Expected Relation"),
        }

        // Test invalid relations
        assert!(parse_relation("a -> ;").is_err());
        assert!(parse_relation("a >>b;").is_err());
        assert!(parse_relation("a -> b").is_err());
    }

    #[test]
    fn test_parse_diagram_header() {
        // Test basic diagram header
        let (rest, kind) = parse_diagram_header("diagram component;").unwrap();
        assert_eq!(rest, "");
        assert_eq!(kind, "component");

        let (rest, kind) = parse_diagram_header("diagram sequence;").unwrap();
        assert_eq!(rest, "");
        assert_eq!(kind, "sequence");

        // Test invalid diagram headers
        assert!(parse_diagram_header("diagram;").is_err());
        assert!(parse_diagram_header("diagram component").is_err());
        assert!(parse_diagram_header("diagramcomponent;").is_err());
    }

    #[test]
    fn test_parse_diagram() {
        // Test minimal diagram
        let diagram_str = "diagram component;";
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        assert_eq!(_rest, "");
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(d.kind, "component");
                assert_eq!(d.type_definitions.len(), 0);
                assert_eq!(d.elements.len(), 0);
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with type definition
        let diagram_str = "
            diagram component;
            type Database = Rectangle [fill_color=\"blue\"];
        ";
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(d.kind, "component");
                assert_eq!(d.type_definitions.len(), 1);
                assert_eq!(d.type_definitions[0].name, "Database");
                assert_eq!(d.elements.len(), 0);
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with components
        let diagram_str = "
            diagram component;
            app: Rectangle;
            db: Rectangle;
        ";
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(d.kind, "component");
                assert_eq!(d.type_definitions.len(), 0);
                assert_eq!(d.elements.len(), 2);
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with relations
        let diagram_str = "
            diagram component;
            app: Rectangle;
            db: Rectangle;
            app -> db;
        ";
        let (_rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(d.kind, "component");
                assert_eq!(d.elements.len(), 3);
                match &d.elements[2] {
                    Element::Relation { source, target, .. } => {
                        assert_eq!(source, &"app");
                        assert_eq!(target, &"db");
                    }
                    _ => panic!("Expected Relation"),
                }
            }
            _ => panic!("Expected Diagram"),
        }

        // Test diagram with nested components
        let diagram_str = "
            diagram component;
            system: Rectangle {
                app: Rectangle;
                db: Rectangle;
                app -> db;
            };
        ";
        let (rest, diagram) = parse_diagram(diagram_str).unwrap();
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(d.kind, "component");
                assert_eq!(d.elements.len(), 1);
                match &d.elements[0] {
                    Element::Component {
                        name,
                        nested_elements,
                        ..
                    } => {
                        assert_eq!(name, &"system");
                        assert_eq!(nested_elements.len(), 3);
                    }
                    _ => panic!("Expected Component"),
                }
            }
            _ => panic!("Expected Diagram"),
        }
        assert!(rest.is_empty());

        // Test complex diagram (from spec example)
        let diagram_str = "
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
        ";
        let result = parse_diagram(diagram_str);
        assert!(result.is_ok());
        let (rest, diagram) = result.unwrap();
        match diagram {
            Element::Diagram(d) => {
                assert_eq!(d.kind, "component");
                assert_eq!(d.type_definitions.len(), 3);
                assert_eq!(d.elements.len(), 5); // 3 components + 2 relations
            }
            _ => panic!("Expected Diagram"),
        }
        assert!(rest.is_empty());
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
