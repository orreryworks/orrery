use super::parser_types;
use crate::ast::span::Spanned;
use crate::{color::Color, draw, error::ElaborationDiagnosticError};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeId(String);

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: TypeId,
    pub value: String,
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
    pub arrow_direction: draw::ArrowDirection,
    label: Option<String>,
    type_definition: Rc<TypeDefinition>,
}

impl Relation {
    pub fn new(
        source: TypeId,
        target: TypeId,
        arrow_direction: draw::ArrowDirection,
        label: Option<String>,
        type_definition: Rc<TypeDefinition>,
    ) -> Self {
        Self {
            source,
            target,
            arrow_direction,
            label,
            type_definition,
        }
    }

    pub fn text(&self) -> Option<draw::Text> {
        self.label.as_ref().map(|label| {
            draw::Text::new(
                Rc::clone(&self.type_definition.text_definition),
                label.clone(),
            )
        })
    }

    pub fn clone_arrow_definition(&self) -> Rc<draw::ArrowDefinition> {
        Rc::clone(
            self.type_definition
                .arrow_definition()
                .expect("Type definition must have an arrow definition"),
        )
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
pub enum DrawDefinition {
    Shape(Rc<dyn draw::ShapeDefinition>),
    Arrow(Rc<draw::ArrowDefinition>),
}

#[derive(Debug)]
pub struct TypeDefinition {
    pub id: TypeId,
    pub text_definition: Rc<draw::TextDefinition>,
    pub draw_definition: DrawDefinition,
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

impl TypeDefinition {
    fn new(
        id: TypeId,
        text_definition: draw::TextDefinition,
        draw_definition: DrawDefinition,
    ) -> Self {
        Self {
            id,
            text_definition: Rc::new(text_definition),
            draw_definition,
        }
    }

    pub fn new_shape(
        id: TypeId,
        text_definition: draw::TextDefinition,
        shape_definition: Box<dyn draw::ShapeDefinition>,
    ) -> Self {
        Self::new(
            id,
            text_definition,
            DrawDefinition::Shape(Rc::from(shape_definition)),
        )
    }

    pub fn new_arrow(
        id: TypeId,
        text_definition: draw::TextDefinition,
        arrow_definition: draw::ArrowDefinition,
    ) -> Self {
        Self::new(
            id,
            text_definition,
            DrawDefinition::Arrow(Rc::from(arrow_definition)),
        )
    }

    pub fn shape_definition(&self) -> Result<&Rc<dyn draw::ShapeDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Shape(shape) => Ok(shape),
            DrawDefinition::Arrow(_) => Err(format!(
                "Type '{}' is an arrow type, not a shape type",
                self.id
            )),
        }
    }

    pub fn arrow_definition(&self) -> Result<&Rc<draw::ArrowDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Arrow(arrow) => Ok(arrow),
            DrawDefinition::Shape(_) => Err(format!(
                "Type '{}' is a shape type, not an arrow type",
                self.id
            )),
        }
    }

    pub fn from_base(
        id: TypeId,
        base: &Self,
        attributes: &[Spanned<parser_types::Attribute>],
    ) -> Result<Self, ElaborationDiagnosticError> {
        let mut text_def = (*base.text_definition).clone();

        match &base.draw_definition {
            DrawDefinition::Shape(shape_def) => {
                let mut new_shape_def = shape_def.clone_box();

                // Process shape attributes
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
                            new_shape_def.set_fill_color(Some(val)).map_err(|err| {
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
                            new_shape_def.set_line_color(val).map_err(|err| {
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
                                    format!("Invalid line_width value '{value}'"),
                                    &attr,
                                    "invalid number",
                                    Some("Width must be a positive number".to_string()),
                                )
                            })?;
                            new_shape_def.set_line_width(val).map_err(|err| {
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
                                    format!("Invalid rounded value '{value}'"),
                                    &attr,
                                    "invalid number",
                                    Some("Rounded value must be a positive integer".to_string()),
                                )
                            })?;
                            new_shape_def.set_rounded(val).map_err(|err| {
                                ElaborationDiagnosticError::from_spanned(
                                    err.to_string(),
                                    &attr,
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "font_size" => {
                            let val = value.parse::<u16>().map_err(|_| {
                                ElaborationDiagnosticError::from_spanned(
                                    format!("Invalid font_size value '{value}'"),
                                    &attr,
                                    "invalid number",
                                    Some("Font size must be a positive integer".to_string()),
                                )
                            })?;
                            text_def.set_font_size(val);
                        }
                        _ => {
                            return Err(ElaborationDiagnosticError::from_spanned(
                                format!("Unknown shape attribute '{name}'"),
                                &attr,
                                "unknown attribute",
                                Some(
                                    "Valid shape attributes are: fill_color, line_color, line_width, rounded, font_size"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_shape(id, text_def, new_shape_def))
            }
            DrawDefinition::Arrow(arrow_def) => {
                let mut new_arrow_def = (**arrow_def).clone();

                // Process arrow attributes
                for attr in Attribute::new_from_parser(attributes) {
                    let name = attr.name.0.as_str();
                    let value = attr.value.as_str();

                    match name {
                        "color" => {
                            let val = Color::new(value).map_err(|err| {
                                ElaborationDiagnosticError::from_spanned(
                                    format!("Invalid color '{value}': {err}"),
                                    &attr,
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_arrow_def.set_color(val);
                        }
                        "width" => {
                            let val = value.parse::<usize>().map_err(|_| {
                                ElaborationDiagnosticError::from_spanned(
                                    format!("Invalid width value '{value}'"),
                                    &attr,
                                    "invalid number",
                                    Some("Width must be a positive number".to_string()),
                                )
                            })?;
                            new_arrow_def.set_width(val);
                        }
                        "style" => {
                            let val = draw::ArrowStyle::from_str(value).map_err(|_| {
                                ElaborationDiagnosticError::from_spanned(
                                    format!("Invalid arrow style '{value}'"),
                                    &attr,
                                    "invalid style",
                                    Some(
                                        "Arrow style must be 'straight', 'curved', or 'orthogonal'"
                                            .to_string(),
                                    ),
                                )
                            })?;
                            new_arrow_def.set_style(val);
                        }
                        "font_size" => {
                            let val = value.parse::<u16>().map_err(|_| {
                                ElaborationDiagnosticError::from_spanned(
                                    format!("Invalid font_size value '{value}'"),
                                    &attr,
                                    "invalid number",
                                    Some("Font size must be a positive integer".to_string()),
                                )
                            })?;
                            text_def.set_font_size(val);
                        }
                        _ => {
                            return Err(ElaborationDiagnosticError::from_spanned(
                                format!("Unknown arrow attribute '{name}'"),
                                &attr,
                                "unknown attribute",
                                Some(
                                    "Valid arrow attributes are: color, width, style, font_size"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_arrow(id, text_def, new_arrow_def))
            }
        }
    }

    pub fn default_arrow_definition() -> Rc<Self> {
        Rc::from(Self::new_arrow(
            TypeId::from_name("Arrow"),
            draw::TextDefinition::new(),
            draw::ArrowDefinition::new(),
        ))
    }

    pub fn defaults(default_arrow_definition: &Rc<Self>) -> Vec<Rc<Self>> {
        vec![
            Rc::new(Self::new_shape(
                TypeId::from_name("Rectangle"),
                draw::TextDefinition::new(),
                Box::new(draw::RectangleDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Oval"),
                draw::TextDefinition::new(),
                Box::new(draw::OvalDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Component"),
                draw::TextDefinition::new(),
                Box::new(draw::ComponentDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Boundary"),
                draw::TextDefinition::new(),
                Box::new(draw::BoundaryDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Actor"),
                draw::TextDefinition::new(),
                Box::new(draw::ActorDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Entity"),
                draw::TextDefinition::new(),
                Box::new(draw::EntityDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Control"),
                draw::TextDefinition::new(),
                Box::new(draw::ControlDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                TypeId::from_name("Interface"),
                draw::TextDefinition::new(),
                Box::new(draw::InterfaceDefinition::new()),
            )),
            Rc::clone(default_arrow_definition),
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
