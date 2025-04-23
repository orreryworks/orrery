use crate::error::FilamentError;
use log::{debug, trace};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha1, alphanumeric1, char, multispace0, multispace1},
    combinator::{map, opt, recognize},
    multi::{many0, separated_list0},
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

// --- Parser Combinators ---
fn parse_identifier(input: &str) -> IResult<&str, &str> {
    // Define a parser for a standard identifier (starts with alpha, can contain alphanum or underscore)
    let standard_identifier = || recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_"))))));
    
    // Now allow multiple identifiers separated by :: for nested identifiers
    let nested_identifier = recognize(pair(
        standard_identifier(),
        many0(preceded(tag("::"), standard_identifier())),
    ));
    
    map(nested_identifier, |s: &str| s).parse(input)
}

fn parse_string_literal(input: &str) -> IResult<&str, &str> {
    map(
        delimited(tag("\""), take_while1(|c: char| c != '"'), tag("\"")),
        |s: &str| s,
    )
    .parse(input)
}

fn parse_attribute(input: &str) -> IResult<&str, Attribute> {
    map(
        separated_pair(
            parse_identifier,
            delimited(multispace0, char('='), multispace0),
            parse_string_literal,
        ),
        |(name, value)| Attribute { name, value },
    )
    .parse(input)
}

fn parse_attributes(input: &str) -> IResult<&str, Vec<Attribute>> {
    delimited(
        terminated(char('['), multispace0),
        separated_list0(
            delimited(multispace0, char(','), multispace0),
            parse_attribute,
        ),
        preceded(multispace0, char(']')),
    )
    .parse(input)
}

fn parse_type_definition(input: &str) -> IResult<&str, TypeDefinition> {
    map(
        delimited(
            multispace0,
            (
                terminated(tag("type"), multispace1),
                parse_identifier,
                delimited(multispace0, char('='), multispace0),
                parse_identifier,
                opt(parse_attributes),
            ),
            preceded(multispace0, char(';')),
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
                parse_identifier,
                delimited(multispace0, char(':'), multispace0),
                parse_identifier,
                opt(parse_attributes),
                opt(delimited(
                    preceded(multispace0, char('{')),
                    parse_elements,
                    preceded(multispace0, char('}')),
                )),
            ),
            preceded(multispace0, char(';')),
        ),
        |(name, _, type_name, attributes, nested_elements)| Element::Component {
            name,
            type_name, // TODO
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
                parse_identifier,
                delimited(multispace1, parse_relation_type, multispace0),
                opt(parse_attributes),
                parse_identifier,
                opt((preceded(multispace0, char(':')), parse_string_literal)),
            ),
            preceded(multispace0, char(';')),
        ),
        |(source, rel_type, attributes, target, _)| Element::Relation {
            source,
            target,
            attributes: attributes.unwrap_or_default(),
            relation_type: rel_type,
        },
    )
    .parse(input)
}

fn parse_element(input: &str) -> IResult<&str, Element> {
    delimited(
        multispace0,
        alt((parse_component, parse_relation)),
        multispace0,
    )
    .parse(input)
}

fn parse_elements(input: &str) -> IResult<&str, Vec<Element>> {
    many0(preceded(multispace0, parse_element)).parse(input)
}

fn parse_diagram_header(input: &str) -> IResult<&str, &str> {
    map(
        terminated((tag("diagram"), multispace1, parse_identifier), char(';')),
        |(_, _, kind)| kind,
    )
    .parse(input)
}

fn parse_diagram(input: &str) -> IResult<&str, Element> {
    map(
        delimited(
            multispace0,
            (
                parse_diagram_header,
                many0(parse_type_definition),
                parse_elements,
            ),
            multispace0,
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
