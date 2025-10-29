//! Parser AST types
//!
//! This module defines the data structures representing parsed Filament diagrams.
//! These types form the output of the parser.
//!
//! ## Source Location Tracking
//!
//! Leaf values are wrapped in [`Spanned<T>`] to preserve source location information
//! for error reporting. Composite types derive their spans from their contents.

use super::span::{Span, Spanned};
use crate::identifier::Id;
use std::fmt;
#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: Spanned<Id>,
    pub base_type: Spanned<Id>,
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

/// Attribute values can be strings, floats, nested attributes, identifier lists, or empty
///
/// **Variants:**
/// - `String` - Text values for colors, names, alignment, etc.
/// - `Float` - Numeric values for dimensions, widths, sizes, etc.
/// - `Attributes` - Nested key-value pairs for complex attributes (stroke, text)
/// - `Identifiers` - Lists of element identifiers (used in note `on` attribute)
/// - `Empty` - Ambiguous empty brackets `[]` that can be interpreted as either
///   empty identifiers or empty nested attributes depending on context
///
/// **Empty Variant Design:**
/// The `Empty` variant elegantly solves the `[]` ambiguity problem:
/// - Both `as_identifiers()` and `as_attributes()` return success with empty slice
/// - Allows `on=[]` (margin note) and `text=[]` (empty attributes) to parse correctly
/// - Parser doesn't need to know the semantic context during parsing
#[derive(Debug, Clone)]
pub enum AttributeValue<'a> {
    String(Spanned<String>),
    Float(Spanned<f32>),
    Attributes(Vec<Attribute<'a>>),
    Identifiers(Vec<Spanned<Id>>),
    Empty,
}

impl<'a> PartialEq for AttributeValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AttributeValue::String(s1), AttributeValue::String(s2)) => s1.inner() == s2.inner(),
            (AttributeValue::Float(f1), AttributeValue::Float(f2)) => f1.inner() == f2.inner(),
            (AttributeValue::Attributes(a1), AttributeValue::Attributes(a2)) => a1 == a2,
            (AttributeValue::Identifiers(l1), AttributeValue::Identifiers(l2)) => l1
                .iter()
                .map(|s| s.inner())
                .eq(l2.iter().map(|s| s.inner())),
            (AttributeValue::Empty, AttributeValue::Empty) => true,
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
            AttributeValue::Identifiers(ids) => {
                write!(f, "[")?;
                for (i, id) in ids.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", id.inner())?;
                }
                write!(f, "]")
            }
            AttributeValue::Empty => write!(f, "[]"),
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
            AttributeValue::Identifiers(ids) => {
                if ids.is_empty() {
                    Span::default()
                } else {
                    ids.iter()
                        .map(|id| id.span())
                        .reduce(|acc, span| acc.union(span))
                        .unwrap_or_default()
                }
            }
            AttributeValue::Empty => Span::default(),
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
        match self {
            AttributeValue::Attributes(attrs) => Ok(attrs),
            AttributeValue::Empty => Ok(&[]),
            _ => Err("Expected nested attributes"),
        }
    }

    /// Extract an identifier list, returning an error if this is not an identifiers value
    pub fn as_identifiers(&self) -> Result<&[Spanned<Id>], &'static str> {
        match self {
            AttributeValue::Identifiers(ids) => Ok(ids),
            AttributeValue::Empty => Ok(&[]),
            _ => Err("Expected identifiers"),
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

/// AST node representing a note element
///
/// **Syntax:**
/// ```text
/// note [attributes]: "content";
/// ```
///
/// **Fields:**
/// - `attributes` - Optional configuration for positioning, styling, and attachment
/// - `content` - The note text as a string literal (supports escape sequences)
#[derive(Debug)]
pub struct Note<'a> {
    pub attributes: Vec<Attribute<'a>>,
    pub content: Spanned<String>,
}

impl Note<'_> {
    pub fn span(&self) -> Span {
        self.attributes
            .iter()
            .map(|attr| attr.span())
            .fold(self.content.span(), |acc, span| acc.union(span))
    }
}

#[derive(Debug)]
pub enum Element<'a> {
    Component {
        name: Spanned<Id>,
        display_name: Option<Spanned<String>>,
        type_name: Spanned<Id>,
        attributes: Vec<Attribute<'a>>,
        nested_elements: Vec<Element<'a>>,
    },
    /// Relation between two components
    ///
    /// Note: `source` and `target` are `String` instead of `&'a str` because they may be
    /// nested identifiers (e.g., "frontend::app") created by joining multiple parts with "::".
    Relation {
        source: Spanned<Id>,
        target: Spanned<Id>,
        relation_type: Spanned<&'a str>,
        type_spec: Option<RelationTypeSpec<'a>>,
        label: Option<Spanned<String>>,
    },
    Diagram(Diagram<'a>),
    Fragment(Fragment<'a>),
    ActivateBlock {
        component: Spanned<Id>,
        elements: Vec<Element<'a>>,
    },
    /// Explicit activation of a component
    Activate {
        component: Spanned<Id>,
    },
    /// Explicit deactivation of a component
    Deactivate {
        component: Spanned<Id>,
    },
    /// Alt/else block (sugar syntax for fragment with "alt" operation)
    AltElseBlock {
        keyword_span: Span,
        sections: Vec<FragmentSection<'a>>,
        attributes: Vec<Attribute<'a>>,
    },
    /// Opt block (sugar syntax for fragment with "opt" operation)
    OptBlock {
        keyword_span: Span,
        section: FragmentSection<'a>,
        attributes: Vec<Attribute<'a>>,
    },
    /// Loop block (sugar syntax for fragment with "loop" operation)
    LoopBlock {
        keyword_span: Span,
        section: FragmentSection<'a>,
        attributes: Vec<Attribute<'a>>,
    },
    /// Par block (sugar syntax for fragment with "par" operation)
    ParBlock {
        keyword_span: Span,
        sections: Vec<FragmentSection<'a>>,
        attributes: Vec<Attribute<'a>>,
    },
    /// Break block (sugar syntax for fragment with "break" operation)
    BreakBlock {
        keyword_span: Span,
        section: FragmentSection<'a>,
        attributes: Vec<Attribute<'a>>,
    },
    /// Critical block (sugar syntax for fragment with "critical" operation)
    CriticalBlock {
        keyword_span: Span,
        section: FragmentSection<'a>,
        attributes: Vec<Attribute<'a>>,
    },
    /// Note element with optional attributes and text content
    Note(Note<'a>),
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

            // Fragment sugar syntax: multiple sections
            Element::AltElseBlock {
                keyword_span,
                sections,
                attributes,
            }
            | Element::ParBlock {
                keyword_span,
                sections,
                attributes,
            } => {
                let mut span = *keyword_span;
                for section in sections {
                    span = span.union(section.span());
                }
                for attr in attributes {
                    span = span.union(attr.span());
                }
                span
            }

            // Fragment sugar syntax: single section
            Element::OptBlock {
                keyword_span,
                section,
                attributes,
            }
            | Element::LoopBlock {
                keyword_span,
                section,
                attributes,
            }
            | Element::BreakBlock {
                keyword_span,
                section,
                attributes,
            }
            | Element::CriticalBlock {
                keyword_span,
                section,
                attributes,
            } => attributes
                .iter()
                .map(|attr| attr.span())
                .fold((*keyword_span).union(section.span()), |acc, span| {
                    acc.union(span)
                }),
            Element::Note(note) => note.span(),
        }
    }
}

#[derive(Debug)]
pub struct RelationTypeSpec<'a> {
    pub type_name: Option<Spanned<Id>>,
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
