use super::parser_types;
use crate::ast::span::Spanned;
use crate::{color::Color, draw, error::ElaborationDiagnosticError};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Ref, RefCell},
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
};

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
    pub display_name: Option<String>,
    pub block: Block,
    pub type_definition: Rc<TypeDefinition>,
}

impl Node {
    /// Returns the display text for this node
    /// Uses display_name if present, otherwise falls back to the identifier name
    pub fn display_text(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub source: TypeId,
    pub target: TypeId,
    pub relation_type: RelationType,
    label: Option<String>,
    arrow_definition: Rc<RefCell<draw::ArrowDefinition>>,
    text_definition: Rc<RefCell<draw::TextDefinition>>,
}

impl Relation {
    pub fn new(
        source: TypeId,
        target: TypeId,
        relation_type: RelationType,
        label: Option<String>,
        arrow_definition: Rc<RefCell<draw::ArrowDefinition>>,
        text_definition: Rc<RefCell<draw::TextDefinition>>,
    ) -> Self {
        Self {
            source,
            target,
            relation_type,
            label,
            arrow_definition,
            text_definition,
        }
    }

    pub fn text(&self) -> Option<draw::Text> {
        self.label
            .as_ref()
            .map(|label| draw::Text::new(Rc::clone(&self.text_definition), label.clone()))
    }

    /// Gets a reference to the arrow definition
    pub fn arrow_definition(&self) -> Ref<draw::ArrowDefinition> {
        self.arrow_definition.borrow()
    }
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

#[derive(Debug)]
pub struct TypeDefinition {
    pub id: TypeId,
    pub text_definition: Rc<RefCell<draw::TextDefinition>>,
    pub shape_definition: Rc<RefCell<dyn draw::ShapeDefinition>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutEngine {
    Basic,
    Force,
    Sugiyama,
}

impl FromStr for LayoutEngine {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Self::Basic),
            "force" => Ok(Self::Force),
            "sugiyama" => Ok(Self::Sugiyama),
            _ => Err("Unsupported layout engine"),
        }
    }
}

impl From<LayoutEngine> for &'static str {
    fn from(val: LayoutEngine) -> Self {
        match val {
            LayoutEngine::Basic => "basic",
            LayoutEngine::Force => "force",
            LayoutEngine::Sugiyama => "sugiyama",
        }
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::Basic
    }
}

impl Display for LayoutEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s: &'static str = (*self).into();
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub struct Diagram {
    pub kind: DiagramKind,
    pub scope: Scope,
    pub layout_engine: LayoutEngine,
    pub background_color: Option<Color>,
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

impl Clone for TypeDefinition {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            text_definition: Rc::new(RefCell::new(self.text_definition.borrow().clone())),
            shape_definition: self.shape_definition.borrow().clone_new_rc(),
        }
    }
}

impl TypeDefinition {
    pub fn from_base(
        id: TypeId,
        base: &Self,
        attributes: &[Spanned<parser_types::Attribute>],
    ) -> Result<Self, ElaborationDiagnosticError> {
        let mut type_def = base.clone(); // TODO: custom clone.
        type_def.id = id;
        // Process attributes with descriptive errors
        {
            let mut shape_def = type_def.shape_definition.borrow_mut();
            for attr in Attribute::new_from_parser(attributes) {
                let name = attr.name.0.as_str();
                let value = attr.value.as_str();

                match name {
                    "fill_color" => {
                        let val = Color::new(value).map_err(|err| {
                            ElaborationDiagnosticError::from_spanned(
                                format!("Invalid fill_color '{value}': {err}"),
                                &attr,
                                "invalid color",
                                Some("Use a CSS color".to_string()),
                            )
                        })?;
                        // Access the value from the RefCell
                        shape_def.set_fill_color(Some(val)).map_err(|err| {
                            ElaborationDiagnosticError::from_spanned(
                                err.to_string(),
                                &attr,
                                "unsupported attribute",
                                None,
                            )
                        })?;
                    }
                    "line_color" => {
                        let val = Color::new(value).map_err(|err| {
                            ElaborationDiagnosticError::from_spanned(
                                format!("Invalid line_color '{value}': {err}"),
                                &attr,
                                "invalid color",
                                Some("Use a CSS color".to_string()),
                            )
                        })?;
                        shape_def.set_line_color(val).map_err(|err| {
                            ElaborationDiagnosticError::from_spanned(
                                err.to_string(),
                                &attr,
                                "unsupported attribute",
                                None,
                            )
                        })?;
                    }
                    "line_width" => {
                        let val = value.parse::<usize>().map_err(|_| {
                            ElaborationDiagnosticError::from_spanned(
                                format!("Invalid line_width '{value}'"),
                                &attr,
                                "invalid positive integer",
                                Some("Use a positive integer".to_string()),
                            )
                        })?;
                        shape_def.set_line_width(val).map_err(|err| {
                            ElaborationDiagnosticError::from_spanned(
                                err.to_string(),
                                &attr,
                                "unsupported attribute",
                                None,
                            )
                        })?;
                    }
                    "rounded" => {
                        let val = value.parse::<usize>().map_err(|_| {
                            ElaborationDiagnosticError::from_spanned(
                                format!("Invalid rounded '{value}'"),
                                &attr,
                                "invalid positive integer",
                                Some("Use a positive integer".to_string()),
                            )
                        })?;
                        shape_def.set_rounded(val).map_err(|err| {
                            ElaborationDiagnosticError::from_spanned(
                                err.to_string(),
                                &attr,
                                "unsupported attribute",
                                None,
                            )
                        })?;
                    }
                    "font_size" => {
                        let mut text_def = type_def.text_definition.borrow_mut();
                        let val = value.parse::<u16>().map_err(|_| {
                            ElaborationDiagnosticError::from_spanned(
                                format!("Invalid font_size '{value}'"),
                                &attr,
                                "invalid positive integer",
                                Some("Use a positive integer".to_string()),
                            )
                        })?;
                        text_def.set_font_size(val);
                    }
                    _ => {
                        return Err(ElaborationDiagnosticError::from_spanned(
                            format!("Unsupported type definition attribute '{}'", name),
                            &attr,
                            "unsupported attribute",
                            None,
                        ));
                    }
                }
            }
        }

        Ok(type_def)
    }

    pub fn defaults() -> Vec<Rc<Self>> {
        vec![
            Rc::new(Self {
                id: TypeId::from_name("Rectangle"),
                text_definition: Rc::new(RefCell::new(draw::TextDefinition::new())),
                shape_definition: Rc::new(RefCell::new(draw::RectangleDefinition::new()))
                    as Rc<RefCell<dyn draw::ShapeDefinition>>,
            }),
            Rc::new(Self {
                id: TypeId::from_name("Oval"),
                text_definition: Rc::new(RefCell::new(draw::TextDefinition::new())),
                shape_definition: Rc::new(RefCell::new(draw::OvalDefinition::new()))
                    as Rc<RefCell<dyn draw::ShapeDefinition>>,
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
