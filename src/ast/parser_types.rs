use super::span::{Span, SpanImpl, Spanned};

/// AST types that utilize span information
/// This module contains parser types with a span.
/// Leaf types (strings, literals) are wrapped in Spanned<T>
/// Composite types use unwrapped collections and derive spans from inner elements
#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: Spanned<&'a str>,
    pub base_type: Spanned<&'a str>,
    pub attributes: Vec<Attribute<'a>>,
}

impl TypeDefinition<'_> {
    pub fn span(&self) -> SpanImpl {
        let span = self.name.span().union(self.base_type.span());

        self.attributes
            .iter()
            .map(|attr| attr.span())
            .fold(span, |acc, span| acc.union(span))
    }
}

#[derive(Debug)]
pub struct Attribute<'a> {
    pub name: Spanned<&'a str>,
    pub value: Spanned<String>,
}

#[derive(Debug)]
pub struct Diagram<'a> {
    pub kind: Spanned<&'a str>,
    pub attributes: Vec<Attribute<'a>>,
    pub type_definitions: Vec<TypeDefinition<'a>>,
    pub elements: Vec<Element<'a>>,
}

impl Diagram<'_> {
    pub fn span(&self) -> SpanImpl {
        let kind_span = self.kind.span();

        let attr_spans = self.attributes.iter().map(|attr| attr.span());

        let type_def_spans = self.type_definitions.iter().map(|td| td.span());

        let element_spans = self.elements.iter().map(|elem| elem.span());

        attr_spans
            .chain(type_def_spans)
            .chain(element_spans)
            .fold(kind_span, |acc, span| acc.union(span))
    }
}

#[derive(Debug)]
pub enum Element<'a> {
    Component {
        name: Spanned<&'a str>,
        display_name: Option<Spanned<String>>,
        type_name: Spanned<&'a str>,
        attributes: Vec<Attribute<'a>>,
        nested_elements: Vec<Element<'a>>,
    },
    /// Relation between two components
    ///
    /// Note: `source` and `target` are `String` instead of `&'a str` because they may be
    /// nested identifiers (e.g., "frontend::app") created by joining multiple parts with "::".
    Relation {
        source: Spanned<String>,
        target: Spanned<String>,
        relation_type: Spanned<&'a str>,
        type_spec: Option<RelationTypeSpec<'a>>,
        label: Option<Spanned<String>>,
    },
    Diagram(Diagram<'a>),
}

impl Element<'_> {
    pub fn span(&self) -> SpanImpl {
        match self {
            Element::Component {
                name,
                display_name,
                type_name,
                attributes,
                nested_elements,
            } => {
                let span = name.span().union(type_name.span());

                let span = attributes
                    .iter()
                    .map(|attr| attr.span())
                    .fold(span, |acc, span| acc.union(span));

                let mut span = nested_elements
                    .iter()
                    .map(|elem| elem.span())
                    .fold(span, |acc, span| acc.union(span));

                if let Some(display_name) = display_name {
                    span = span.union(display_name.span());
                }

                span
            }
            Element::Relation {
                source,
                target,
                relation_type,
                type_spec,
                label,
            } => {
                let mut span = source
                    .span()
                    .union(target.span())
                    .union(relation_type.span());

                if let Some(type_spec) = type_spec {
                    span = span.union(type_spec.span());
                }

                if let Some(label) = label {
                    span = span.union(label.span());
                }

                span
            }
            Element::Diagram(diagram) => diagram.span(),
        }
    }
}

#[derive(Debug)]
pub struct RelationTypeSpec<'a> {
    pub type_name: Option<Spanned<&'a str>>,
    pub attributes: Vec<Attribute<'a>>,
}

impl RelationTypeSpec<'_> {
    pub fn span(&self) -> SpanImpl {
        if let Some(type_name) = &self.type_name {
            self.attributes
                .iter()
                .map(|attr| attr.span())
                .fold(type_name.span(), |acc, span| acc.union(span))
        } else {
            self.attributes
                .iter()
                .map(|attr| attr.span())
                .reduce(|acc, span| acc.union(span))
                .unwrap_or_else(|| SpanImpl::new((), 0..0))
        }
    }
}

impl Attribute<'_> {
    pub fn span(&self) -> SpanImpl {
        self.name.span().union(self.value.span())
    }
}
