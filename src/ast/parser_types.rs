use super::span::Spanned;

/// AST types that utilize span information
/// This module contains versions of the parser.rs types but with proper span tracking

#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: Spanned<&'a str>,
    pub base_type: Spanned<&'a str>,
    pub attributes: Spanned<Vec<Spanned<Attribute<'a>>>>,
}

#[derive(Debug)]
pub struct Attribute<'a> {
    pub name: Spanned<&'a str>,
    pub value: Spanned<&'a str>,
}

#[derive(Debug)]
pub struct Diagram<'a> {
    pub kind: Spanned<&'a str>,
    pub attributes: Spanned<Vec<Spanned<Attribute<'a>>>>,
    pub type_definitions: Spanned<Vec<Spanned<TypeDefinition<'a>>>>,
    pub elements: Spanned<Vec<Spanned<Element<'a>>>>,
}

#[derive(Debug)]
pub enum Element<'a> {
    Component {
        name: Spanned<&'a str>,
        display_name: Option<Spanned<&'a str>>,
        type_name: Spanned<&'a str>,
        attributes: Spanned<Vec<Spanned<Attribute<'a>>>>,
        nested_elements: Spanned<Vec<Spanned<Element<'a>>>>,
    },
    Relation {
        source: Spanned<&'a str>,
        target: Spanned<&'a str>,
        relation_type: Spanned<&'a str>,
        attributes: Spanned<Vec<Spanned<Attribute<'a>>>>,
        label: Option<Spanned<&'a str>>,
    },
    Diagram(Diagram<'a>),
}
