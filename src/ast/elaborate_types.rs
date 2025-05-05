use super::parser_types;
use crate::ast::span::Spanned;
use crate::{
    color::Color,
    error::ElaborationDiagnosticError,
    shape::{Oval, Rectangle, Shape},
};
use std::{fmt, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeId(String);

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: TypeId,
    pub value: String, // TODO: Can I convert it to str?
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationType {
    Forward,       // ->
    Backward,      // <-
    Bidirectional, // <->
    Plain,         // -
}

impl RelationType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "->" => Self::Forward,
            "<-" => Self::Backward,
            "<->" => Self::Bidirectional,
            "-" => Self::Plain,
            _ => Self::Forward, // Default to forward if unknown
        }
    }

    fn to_string(&self) -> &'static str {
        match self {
            Self::Forward => "->",
            Self::Backward => "<-",
            Self::Bidirectional => "<->",
            Self::Plain => "-",
        }
    }
}
impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: TypeId,
    pub name: String,
    pub block: Block,
    pub type_definition: Rc<TypeDefinition>,
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub source: TypeId,
    pub target: TypeId,
    pub relation_type: RelationType,
    pub color: Color,
    pub width: usize,
}

#[derive(Debug, Clone)]
pub enum Element {
    Node(Node),
    Relation(Relation),
}

#[derive(Debug, Default, Clone)]
pub struct Scope {
    pub elements: Vec<Element>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DiagramKind {
    Component,
    Sequence,
}

#[derive(Clone)]
pub struct TypeDefinition {
    pub id: TypeId,
    pub fill_color: Option<Color>,
    pub line_color: Color,
    pub line_width: usize,
    pub rounded: usize,
    pub font_size: usize,
    pub shape_type: Rc<dyn Shape>,
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub kind: DiagramKind,
    pub scope: Scope,
}

#[derive(Debug, Clone)]
pub enum Block {
    None,
    Scope(Scope),
    Diagram(Diagram),
}

impl Block {
    /// Returns true if this block contains any elements
    pub fn has_nested_blocks(&self) -> bool {
        match self {
            Self::None => false,
            Self::Scope(scope) => !scope.elements.is_empty(),
            Self::Diagram(diagram) => !diagram.scope.elements.is_empty(),
        }
    }
}

impl TypeId {
    /// Creates a `TypeId` from a component name as defined in the diagram
    pub fn from_name(name: &str) -> Self {
        Self(name.to_string())
    }

    /// Creates an internal `TypeId` used for generated types
    /// (e.g., for anonymous type definitions)
    pub fn from_anonymous(idx: usize) -> Self {
        Self(format!("__{idx}"))
    }

    /// Creates a nested ID by combining parent ID and child ID with '::' separator
    pub fn create_nested(&self, child_id: &str) -> Self {
        Self(format!("{}::{}", self.0, child_id))
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Implement Debug manually for TypeDefinition since we can't derive it due to the dyn ShapeType
impl std::fmt::Debug for TypeDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeDefinition")
            .field("id", &self.id)
            .field("fill_color", &self.fill_color)
            .field("line_color", &self.line_color)
            .field("line_width", &self.line_width)
            .field("rounded", &self.rounded)
            .field("font_size", &self.font_size)
            .field("shape_type", &self.shape_type.name())
            .finish()
    }
}

impl TypeDefinition {
    pub fn from_base(
        id: TypeId,
        base: &Self,
        attributes: &[Spanned<parser_types::Attribute>],
    ) -> Result<Self, ElaborationDiagnosticError> {
        let mut type_def = base.clone();
        type_def.id = id;
        // Process attributes with descriptive errors
        for attr in Attribute::new_from_parser(attributes) {
            let name = attr.name.0.as_str();
            let value = attr.value.as_str();

            match name {
                "fill_color" => {
                    type_def.fill_color = Some(Color::new(value).map_err(|err| {
                        ElaborationDiagnosticError::from_spanned(
                            format!("Invalid fill_color '{value}': {err}"),
                            &attr,
                            "invalid color",
                            Some("Use a CSS color".to_string()),
                        )
                    })?);
                }
                "line_color" => {
                    type_def.line_color = Color::new(value).map_err(|err| {
                        ElaborationDiagnosticError::from_spanned(
                            format!("Invalid line_color '{value}': {err}"),
                            &attr,
                            "invalid color",
                            Some("Use a CSS color".to_string()),
                        )
                    })?;
                }
                "line_width" => {
                    type_def.line_width = value.parse::<usize>().map_err(|_| {
                        ElaborationDiagnosticError::from_spanned(
                            format!("Invalid line_width '{value}'"),
                            &attr,
                            "invalid positive integer",
                            Some("Use a positive integer".to_string()),
                        )
                    })?;
                }
                "rounded" => {
                    type_def.rounded = value.parse::<usize>().map_err(|_| {
                        ElaborationDiagnosticError::from_spanned(
                            format!("Invalid rounded '{value}'"),
                            &attr,
                            "invalid positive integer",
                            Some("Use a positive integer".to_string()),
                        )
                    })?;
                }
                "font_size" => {
                    type_def.font_size = value.parse::<usize>().map_err(|_| {
                        ElaborationDiagnosticError::from_spanned(
                            format!("Invalid font_size '{value}'"),
                            &attr,
                            "invalid positive integer",
                            Some("Use a positive integer".to_string()),
                        )
                    })?;
                }
                _ => {
                    // TODO: For unknown attributes, just add them to the list
                    // We could warn about them, but we'll just keep them for now
                }
            }
        }

        Ok(type_def)
    }

    pub fn defaults() -> Vec<Rc<Self>> {
        let black = Color::default();
        vec![
            Rc::new(Self {
                id: TypeId::from_name("Rectangle"),
                fill_color: None,
                line_color: black.clone(),
                line_width: 2,
                rounded: 0,
                font_size: 15,
                shape_type: Rc::new(Rectangle) as Rc<dyn Shape>,
            }),
            Rc::new(Self {
                id: TypeId::from_name("Oval"),
                fill_color: None,
                line_color: black,
                line_width: 2,
                rounded: 0,
                font_size: 15,
                shape_type: Rc::new(Oval) as Rc<dyn Shape>,
            }),
        ]
    }
}

impl Attribute {
    fn new(name: &str, value: &str) -> Self {
        Self {
            name: TypeId::from_name(name),
            value: value.to_string(),
        }
    }

    fn new_from_parser(parser_attrs: &[Spanned<parser_types::Attribute>]) -> Vec<Spanned<Self>> {
        parser_attrs
            .iter()
            .map(|attr| attr.map(|attr| Self::new(&attr.name, &attr.value)))
            .collect()
    }
}
