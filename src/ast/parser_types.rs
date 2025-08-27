use super::span::{Span, Spanned};
use std::fmt;

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
    pub fn span(&self) -> Span {
        let span = self.name.span().union(self.base_type.span());

        self.attributes
            .iter()
            .map(|attr| attr.span())
            .fold(span, |acc, span| acc.union(span))
    }
}

/// Attribute values can be either strings, float numbers, or nested attributes
#[derive(Debug, Clone)]
pub enum AttributeValue<'a> {
    String(Spanned<String>),
    Float(Spanned<f32>),
    Attributes(Vec<Attribute<'a>>),
}

impl<'a> PartialEq for AttributeValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AttributeValue::String(s1), AttributeValue::String(s2)) => s1.inner() == s2.inner(),
            (AttributeValue::Float(f1), AttributeValue::Float(f2)) => f1.inner() == f2.inner(),
            (AttributeValue::Attributes(a1), AttributeValue::Attributes(a2)) => a1 == a2,
            _ => false,
        }
    }
}

impl<'a> fmt::Display for AttributeValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttributeValue::String(s) => write!(f, "\"{}\"", s.inner()),
            AttributeValue::Float(n) => write!(f, "{}", n.inner()),
            AttributeValue::Attributes(attrs) => {
                write!(f, "[")?;
                for (i, attr) in attrs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}={}", attr.name.inner(), attr.value)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl<'a> AttributeValue<'a> {
    /// Get the span for this attribute value
    pub fn span(&self) -> Span {
        match self {
            AttributeValue::String(spanned) => spanned.span(),
            AttributeValue::Float(spanned) => spanned.span(),
            AttributeValue::Attributes(attrs) => {
                if attrs.is_empty() {
                    Span::default()
                } else {
                    attrs
                        .iter()
                        .map(|attr| attr.span())
                        .reduce(|acc, span| acc.union(span))
                        .unwrap_or_default()
                }
            }
        }
    }

    /// Extract a string reference, returning an error if this is not a string value
    pub fn as_str(&self) -> Result<&str, &'static str> {
        if let AttributeValue::String(s) = self {
            Ok(s.inner())
        } else {
            Err("Expected string value")
        }
    }

    /// Extract a float value, returning an error if this is not a float value
    pub fn as_float(&self) -> Result<f32, &'static str> {
        if let AttributeValue::Float(f) = self {
            Ok(*f.inner())
        } else {
            Err("Expected float value")
        }
    }

    /// Extract a numeric value as u32 (casting f32 if necessary)
    pub fn as_u32(&self) -> Result<u32, &'static str> {
        if let AttributeValue::Float(f) = self {
            Ok(*f.inner() as u32)
        } else {
            Err("Expected float value")
        }
    }

    /// Extract a numeric value as usize (casting f32 if necessary)
    pub fn as_usize(&self) -> Result<usize, &'static str> {
        if let AttributeValue::Float(f) = self {
            Ok(*f.inner() as usize)
        } else {
            Err("Expected float value")
        }
    }

    /// Extract a numeric value as u16 (casting f32 if necessary)
    pub fn as_u16(&self) -> Result<u16, &'static str> {
        if let AttributeValue::Float(f) = self {
            Ok(*f.inner() as u16)
        } else {
            Err("Expected float value")
        }
    }

    /// Extract nested attributes, returning an error if this is an attributes value
    pub fn as_attributes(&self) -> Result<&[Attribute<'a>], &'static str> {
        if let AttributeValue::Attributes(attrs) = self {
            Ok(attrs)
        } else {
            Err("Expected nested attributes")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'a> {
    pub name: Spanned<&'a str>,
    pub value: AttributeValue<'a>,
}

#[derive(Debug)]
pub struct Diagram<'a> {
    pub kind: Spanned<&'a str>,
    pub attributes: Vec<Attribute<'a>>,
    pub type_definitions: Vec<TypeDefinition<'a>>,
    pub elements: Vec<Element<'a>>,
}

impl Diagram<'_> {
    pub fn span(&self) -> Span {
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
pub struct FragmentSection<'a> {
    pub title: Option<Spanned<String>>,
    pub elements: Vec<Element<'a>>,
}

impl FragmentSection<'_> {
    pub fn span(&self) -> Span {
        let elements_span = self
            .elements
            .iter()
            .map(|elem| elem.span())
            .reduce(|acc, span| acc.union(span));

        match (&self.title, elements_span) {
            (Some(title), Some(es)) => title.span().union(es),
            (Some(title), None) => title.span(),
            (None, Some(es)) => es,
            (None, None) => Span::default(),
        }
    }
}

#[derive(Debug)]
pub struct Fragment<'a> {
    pub operation: Spanned<String>,
    pub sections: Vec<FragmentSection<'a>>,
    pub attributes: Vec<Attribute<'a>>,
}

impl Fragment<'_> {
    pub fn span(&self) -> Span {
        self.sections
            .iter()
            .map(|section| section.span())
            .fold(self.operation.span(), |acc, span| acc.union(span))
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
    Fragment(Fragment<'a>),
    ActivateBlock {
        component: Spanned<String>,
        elements: Vec<Element<'a>>,
    },
    /// Explicit activation of a component
    Activate {
        component: Spanned<String>,
    },
    /// Explicit deactivation of a component
    Deactivate {
        component: Spanned<String>,
    },
}

impl Element<'_> {
    pub fn span(&self) -> Span {
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
            Element::Fragment(fragment) => fragment.span(),
            Element::ActivateBlock {
                component,
                elements,
            } => elements
                .iter()
                .map(|elem| elem.span())
                .fold(component.span(), |acc, span| acc.union(span)),
            Element::Activate { component } => component.span(),
            Element::Deactivate { component } => component.span(),
        }
    }
}

#[derive(Debug)]
pub struct RelationTypeSpec<'a> {
    pub type_name: Option<Spanned<&'a str>>,
    pub attributes: Vec<Attribute<'a>>,
}

impl RelationTypeSpec<'_> {
    pub fn span(&self) -> Span {
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
                .unwrap_or_default()
        }
    }
}

impl Attribute<'_> {
    pub fn span(&self) -> Span {
        self.name.span().union(self.value.span())
    }
}
