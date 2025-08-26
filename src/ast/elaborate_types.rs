use crate::{
    ast::parser_types, color::Color, draw, error::ElaborationDiagnosticError, geometry::Insets,
    identifier::Id,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
};

/// A diagram node (component/participant) with visual definition and nested content.
#[derive(Debug, Clone)]
pub struct Node {
    id: Id,
    name: String,
    display_name: Option<String>,
    block: Block,
    type_definition: Rc<TypeDefinition>,
}

impl Node {
    /// Create a new Node.
    pub fn new(
        id: Id,
        name: String,
        display_name: Option<String>,
        block: Block,
        type_definition: Rc<TypeDefinition>,
    ) -> Self {
        Self {
            id,
            name,
            display_name,
            block,
            type_definition,
        }
    }

    /// Get the node identifier.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Borrow the node's content block.
    pub fn block(&self) -> &Block {
        &self.block
    }

    /// Borrow the node's type definition.
    pub fn type_definition(&self) -> &TypeDefinition {
        &self.type_definition
    }

    /// Returns the display text for this node
    /// Uses display_name if present, otherwise falls back to the identifier name
    pub fn display_text(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }
}

/// A relation (edge/message) between two nodes, carrying direction, text, and style.
#[derive(Debug, Clone)]
pub struct Relation {
    source: Id,
    target: Id,
    arrow_direction: draw::ArrowDirection,
    label: Option<String>,
    type_definition: Rc<TypeDefinition>,
}

impl Relation {
    /// Create a new Relation between two node Ids with an optional label
    /// and a type definition that determines appearance.
    pub fn new(
        source: Id,
        target: Id,
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

    /// Build a Text drawable for the relation's label using its text definition, if a label exists.
    pub fn text(&self) -> Option<draw::Text> {
        self.label.as_ref().map(|label| {
            draw::Text::new(
                Rc::clone(self.type_definition.text_definition_rc()),
                label.clone(),
            )
        })
    }

    /// Clone the underlying ArrowDefinition Rc for rendering this relation.
    // TODO: Consider removing clone from here?!
    pub fn clone_arrow_definition(&self) -> Rc<draw::ArrowDefinition> {
        Rc::clone(
            self.type_definition
                .arrow_definition_rc()
                .expect("Type definition must have an arrow definition"),
        )
    }

    /// Get the source node Id of this relation.
    pub fn source(&self) -> Id {
        self.source
    }

    /// Get the target node Id of this relation.
    pub fn target(&self) -> Id {
        self.target
    }

    /// Get the arrow direction for this relation.
    pub fn arrow_direction(&self) -> draw::ArrowDirection {
        self.arrow_direction
    }
}

/// Top-level elaborated element within a scope.
/// Represents nodes, relations, and activation events in AST order.
#[derive(Debug, Clone)]
pub enum Element {
    Node(Node),
    Relation(Relation),
    Activate(Id),
    Deactivate(Id),
    Fragment(Fragment),
}

/// Represents a fragment block in a sequence diagram.
///
/// Fragments group related interactions into labeled sections, helping structure
/// complex message flows and illustrate alternatives or phases.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// The operation string (e.g., "alt", "opt", "loop", "par")
    operation: String,
    /// The sections within this fragment
    sections: Vec<FragmentSection>,
}

impl Fragment {
    /// Create a new Fragment with the given operation and sections.
    pub fn new(operation: String, sections: Vec<FragmentSection>) -> Self {
        Self {
            operation,
            sections,
        }
    }

    /// Get the operation string for this fragment.
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Get the sections in this fragment.
    pub fn sections(&self) -> &[FragmentSection] {
        &self.sections
    }
}

/// Represents a section within a fragment.
///
/// Each section can have an optional title and contains a sequence of elements
/// that represent one phase within the fragment.
#[derive(Debug, Clone)]
pub struct FragmentSection {
    /// Optional title for this section (e.g., "successful login", "failed login")
    title: Option<String>,
    /// Elements contained in this section
    elements: Vec<Element>,
}

impl FragmentSection {
    /// Create a new FragmentSection with optional title and elements.
    pub fn new(title: Option<String>, elements: Vec<Element>) -> Self {
        Self { title, elements }
    }

    /// Get the optional title of this section.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Get the elements in this section.
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }
}

/// A containment scope that groups a sequence of elements at the same nesting level.
#[derive(Debug, Default, Clone)]
pub struct Scope {
    elements: Vec<Element>,
}

impl Scope {
    /// Create a new Scope from a list of elements.
    pub fn new(elements: Vec<Element>) -> Self {
        Self { elements }
    }

    /// Borrow the elements contained in this scope.
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }
}

/// The kind of a diagram: component or sequence.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DiagramKind {
    Component,
    Sequence,
}

/// Unified drawing definition for types: either a shape or an arrow.
#[derive(Debug)]
pub enum DrawDefinition {
    Shape(Rc<dyn draw::ShapeDefinition>),
    Arrow(Rc<draw::ArrowDefinition>),
}

/// A concrete, elaborated type with text styling and drawing definition.
#[derive(Debug)]
pub struct TypeDefinition {
    id: Id,
    text_definition: Rc<draw::TextDefinition>,
    draw_definition: DrawDefinition,
}

/// Available layout engines controlling automatic positioning for diagrams.
/// Names match external configuration strings (snake_case).
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

/// A fully elaborated diagram, with kind, content scope, layout engine, and optional background.
#[derive(Debug, Clone)]
pub struct Diagram {
    kind: DiagramKind,
    scope: Scope,
    layout_engine: LayoutEngine,
    background_color: Option<Color>,
}

impl Diagram {
    /// Create a new Diagram with its kind, scope, layout engine, and optional background color.
    pub fn new(
        kind: DiagramKind,
        scope: Scope,
        layout_engine: LayoutEngine,
        background_color: Option<Color>,
    ) -> Self {
        Self {
            kind,
            scope,
            layout_engine,
            background_color,
        }
    }

    /// Get the diagram kind.
    pub fn kind(&self) -> DiagramKind {
        self.kind
    }

    /// Borrow the diagram's top-level scope.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// Get the configured layout engine for this diagram.
    pub fn layout_engine(&self) -> LayoutEngine {
        self.layout_engine
    }

    /// Get the diagram's background color if specified.
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }
}

/// A block wrapper representing empty content, a nested scope, or an embedded diagram.
#[derive(Debug, Clone)]
pub enum Block {
    None,
    Scope(Scope),
    Diagram(Diagram),
}

impl TypeDefinition {
    fn new(id: Id, text_definition: draw::TextDefinition, draw_definition: DrawDefinition) -> Self {
        Self {
            id,
            text_definition: Rc::new(text_definition),
            draw_definition,
        }
    }

    /// Construct a concrete shape type definition from a text definition and a shape definition.
    pub fn new_shape(
        id: Id,
        text_definition: draw::TextDefinition,
        shape_definition: Box<dyn draw::ShapeDefinition>,
    ) -> Self {
        Self::new(
            id,
            text_definition,
            DrawDefinition::Shape(Rc::from(shape_definition)),
        )
    }

    /// Construct a concrete arrow type definition from a text definition and an arrow definition.
    pub fn new_arrow(
        id: Id,
        text_definition: draw::TextDefinition,
        arrow_definition: draw::ArrowDefinition,
    ) -> Self {
        Self::new(
            id,
            text_definition,
            DrawDefinition::Arrow(Rc::from(arrow_definition)),
        )
    }

    /// Get the identifier for this type definition.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Borrow the Rc-backed text definition.
    ///
    /// This returns &Rc<_> so callers can explicitly Rc::clone when they need ownership.
    pub fn text_definition_rc(&self) -> &Rc<draw::TextDefinition> {
        &self.text_definition
    }

    /// Borrow the Rc-backed shape definition if this type is a shape; otherwise returns an error.
    ///
    /// Returning &Rc<_> makes cloning explicit at the call site when needed.
    pub fn shape_definition_rc(&self) -> Result<&Rc<dyn draw::ShapeDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Shape(shape) => Ok(shape),
            DrawDefinition::Arrow(_) => Err(format!(
                "Type '{}' is an arrow type, not a shape type",
                self.id
            )),
        }
    }

    /// Borrow the Rc-backed arrow definition if this type is an arrow; otherwise returns an error.
    ///
    /// Returning &Rc<_> makes cloning explicit at the call site when needed.
    pub fn arrow_definition_rc(&self) -> Result<&Rc<draw::ArrowDefinition>, String> {
        match &self.draw_definition {
            DrawDefinition::Arrow(arrow) => Ok(arrow),
            DrawDefinition::Shape(_) => Err(format!(
                "Type '{}' is a shape type, not an arrow type",
                self.id
            )),
        }
    }
}

/// Extractor for text-related attributes that can be applied to TextDefinition
pub struct TextAttributeExtractor;

impl TextAttributeExtractor {
    /// Extract and apply text-related attributes to a TextDefinition from a group of nested attributes.
    ///
    /// Returns `Ok(())` if all attributes were processed successfully,
    /// `Err(...)` if any attribute has an invalid value or is not a valid text attribute.
    fn extract_text_attributes(
        text_def: &mut draw::TextDefinition,
        attrs: &[parser_types::Attribute],
    ) -> Result<(), ElaborationDiagnosticError> {
        for attr in attrs {
            Self::extract_single_attribute(text_def, attr)?;
        }
        Ok(())
    }

    /// Extract and apply a single text-related attribute to a TextDefinition.
    ///
    /// Returns `Ok(())` if the attribute was processed successfully,
    /// `Err(...)` if the attribute has an invalid value or is not a valid text attribute.
    fn extract_single_attribute(
        text_def: &mut draw::TextDefinition,
        attr: &parser_types::Attribute,
    ) -> Result<(), ElaborationDiagnosticError> {
        let name = attr.name.inner();
        let value = &attr.value;

        match *name {
            "font_size" => {
                let val = value.as_u16().map_err(|_| {
                    ElaborationDiagnosticError::from_span(
                        format!("Invalid font_size value '{value}'"),
                        attr.span(),
                        "invalid number",
                        Some("Font size must be a positive integer".to_string()),
                    )
                })?;
                text_def.set_font_size(val);
                Ok(())
            }
            "font_family" => {
                text_def.set_font_family(value.as_str().map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid font family",
                        Some("Font family must be a string value".to_string()),
                    )
                })?);
                Ok(())
            }
            "background_color" => {
                let val = Color::new(value.as_str().map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid color value",
                        Some("Color values must be strings".to_string()),
                    )
                })?)
                .map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        format!("Invalid background_color: {err}"),
                        attr.span(),
                        "invalid color",
                        Some("Use a CSS color".to_string()),
                    )
                })?;
                text_def.set_background_color(Some(val));
                Ok(())
            }
            "padding" => {
                let val = value.as_float().map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        format!("Invalid padding value: {err}"),
                        attr.span(),
                        "invalid number",
                        Some("Text padding must be a positive number".to_string()),
                    )
                })?;
                text_def.set_padding(Insets::uniform(val));
                Ok(())
            }
            "color" => {
                let val = Color::new(value.as_str().map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid color value",
                        Some("Color values must be strings".to_string()),
                    )
                })?)
                .map_err(|err| {
                    ElaborationDiagnosticError::from_span(
                        format!("Invalid color: {err}"),
                        attr.span(),
                        "invalid color",
                        Some("Use a CSS color".to_string()),
                    )
                })?;
                text_def.set_color(Some(val));
                Ok(())
            }
            name => Err(ElaborationDiagnosticError::from_span(
                format!("Unknown text attribute '{name}'"),
                attr.span(),
                "unknown text attribute",
                Some(
                    "Valid text attributes are: font_size, font_family, background_color, padding, color"
                        .to_string(),
                ),
            )),
        }
    }
}

impl TypeDefinition {
    pub fn from_base(
        id: Id,
        base: &Self,
        attributes: &[parser_types::Attribute],
    ) -> Result<Self, ElaborationDiagnosticError> {
        let mut text_def = (*base.text_definition).clone();

        match &base.draw_definition {
            DrawDefinition::Shape(shape_def) => {
                let mut new_shape_def = shape_def.clone_box();

                // Process shape attributes
                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "fill_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid fill_color '{value}': {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_shape_def.set_fill_color(Some(val)).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "line_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid line_color: {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_shape_def.set_line_color(val).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "line_width" => {
                            let val = value.as_usize().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid line_width value: {err}"),
                                    attr.span(),
                                    "invalid number",
                                    Some("Width must be a positive number".to_string()),
                                )
                            })?;
                            new_shape_def.set_line_width(val).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "rounded" => {
                            let val = value.as_usize().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid rounded value: {err}"),
                                    attr.span(),
                                    "invalid number",
                                    Some("Rounded must be a positive number".to_string()),
                                )
                            })?;
                            new_shape_def.set_rounded(val).map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "text" => {
                            // Handle nested text attributes
                            let nested_attrs = value.as_attributes().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid text attribute value",
                                    Some("Text attribute must contain nested attributes like [font_size=12, padding=6.5]".to_string()),
                                )
                            })?;

                            // Process all nested text attributes
                            TextAttributeExtractor::extract_text_attributes(
                                &mut text_def,
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(ElaborationDiagnosticError::from_span(
                                format!("Unknown shape attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid shape attributes are: fill_color, line_color, line_width, rounded, text=[...]"
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
                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid color: {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_arrow_def.set_color(val);
                        }
                        "width" => {
                            let val = value.as_usize().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    format!("Invalid width value: {err}"),
                                    attr.span(),
                                    "invalid number",
                                    Some("Width must be a positive number".to_string()),
                                )
                            })?;
                            new_arrow_def.set_width(val);
                        }
                        "style" => {
                            let val =
                                draw::ArrowStyle::from_str(value.as_str().map_err(|err| {
                                    ElaborationDiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid style value",
                                        Some("Style values must be strings".to_string()),
                                    )
                                })?)
                                .map_err(|_| {
                                    ElaborationDiagnosticError::from_span(
                                    "Invalid arrow style".to_string(),
                                    attr.span(),
                                    "invalid style",
                                    Some(
                                        "Arrow style must be 'straight', 'curved', or 'orthogonal'"
                                            .to_string(),
                                    ),
                                )
                                })?;
                            new_arrow_def.set_style(val);
                        }
                        "text" => {
                            // Handle nested text attributes
                            let nested_attrs = value.as_attributes().map_err(|err| {
                                ElaborationDiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid text attribute value",
                                    Some("Text attribute must contain nested attributes like [font_size=12, padding=6.5]".to_string()),
                                )
                            })?;

                            // Process all nested text attributes
                            TextAttributeExtractor::extract_text_attributes(
                                &mut text_def,
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(ElaborationDiagnosticError::from_span(
                                format!("Unknown arrow attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid arrow attributes are: color, width, style, text=[...]"
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
            Id::new("Arrow"),
            draw::TextDefinition::new(),
            draw::ArrowDefinition::new(),
        ))
    }

    pub fn defaults(default_arrow_definition: &Rc<Self>) -> Vec<Rc<Self>> {
        vec![
            Rc::new(Self::new_shape(
                Id::new("Rectangle"),
                draw::TextDefinition::new(),
                Box::new(draw::RectangleDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                Id::new("Oval"),
                draw::TextDefinition::new(),
                Box::new(draw::OvalDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                Id::new("Component"),
                draw::TextDefinition::new(),
                Box::new(draw::ComponentDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                Id::new("Boundary"),
                draw::TextDefinition::new(),
                Box::new(draw::BoundaryDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                Id::new("Actor"),
                draw::TextDefinition::new(),
                Box::new(draw::ActorDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                Id::new("Entity"),
                draw::TextDefinition::new(),
                Box::new(draw::EntityDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                Id::new("Control"),
                draw::TextDefinition::new(),
                Box::new(draw::ControlDefinition::new()),
            )),
            Rc::new(Self::new_shape(
                Id::new("Interface"),
                draw::TextDefinition::new(),
                Box::new(draw::InterfaceDefinition::new()),
            )),
            Rc::clone(default_arrow_definition),
        ]
    }
}

#[cfg(test)]
mod elaborate_tests {
    use super::*;
    use crate::ast::span::{Span, Spanned};

    fn create_test_attribute(
        name: &'static str,
        value: parser_types::AttributeValue<'static>,
    ) -> parser_types::Attribute<'static> {
        parser_types::Attribute {
            name: Spanned::new(name, Span::default()),
            value,
        }
    }

    fn create_string_value(s: &str) -> parser_types::AttributeValue<'static> {
        parser_types::AttributeValue::String(Spanned::new(s.to_string(), Span::default()))
    }

    fn create_float_value(f: f32) -> parser_types::AttributeValue<'static> {
        parser_types::AttributeValue::Float(Spanned::new(f, Span::default()))
    }

    #[test]
    fn test_text_attribute_extractor_all_attributes() {
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![
            create_test_attribute("font_size", create_float_value(16.0)),
            create_test_attribute("font_family", create_string_value("Helvetica")),
            create_test_attribute("background_color", create_string_value("red")),
            create_test_attribute("padding", create_float_value(5.0)),
            create_test_attribute("color", create_string_value("blue")),
        ];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_attribute_extractor_color_attribute() {
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute("color", create_string_value("red"))];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_ok());

        // Test with invalid color value (should be string)
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute("color", create_float_value(255.0))];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());
    }

    #[test]
    fn test_text_attribute_extractor_empty_attributes() {
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![];

        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_attribute_extractor_invalid_attribute_name() {
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![
            create_test_attribute("font_size", create_float_value(16.0)),
            create_test_attribute("invalid_attribute", create_string_value("test")),
        ];

        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());

        if let Err(error) = result {
            let error_message = error.to_string();
            assert!(error_message.contains("Unknown text attribute 'invalid_attribute'"));
        }
    }

    #[test]
    fn test_text_attribute_extractor_invalid_value_types() {
        // Test font_size with string value (should be float)
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute(
            "font_size",
            create_string_value("not_a_number"),
        )];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());

        // Test font_family with float value (should be string)
        let mut text_def = draw::TextDefinition::new();
        let attributes = vec![create_test_attribute(
            "font_family",
            create_float_value(123.0),
        )];
        let result = TextAttributeExtractor::extract_text_attributes(&mut text_def, &attributes);
        assert!(result.is_err());
    }

    #[test]
    fn test_type_definition_with_text_attributes() {
        // Create a base rectangle type
        let base_id = Id::new("Rectangle");
        let base_text_def = draw::TextDefinition::new();
        let base_shape_def = Box::new(draw::RectangleDefinition::new());
        let base_type = TypeDefinition::new_shape(base_id, base_text_def, base_shape_def);

        // Create attributes including text group
        let text_attrs = vec![
            create_test_attribute("font_size", create_float_value(14.0)),
            create_test_attribute("font_family", create_string_value("Arial")),
        ];

        let attributes = vec![
            create_test_attribute("fill_color", create_string_value("blue")),
            create_test_attribute("text", parser_types::AttributeValue::Attributes(text_attrs)),
        ];

        // Create new type from base with text attributes
        let new_id = Id::new("StyledRectangle");
        let result = TypeDefinition::from_base(new_id, &base_type, &attributes);

        assert!(result.is_ok());
    }

    #[test]
    fn test_type_definition_text_not_nested_attributes() {
        // Create a base rectangle type
        let base_id = Id::new("Rectangle");
        let base_text_def = draw::TextDefinition::new();
        let base_shape_def = Box::new(draw::RectangleDefinition::new());
        let base_type = TypeDefinition::new_shape(base_id, base_text_def, base_shape_def);

        // Try to use text with string value instead of nested attributes
        let attributes = vec![
            create_test_attribute("fill_color", create_string_value("blue")),
            create_test_attribute(
                "text",
                create_string_value("this_should_be_nested_attributes"),
            ),
        ];

        // This should fail because text must contain nested attributes
        let new_id = Id::new("InvalidTextType");
        let result = TypeDefinition::from_base(new_id, &base_type, &attributes);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_message = format!("{}", error);
            assert!(error_message.contains("Expected nested attributes"));
        }
    }
}
