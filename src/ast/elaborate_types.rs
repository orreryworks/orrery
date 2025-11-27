use std::{
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{
    ast::parser_types,
    color::Color,
    draw,
    error::diagnostic::{DiagnosticError, Result as DiagnosticResult},
    geometry::Insets,
    identifier::Id,
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
    pub fn text(&self) -> Option<draw::Text<'_>> {
        let label = self.label.as_ref()?;
        let arrow_def = self.type_definition.arrow_definition().ok()?;
        let text_def = arrow_def.text();
        Some(draw::Text::new(text_def, label))
    }

    /// Clone the underlying ArrowDefinition Rc for rendering this relation.
    pub fn clone_arrow_definition(&self) -> draw::ArrowDefinition {
        self.type_definition
            .arrow_definition()
            .expect("Type definition must have an arrow definition")
            .clone()
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

/// Alignment for note positioning in diagrams.
///
/// Different diagram types support different alignment values:
/// - Sequence diagrams: Over, Left, Right
/// - Component diagrams: Left, Right, Top, Bottom
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteAlign {
    Over,
    Left,
    Right,
    Top,
    Bottom,
}

impl FromStr for NoteAlign {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "over" => Ok(NoteAlign::Over),
            "left" => Ok(NoteAlign::Left),
            "right" => Ok(NoteAlign::Right),
            "top" => Ok(NoteAlign::Top),
            "bottom" => Ok(NoteAlign::Bottom),
            _ => Err("Invalid alignment value"),
        }
    }
}

/// Represents a note annotation in a diagram.
///
/// Notes provide additional context or documentation without participating
/// in the diagram's structural relationships.
///
/// # Examples
///
/// ```
/// # use filament::ast::{Note, NoteAlign};
/// # use filament::identifier::Id;
/// # use filament::draw::NoteDefinition;
/// #
/// // Create a margin note (not attached to any elements)
/// let note = Note::new(
///     vec![],  // Empty vec = margin note
///     NoteAlign::Over,
///     "This is a note".to_string(),
///     NoteDefinition::new(),
/// );
/// assert_eq!(note.on().len(), 0);
/// assert_eq!(note.content(), "This is a note");
///
/// // Create a note attached to an element
/// let attached_note = Note::new(
///     vec![Id::new("server")],
///     NoteAlign::Right,
///     "Server note".to_string(),
///     NoteDefinition::new(),
/// );
/// assert_eq!(attached_note.on().len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct Note {
    /// Element IDs this note is attached to. Empty vec means margin note.
    on: Vec<Id>,
    /// Alignment of the note relative to attached elements
    align: NoteAlign,
    /// Text content of the note
    content: String,
    /// Styling definition for the note
    definition: draw::NoteDefinition,
}

impl Note {
    /// Create a new Note.
    pub fn new(
        on: Vec<Id>,
        align: NoteAlign,
        content: String,
        definition: draw::NoteDefinition,
    ) -> Self {
        Self {
            on,
            align,
            content,
            definition,
        }
    }

    /// Get the element IDs this note is attached to.
    pub fn on(&self) -> &[Id] {
        &self.on
    }

    /// Get the alignment of the note.
    pub fn align(&self) -> NoteAlign {
        self.align
    }

    /// Get the text content of the note.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Borrow the note's styling definition.
    pub fn definition(&self) -> &draw::NoteDefinition {
        &self.definition
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
    Note(Note),
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
    /// The type definition for this fragment's styling
    type_definition: Rc<TypeDefinition>,
}

impl Fragment {
    /// Create a new Fragment with the given operation, sections, and type definition.
    pub fn new(
        operation: String,
        sections: Vec<FragmentSection>,
        type_definition: Rc<TypeDefinition>,
    ) -> Self {
        Self {
            operation,
            sections,
            type_definition,
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

    /// Clone and return the [`Rc<FragmentDefinition>`] for this fragment.
    ///
    /// This method retrieves the fragment definition from the type definition and
    /// returns a cloned Rc reference, allowing shared ownership of the definition.
    ///
    /// # Panics
    /// Panics if the type definition does not have a fragment definition.
    pub fn clone_fragment_definition(&self) -> draw::FragmentDefinition {
        self.type_definition
            .fragment_definition()
            .expect("Type definition must have a fragment definition")
            .clone()
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
    Shape(Box<dyn draw::ShapeDefinition>),
    Arrow(draw::ArrowDefinition),
    Fragment(draw::FragmentDefinition),
    Note(draw::NoteDefinition),
    ActivationBox(draw::ActivationBoxDefinition),
    Stroke(draw::StrokeDefinition),
    Text(draw::TextDefinition),
}

/// A concrete, elaborated type with text styling and drawing definition.
#[derive(Debug)]
pub struct TypeDefinition {
    id: Id,
    draw_definition: DrawDefinition,
}

/// Available layout engines controlling automatic positioning for diagrams.
/// Names match external configuration strings (snake_case).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutEngine {
    #[default]
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
    lifeline_definition: Option<Rc<draw::LifelineDefinition>>,
    activation_box_definition: Option<Rc<draw::ActivationBoxDefinition>>,
}

impl Diagram {
    /// Create a new Diagram with its kind, scope, layout engine, and optional background color.
    pub fn new(
        kind: DiagramKind,
        scope: Scope,
        layout_engine: LayoutEngine,
        background_color: Option<Color>,
        lifeline_definition: Option<Rc<draw::LifelineDefinition>>,
        activation_box_definition: Option<Rc<draw::ActivationBoxDefinition>>,
    ) -> Self {
        Self {
            kind,
            scope,
            layout_engine,
            background_color,
            lifeline_definition,
            activation_box_definition,
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

    /// Get the lifeline definition if specified (for sequence diagrams).
    pub fn lifeline_definition(&self) -> Option<&Rc<draw::LifelineDefinition>> {
        self.lifeline_definition.as_ref()
    }

    /// Get the activation box definition if specified (for sequence diagrams).
    pub fn activation_box_definition(&self) -> Option<&Rc<draw::ActivationBoxDefinition>> {
        self.activation_box_definition.as_ref()
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
    fn new(id: Id, draw_definition: DrawDefinition) -> Self {
        Self {
            id,
            draw_definition,
        }
    }

    /// Construct a concrete shape type definition from a shape definition.
    pub fn new_shape(id: Id, shape_definition: Box<dyn draw::ShapeDefinition>) -> Self {
        Self::new(id, DrawDefinition::Shape(shape_definition))
    }

    /// Construct a concrete arrow type definition from an arrow definition.
    pub fn new_arrow(id: Id, arrow_definition: draw::ArrowDefinition) -> Self {
        Self::new(id, DrawDefinition::Arrow(arrow_definition))
    }

    /// Construct a concrete fragment type definition from a fragment definition.
    pub fn new_fragment(id: Id, fragment_definition: draw::FragmentDefinition) -> Self {
        Self::new(id, DrawDefinition::Fragment(fragment_definition))
    }

    /// Construct a concrete note type definition from a note definition.
    pub fn new_note(id: Id, note_definition: draw::NoteDefinition) -> Self {
        Self::new(id, DrawDefinition::Note(note_definition))
    }

    /// Construct a concrete activation box type definition from a activation box definition.
    pub fn new_activation_box(
        id: Id,
        activation_box_definition: draw::ActivationBoxDefinition,
    ) -> Self {
        Self::new(id, DrawDefinition::ActivationBox(activation_box_definition))
    }

    /// Construct a concrete stroke type definition from a stroke definition.
    pub fn new_stroke(id: Id, stroke_definition: draw::StrokeDefinition) -> Self {
        Self::new(id, DrawDefinition::Stroke(stroke_definition))
    }

    /// Construct a concrete text type definition from a text definition.
    pub fn new_text(id: Id, text_definition: draw::TextDefinition) -> Self {
        Self::new(id, DrawDefinition::Text(text_definition))
    }

    /// Get the identifier for this type definition.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Borrow the shape definition if this type is a shape; otherwise returns an error.
    pub fn shape_definition(&self) -> Result<&dyn draw::ShapeDefinition, String> {
        match &self.draw_definition {
            DrawDefinition::Shape(shape) => Ok(&**shape),
            _ => Err(format!("Type '{}' is not a shape type", self.id)),
        }
    }

    /// Borrow the arrow definition if this type is an arrow; otherwise returns an error.
    pub fn arrow_definition(&self) -> Result<&draw::ArrowDefinition, String> {
        match &self.draw_definition {
            DrawDefinition::Arrow(arrow) => Ok(arrow),
            _ => Err(format!("Type '{}' is not an arrow type", self.id)),
        }
    }

    /// Borrow the fragment definition if this type is a fragment; otherwise returns an error.
    pub fn fragment_definition(&self) -> Result<&draw::FragmentDefinition, String> {
        match &self.draw_definition {
            DrawDefinition::Fragment(fragment) => Ok(fragment),
            _ => Err(format!("Type '{}' is not a fragment type", self.id)),
        }
    }

    /// Borrow the note definition if this type is a note; otherwise returns an error.
    pub fn note_definition(&self) -> Result<&draw::NoteDefinition, String> {
        match &self.draw_definition {
            DrawDefinition::Note(note) => Ok(note),
            _ => Err(format!("Type '{}' is not a note type", self.id)),
        }
    }

    /// Borrow the stroke definition if this type is a stroke; otherwise returns an error.
    pub fn stroke_definition(&self) -> Result<&draw::StrokeDefinition, String> {
        match &self.draw_definition {
            DrawDefinition::Stroke(stroke) => Ok(stroke),
            _ => Err(format!("Type '{}' is not a stroke type", self.id)),
        }
    }

    /// Borrow the text definition if this type is a text; otherwise returns an error.
    pub fn text_definition_from_draw(&self) -> Result<&draw::TextDefinition, String> {
        match &self.draw_definition {
            DrawDefinition::Text(text) => Ok(text),
            _ => Err(format!("Type '{}' is not a text type", self.id)),
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
    ) -> DiagnosticResult<()> {
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
    ) -> DiagnosticResult<()> {
        let name = attr.name.inner();
        let value = &attr.value;

        match *name {
            "font_size" => {
                let val = value.as_u16().map_err(|_| {
                    DiagnosticError::from_span(
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
                    DiagnosticError::from_span(
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
                    DiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid color value",
                        Some("Color values must be strings".to_string()),
                    )
                })?)
                .map_err(|err| {
                    DiagnosticError::from_span(
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
                    DiagnosticError::from_span(
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
                    DiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid color value",
                        Some("Color values must be strings".to_string()),
                    )
                })?)
                .map_err(|err| {
                    DiagnosticError::from_span(
                        format!("Invalid color: {err}"),
                        attr.span(),
                        "invalid color",
                        Some("Use a CSS color".to_string()),
                    )
                })?;
                text_def.set_color(Some(val));
                Ok(())
            }
            name => Err(DiagnosticError::from_span(
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

/// Helper for extracting stroke attributes from nested attribute lists.
pub struct StrokeAttributeExtractor;

impl StrokeAttributeExtractor {
    /// Extract and apply stroke-related attributes to a StrokeDefinition from a group of nested attributes.
    pub fn extract_stroke_attributes(
        stroke_def: &mut draw::StrokeDefinition,
        attrs: &[parser_types::Attribute],
    ) -> DiagnosticResult<()> {
        for attr in attrs {
            Self::extract_single_attribute(stroke_def, attr)?;
        }
        Ok(())
    }

    /// Extract and apply a single stroke-related attribute to a StrokeDefinition.
    fn extract_single_attribute(
        stroke_def: &mut draw::StrokeDefinition,
        attr: &parser_types::Attribute,
    ) -> DiagnosticResult<()> {
        let name = *attr.name.inner();
        let value = &attr.value;

        match name {
            "color" => {
                let color_str = value.as_str().map_err(|err| {
                    DiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid color value",
                        Some("Color values must be strings".to_string()),
                    )
                })?;
                let val = Color::new(color_str).map_err(|err| {
                    DiagnosticError::from_span(
                        format!("Invalid stroke color: {err}"),
                        attr.span(),
                        "invalid color",
                        Some("Use a CSS color".to_string()),
                    )
                })?;
                stroke_def.set_color(val);
                Ok(())
            }
            "width" => {
                let val = value.as_float().map_err(|err| {
                    DiagnosticError::from_span(
                        format!("Invalid stroke width value: {err}"),
                        attr.span(),
                        "invalid number",
                        Some("Width must be a positive number".to_string()),
                    )
                })?;
                stroke_def.set_width(val);
                Ok(())
            }
            "style" => {
                let style_str = value.as_str().map_err(|err| {
                    DiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid stroke style value",
                        Some("Stroke style must be a string".to_string()),
                    )
                })?;

                // Parse as predefined style or custom pattern
                // Note: Currently never fails, but may fail in the future when custom pattern validation is added
                let style = draw::StrokeStyle::from_str(style_str).map_err(|err| {
                    DiagnosticError::from_span(
                        err,
                        attr.span(),
                        "invalid stroke style",
                        Some("Use a valid style name or dasharray pattern".to_string()),
                    )
                })?;

                stroke_def.set_style(style);
                Ok(())
            }
            "cap" => {
                let cap_str = value.as_str().map_err(|err| {
                    DiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid stroke cap value",
                        Some("Stroke cap must be a string".to_string()),
                    )
                })?;
                let cap = draw::StrokeCap::from_str(cap_str).map_err(|err| {
                    DiagnosticError::from_span(
                        err,
                        attr.span(),
                        "invalid stroke cap",
                        Some("Valid values are: butt, round, square".to_string()),
                    )
                })?;
                stroke_def.set_cap(cap);
                Ok(())
            }
            "join" => {
                let join_str = value.as_str().map_err(|err| {
                    DiagnosticError::from_span(
                        err.to_string(),
                        attr.span(),
                        "invalid stroke join value",
                        Some("Stroke join must be a string".to_string()),
                    )
                })?;
                let join = draw::StrokeJoin::from_str(join_str).map_err(|err| {
                    DiagnosticError::from_span(
                        err,
                        attr.span(),
                        "invalid stroke join",
                        Some("Valid values are: miter, round, bevel".to_string()),
                    )
                })?;
                stroke_def.set_join(join);
                Ok(())
            }
            name => Err(DiagnosticError::from_span(
                format!("Unknown stroke attribute '{name}'"),
                attr.span(),
                "unknown stroke attribute",
                Some("Valid stroke attributes are: color, width, style, cap, join".to_string()),
            )),
        }
    }
}

impl TypeDefinition {
    pub fn from_base(
        id: Id,
        base: &Self,
        attributes: &[parser_types::Attribute],
    ) -> DiagnosticResult<Self> {
        match &base.draw_definition {
            DrawDefinition::Shape(shape_def) => {
                let mut new_shape_def = shape_def.clone_box();

                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "fill_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                DiagnosticError::from_span(
                                    format!("Invalid fill_color '{value}': {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_shape_def.set_fill_color(Some(val)).map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "stroke" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid stroke attribute value",
                                        Some(
                                            "Stroke attribute must contain type spec like"
                                                .to_string(),
                                        ),
                                    )
                                })?
                                .attributes;

                            StrokeAttributeExtractor::extract_stroke_attributes(
                                new_shape_def.mut_stroke(),
                                nested_attrs,
                            )?;
                        }
                        "rounded" => {
                            let val = value.as_usize().map_err(|err| {
                                DiagnosticError::from_span(
                                    format!("Invalid rounded value: {err}"),
                                    attr.span(),
                                    "invalid number",
                                    Some("Rounded must be a positive number".to_string()),
                                )
                            })?;
                            new_shape_def.set_rounded(val).map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "unsupported attribute",
                                    None,
                                )
                            })?;
                        }
                        "text" => {
                            // Handle nested text attributes
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid text attribute value",
                                        Some(
                                            "Text attribute must contain type spec like"
                                                .to_string(),
                                        ),
                                    )
                                })?
                                .attributes;

                            TextAttributeExtractor::extract_text_attributes(
                                new_shape_def.mut_text(),
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(DiagnosticError::from_span(
                                format!("Unknown shape attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid shape attributes are: fill_color, stroke=[...], rounded, text=[...]"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_shape(id, new_shape_def))
            }
            DrawDefinition::Arrow(arrow_def) => {
                let mut new_arrow_def = arrow_def.clone();

                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "stroke" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid stroke attribute value",
                                        Some("Stroke attribute must contain type spec".to_string()),
                                    )
                                })?
                                .attributes;

                            StrokeAttributeExtractor::extract_stroke_attributes(
                                new_arrow_def.mut_stroke(),
                                nested_attrs,
                            )?;
                        }
                        "style" => {
                            let val =
                                draw::ArrowStyle::from_str(value.as_str().map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid style value",
                                        Some("Style values must be strings".to_string()),
                                    )
                                })?)
                                .map_err(|_| {
                                    DiagnosticError::from_span(
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
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid text attribute value",
                                        Some("Text attribute must contain type spec".to_string()),
                                    )
                                })?
                                .attributes;

                            TextAttributeExtractor::extract_text_attributes(
                                new_arrow_def.mut_text(),
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(DiagnosticError::from_span(
                                format!("Unknown arrow attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid arrow attributes are: stroke=[...], style, text=[...]"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_arrow(id, new_arrow_def))
            }
            DrawDefinition::Fragment(fragment_def) => {
                let mut new_fragment_def = fragment_def.clone();

                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "border_stroke" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid border_stroke attribute value",
                                        Some(
                                            "Border stroke attribute must contain type spec"
                                                .to_string(),
                                        ),
                                    )
                                })?
                                .attributes;

                            StrokeAttributeExtractor::extract_stroke_attributes(
                                new_fragment_def.mut_border_stroke(),
                                nested_attrs,
                            )?;
                        }
                        "background_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                DiagnosticError::from_span(
                                    format!("Invalid background_color '{value}': {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_fragment_def.set_background_color(Some(val));
                        }
                        "separator_stroke" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid separator_stroke attribute value",
                                        Some(
                                            "Separator stroke attribute must contain type spec"
                                                .to_string(),
                                        ),
                                    )
                                })?
                                .attributes;

                            StrokeAttributeExtractor::extract_stroke_attributes(
                                new_fragment_def.mut_separator_stroke(),
                                nested_attrs,
                            )?;
                        }
                        "content_padding" => {
                            let val = value.as_float().map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid padding value",
                                    Some("Content padding must be a positive number".to_string()),
                                )
                            })?;
                            new_fragment_def.set_content_padding(Insets::uniform(val));
                        }
                        "text" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid text attribute value",
                                        Some("Text attribute must contain type spec".to_string()),
                                    )
                                })?
                                .attributes;

                            TextAttributeExtractor::extract_text_attributes(
                                new_fragment_def.mut_operation_label_text(),
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(DiagnosticError::from_span(
                                format!("Unknown fragment attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid fragment attributes are: border_stroke=[...], separator_stroke=[...], background_color, content_padding, text=[...]"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_fragment(id, new_fragment_def))
            }
            DrawDefinition::Note(note_def) => {
                let mut new_note_def = note_def.clone();

                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "background_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                DiagnosticError::from_span(
                                    format!("Invalid background_color '{value}': {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_note_def.set_background_color(Some(val));
                        }
                        "stroke" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid stroke attribute value",
                                        Some("Stroke attribute must contain type spec".to_string()),
                                    )
                                })?
                                .attributes;

                            StrokeAttributeExtractor::extract_stroke_attributes(
                                new_note_def.mut_stroke(),
                                nested_attrs,
                            )?;
                        }
                        "text" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid text attribute value",
                                        Some("Text attribute must contain type spec".to_string()),
                                    )
                                })?
                                .attributes;

                            TextAttributeExtractor::extract_text_attributes(
                                new_note_def.mut_text(),
                                nested_attrs,
                            )?;
                        }
                        "on" | "align" => {
                            // Skip positioning attributes - these are handled by build_note_element
                            // and are not part of the note's styling definition
                        }
                        name => {
                            return Err(DiagnosticError::from_span(
                                format!("Unknown note attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid note attributes are: background_color, stroke=[...], text=[...]"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_note(id, new_note_def))
            }
            DrawDefinition::ActivationBox(activation_box_def) => {
                let mut new_activation_box_def = activation_box_def.clone();

                for attr in attributes {
                    let name = attr.name.inner();
                    let value = &attr.value;

                    match *name {
                        "width" => {
                            let val = value.as_float().map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid width value",
                                    Some("Width must be a positive number".to_string()),
                                )
                            })?;
                            new_activation_box_def.set_width(val);
                        }
                        "nesting_offset" => {
                            let val = value.as_float().map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid nesting_offset value",
                                    Some("Nesting offset must be a positive number".to_string()),
                                )
                            })?;
                            new_activation_box_def.set_nesting_offset(val);
                        }
                        "fill_color" => {
                            let val = Color::new(value.as_str().map_err(|err| {
                                DiagnosticError::from_span(
                                    err.to_string(),
                                    attr.span(),
                                    "invalid color value",
                                    Some("Color values must be strings".to_string()),
                                )
                            })?)
                            .map_err(|err| {
                                DiagnosticError::from_span(
                                    format!("Invalid fill_color '{value}': {err}"),
                                    attr.span(),
                                    "invalid color",
                                    Some("Use a CSS color".to_string()),
                                )
                            })?;
                            new_activation_box_def.set_fill_color(val);
                        }
                        "stroke" => {
                            let nested_attrs = &value
                                .as_type_spec()
                                .map_err(|err| {
                                    DiagnosticError::from_span(
                                        err.to_string(),
                                        attr.span(),
                                        "invalid stroke attribute value",
                                        Some("Stroke attribute must contain type spec".to_string()),
                                    )
                                })?
                                .attributes;

                            StrokeAttributeExtractor::extract_stroke_attributes(
                                new_activation_box_def.mut_stroke(),
                                nested_attrs,
                            )?;
                        }
                        name => {
                            return Err(DiagnosticError::from_span(
                                format!("Unknown activation box attribute '{name}'"),
                                attr.span(),
                                "unknown attribute",
                                Some(
                                    "Valid activation box attributes are: width, nesting_offset, fill_color, stroke=[...]"
                                        .to_string(),
                                ),
                            ));
                        }
                    }
                }

                Ok(Self::new_activation_box(id, new_activation_box_def))
            }
            DrawDefinition::Stroke(stroke_def) => {
                let mut new_stroke = stroke_def.clone();
                StrokeAttributeExtractor::extract_stroke_attributes(&mut new_stroke, attributes)?;
                Ok(Self::new_stroke(id, new_stroke))
            }
            DrawDefinition::Text(text_def) => {
                let mut new_text_def = text_def.clone();
                TextAttributeExtractor::extract_text_attributes(&mut new_text_def, attributes)?;
                Ok(Self::new_text(id, new_text_def))
            }
        }
    }
}

#[cfg(test)]
mod elaborate_tests {
    use super::*;
    use crate::ast::span::{Span, Spanned};

    #[test]
    fn test_new_stroke_type() {
        let stroke = draw::StrokeDefinition::default();
        let type_def = TypeDefinition::new_stroke(Id::new("TestStroke"), stroke);
        assert_eq!(type_def.id(), "TestStroke");
        assert!(type_def.stroke_definition().is_ok());
        assert!(type_def.text_definition_from_draw().is_err());
    }

    #[test]
    fn test_new_text_type() {
        let text = draw::TextDefinition::default();
        let type_def = TypeDefinition::new_text(Id::new("TestText"), text);
        assert_eq!(type_def.id(), "TestText");
        assert!(type_def.text_definition_from_draw().is_ok());
        assert!(type_def.stroke_definition().is_err());
    }

    #[test]
    fn test_shape_type_has_text_definition() {
        let type_def =
            TypeDefinition::new_shape(Id::new("Rect"), Box::new(draw::RectangleDefinition::new()));
        // Verify shape has embedded text
        assert!(type_def.shape_definition().is_ok());
        let shape_def = type_def.shape_definition().unwrap();
        let _text = shape_def.text(); // Should not panic
        assert!(type_def.stroke_definition().is_err());
    }

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
        let base_shape_def = Box::new(draw::RectangleDefinition::new());
        let base_type = TypeDefinition::new_shape(base_id, base_shape_def);

        // Create attributes including text group
        let text_attrs = vec![
            create_test_attribute("font_size", create_float_value(14.0)),
            create_test_attribute("font_family", create_string_value("Arial")),
        ];

        let attributes = vec![
            create_test_attribute("fill_color", create_string_value("blue")),
            create_test_attribute(
                "text",
                parser_types::AttributeValue::TypeSpec(parser_types::TypeSpec {
                    type_name: None,
                    attributes: text_attrs,
                }),
            ),
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
        let base_shape_def = Box::new(draw::RectangleDefinition::new());
        let base_type = TypeDefinition::new_shape(base_id, base_shape_def);

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
            assert!(error_message.contains("Expected type spec"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_all_attributes() {
        let attrs = vec![
            create_test_attribute("color", create_string_value("blue")),
            create_test_attribute("width", create_float_value(2.5)),
            create_test_attribute("style", create_string_value("dashed")),
            create_test_attribute("cap", create_string_value("round")),
            create_test_attribute("join", create_string_value("bevel")),
        ];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_ok());
        assert_eq!(stroke_def.color().to_string(), "blue");
        assert_eq!(stroke_def.width(), 2.5);
        assert_eq!(*stroke_def.style(), draw::StrokeStyle::Dashed);
        assert_eq!(stroke_def.cap(), draw::StrokeCap::Round);
        assert_eq!(stroke_def.join(), draw::StrokeJoin::Bevel);
    }

    #[test]
    fn test_stroke_attribute_extractor_color_only() {
        let attrs = vec![create_test_attribute("color", create_string_value("red"))];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_ok());
        assert_eq!(stroke_def.color().to_string(), "red");
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_attribute_name() {
        let attrs = vec![create_test_attribute(
            "invalid_attr",
            create_string_value("value"),
        )];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_message = format!("{}", error);
            assert!(error_message.contains("Unknown stroke attribute"));
            assert!(error_message.contains("invalid_attr"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_color() {
        let attrs = vec![create_test_attribute(
            "color",
            create_string_value("not-a-valid-color-12345"),
        )];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_message = format!("{}", error);
            assert!(error_message.contains("Invalid stroke color"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_cap() {
        let attrs = vec![create_test_attribute("cap", create_string_value("invalid"))];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_message = format!("{}", error);
            assert!(error_message.contains("Invalid stroke cap"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_invalid_join() {
        let attrs = vec![create_test_attribute(
            "join",
            create_string_value("invalid"),
        )];

        let mut stroke_def = draw::StrokeDefinition::default();
        let result = StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

        assert!(result.is_err());
        if let Err(error) = result {
            let error_message = format!("{}", error);
            assert!(error_message.contains("Invalid stroke join"));
        }
    }

    #[test]
    fn test_stroke_attribute_extractor_all_predefined_styles() {
        let styles = vec![
            ("solid", draw::StrokeStyle::Solid),
            ("dashed", draw::StrokeStyle::Dashed),
            ("dotted", draw::StrokeStyle::Dotted),
            ("dash-dot", draw::StrokeStyle::DashDot),
            ("dash-dot-dot", draw::StrokeStyle::DashDotDot),
        ];

        for (style_str, expected_style) in styles {
            let attrs = vec![create_test_attribute(
                "style",
                create_string_value(style_str),
            )];
            let mut stroke_def = draw::StrokeDefinition::default();
            let result =
                StrokeAttributeExtractor::extract_stroke_attributes(&mut stroke_def, &attrs);

            assert!(result.is_ok());
            assert_eq!(*stroke_def.style(), expected_style);
        }
    }
}
