//! Parser AST types
//!
//! This module defines the data structures representing parsed Filament diagrams.
//! These types form the output of the parser.
//!
//! ## Source Location Tracking
//!
//! Leaf values are wrapped in [`Spanned<T>`] to preserve source location information
//! for error reporting. Composite types derive their spans from their contents.

use std::fmt;

use super::span::{Span, Spanned};
use crate::identifier::Id;

/// Type Specifier - used in both declarations and invocations
///
/// Represents a type with optional attributes:
/// - `TypeName[attrs]` - Named with attributes
/// - `TypeName` - Named without attributes
/// - `[attrs]` - Anonymous (no type name, just attributes)
#[derive(Debug, Clone, Default)]
pub struct TypeSpec<'a> {
    pub type_name: Option<Spanned<Id>>,
    pub attributes: Vec<Attribute<'a>>,
}

impl<'a> TypeSpec<'a> {
    pub fn span(&self) -> Span {
        match &self.type_name {
            Some(name) => self
                .attributes
                .iter()
                .map(|attr| attr.span())
                .fold(name.span(), |acc, span| acc.union(span)),
            None => self
                .attributes
                .iter()
                .map(|attr| attr.span())
                .reduce(|acc, span| acc.union(span))
                .unwrap_or_default(),
        }
    }
}

impl<'a> fmt::Display for TypeSpec<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.type_name {
            write!(f, "{}", name)?;
        }
        if !self.attributes.is_empty() {
            write!(f, "[")?;
            for (i, attr) in self.attributes.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", attr)?;
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

/// Empty TypeSpec constant for use with Empty variant
static EMPTY_TYPE_SPEC: TypeSpec<'static> = TypeSpec {
    type_name: None,
    attributes: Vec::new(),
};

/// Attribute values can be strings, floats, nested attributes, identifier lists, or empty
///
/// **Variants:**
/// - `String` - Text values for colors, names, alignment, etc.
/// - `Float` - Numeric values for dimensions, widths, sizes, etc.
/// - `TypeSpec` - Type specifiers for complex attributes supporting named types
/// - `Identifiers` - Lists of element identifiers (used in note `on` attribute)
/// - `Empty` - Ambiguous empty brackets `[]` that can be interpreted as either
///   empty identifiers or empty type specs depending on context
///
/// **Empty Variant Design:**
/// The `Empty` variant elegantly solves the `[]` ambiguity problem:
/// - Both `as_identifiers()` and `as_type_spec()` return success with empty/default value
/// - Allows `on=[]` (margin note) and `text=[]` (empty type spec) to parse correctly
/// - Parser doesn't need to know the semantic context during parsing
#[derive(Debug, Clone)]
pub enum AttributeValue<'a> {
    String(Spanned<String>),
    Float(Spanned<f32>),
    TypeSpec(TypeSpec<'a>),
    Identifiers(Vec<Spanned<Id>>),
    Empty,
}

impl<'a> PartialEq for AttributeValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AttributeValue::String(s1), AttributeValue::String(s2)) => s1.inner() == s2.inner(),
            (AttributeValue::Float(f1), AttributeValue::Float(f2)) => f1.inner() == f2.inner(),
            (AttributeValue::TypeSpec(t1), AttributeValue::TypeSpec(t2)) => {
                t1.type_name.as_ref().map(|s| s.inner()) == t2.type_name.as_ref().map(|s| s.inner())
                    && t1.attributes == t2.attributes
            }
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
            AttributeValue::TypeSpec(type_spec) => {
                write!(f, "{}", type_spec)
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
            AttributeValue::TypeSpec(type_spec) => type_spec.span(),
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

    /// Extract a type spec, returning an error if this is not a type spec value
    pub fn as_type_spec(&self) -> Result<&TypeSpec<'a>, &'static str> {
        match self {
            AttributeValue::TypeSpec(type_spec) => Ok(type_spec),
            AttributeValue::Empty => Ok(&EMPTY_TYPE_SPEC),
            _ => Err("Expected type spec"),
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

impl<'a> fmt::Display for Attribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

/// Type Definition - declares a new type name as an alias with attributes
#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: Spanned<Id>,
    pub type_spec: TypeSpec<'a>,
}

impl TypeDefinition<'_> {
    pub fn span(&self) -> Span {
        self.name.span().union(self.type_spec.span())
    }
}

/// The kind of a diagram: component or sequence.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DiagramKind {
    Component,
    Sequence,
}

impl fmt::Display for DiagramKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagramKind::Component => write!(f, "component"),
            DiagramKind::Sequence => write!(f, "sequence"),
        }
    }
}

#[derive(Debug)]
pub struct Diagram<'a> {
    pub kind: Spanned<DiagramKind>,
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

/// Fragment block
///
/// **Fields:**
/// - `operation` - The fragment operation/title as a string literal
/// - `type_spec` - Optional type specification with attributes
/// - `sections` - One or more fragment sections containing elements
#[derive(Debug)]
pub struct Fragment<'a> {
    pub operation: Spanned<String>,
    pub type_spec: TypeSpec<'a>,
    pub sections: Vec<FragmentSection<'a>>,
}

impl Fragment<'_> {
    pub fn span(&self) -> Span {
        let span = self.operation.span().union(self.type_spec.span());
        self.sections
            .iter()
            .map(|section| section.span())
            .fold(span, |acc, s| acc.union(s))
    }
}

/// AST node representing a note element
///
/// **Fields:**
/// - `type_spec` - Optional type specification with attributes for positioning, styling, and attachment
/// - `content` - The note text as a string literal (supports escape sequences)
#[derive(Debug)]
pub struct Note<'a> {
    pub type_spec: TypeSpec<'a>,
    pub content: Spanned<String>,
}

impl Note<'_> {
    pub fn span(&self) -> Span {
        self.content.span().union(self.type_spec.span())
    }
}

#[derive(Debug)]
pub enum Element<'a> {
    Component {
        name: Spanned<Id>,
        display_name: Option<Spanned<String>>,
        type_spec: TypeSpec<'a>,
        nested_elements: Vec<Element<'a>>,
    },
    Relation {
        source: Spanned<Id>,
        target: Spanned<Id>,
        relation_type: Spanned<&'a str>,
        type_spec: TypeSpec<'a>,
        label: Option<Spanned<String>>,
    },
    Diagram(Diagram<'a>),
    Fragment(Fragment<'a>),
    ActivateBlock {
        component: Spanned<Id>,
        type_spec: TypeSpec<'a>,
        elements: Vec<Element<'a>>,
    },
    Activate {
        component: Spanned<Id>,
        type_spec: TypeSpec<'a>,
    },
    /// Explicit deactivation of a component
    Deactivate {
        component: Spanned<Id>,
    },
    /// Alt/else block (sugar syntax for fragment with "alt" operation)
    AltElseBlock {
        keyword_span: Span,
        type_spec: TypeSpec<'a>,
        sections: Vec<FragmentSection<'a>>,
    },
    /// Opt block (sugar syntax for fragment with "opt" operation)
    OptBlock {
        keyword_span: Span,
        type_spec: TypeSpec<'a>,
        section: FragmentSection<'a>,
    },
    /// Loop block (sugar syntax for fragment with "loop" operation)
    LoopBlock {
        keyword_span: Span,
        type_spec: TypeSpec<'a>,
        section: FragmentSection<'a>,
    },
    /// Par block (sugar syntax for fragment with "par" operation)
    ParBlock {
        keyword_span: Span,
        type_spec: TypeSpec<'a>,
        sections: Vec<FragmentSection<'a>>,
    },
    /// Break block (sugar syntax for fragment with "break" operation)
    BreakBlock {
        keyword_span: Span,
        type_spec: TypeSpec<'a>,
        section: FragmentSection<'a>,
    },
    /// Critical block (sugar syntax for fragment with "critical" operation)
    CriticalBlock {
        keyword_span: Span,
        type_spec: TypeSpec<'a>,
        section: FragmentSection<'a>,
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
                type_spec,
                nested_elements,
            } => {
                let span = name.span().union(type_spec.span());

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
                    .union(relation_type.span())
                    .union(type_spec.span());

                if let Some(label) = label {
                    span = span.union(label.span());
                }

                span
            }
            Element::Diagram(diagram) => diagram.span(),
            Element::Fragment(fragment) => fragment.span(),
            Element::ActivateBlock {
                component,
                type_spec,
                elements,
            } => {
                let span = component.span().union(type_spec.span());
                elements
                    .iter()
                    .map(|elem| elem.span())
                    .fold(span, |acc, s| acc.union(s))
            }
            Element::Activate {
                component,
                type_spec,
            } => component.span().union(type_spec.span()),
            Element::Deactivate { component } => component.span(),

            // Fragment sugar syntax: multiple sections
            Element::AltElseBlock {
                keyword_span,
                type_spec,
                sections,
            }
            | Element::ParBlock {
                keyword_span,
                type_spec,
                sections,
            } => {
                let mut span = (*keyword_span).union(type_spec.span());
                for section in sections {
                    span = span.union(section.span());
                }
                span
            }

            // Fragment sugar syntax: single section
            Element::OptBlock {
                keyword_span,
                type_spec,
                section,
            }
            | Element::LoopBlock {
                keyword_span,
                type_spec,
                section,
            }
            | Element::BreakBlock {
                keyword_span,
                type_spec,
                section,
            }
            | Element::CriticalBlock {
                keyword_span,
                type_spec,
                section,
            } => (*keyword_span)
                .union(type_spec.span())
                .union(section.span()),

            Element::Note(note) => note.span(),
        }
    }
}

impl Attribute<'_> {
    pub fn span(&self) -> Span {
        self.name.span().union(self.value.span())
    }
}
