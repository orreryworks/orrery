//! Diagram element types for the semantic model.

use std::{fmt, rc::Rc, str::FromStr};

use crate::{draw, identifier::Id, semantic::diagram::Block};

/// A diagram node (component/participant) with visual definition and nested content.
#[derive(Debug, Clone)]
pub struct Node {
    id: Id,
    name: String,
    display_name: Option<String>,
    block: Block,
    shape_definition: Rc<Box<dyn draw::ShapeDefinition>>,
}

impl Node {
    /// Create a new Node.
    pub fn new(
        id: Id,
        name: String,
        display_name: Option<String>,
        block: Block,
        shape_definition: Rc<Box<dyn draw::ShapeDefinition>>,
    ) -> Self {
        Self {
            id,
            name,
            display_name,
            block,
            shape_definition,
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

    /// Borrow the node's shape definition.
    pub fn shape_definition(&self) -> &Rc<Box<dyn draw::ShapeDefinition>> {
        &self.shape_definition
    }

    /// Returns the display text for this node
    /// Uses display_name if present, otherwise falls back to the identifier name
    pub fn display_text(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// A relation (edge/message) between two nodes, carrying direction, text, and style.
///
/// Relations represent connections between nodes in diagrams.
#[derive(Debug, Clone)]
pub struct Relation {
    source: Id,
    target: Id,
    arrow_direction: draw::ArrowDirection,
    label: Option<String>,
    arrow_definition: Rc<draw::ArrowDefinition>,
}

impl Relation {
    /// Create a new Relation between two node Ids with an optional label
    /// and an arrow definition that determines appearance.
    pub fn new(
        source: Id,
        target: Id,
        arrow_direction: draw::ArrowDirection,
        label: Option<String>,
        arrow_definition: Rc<draw::ArrowDefinition>,
    ) -> Self {
        Self {
            source,
            target,
            arrow_direction,
            label,
            arrow_definition,
        }
    }

    /// Build a Text drawable for the relation's label using its text definition, if a label exists.
    pub fn text(&self) -> Option<draw::Text<'_>> {
        let label = self.label.as_ref()?;
        let text_def = self.arrow_definition.text();
        Some(draw::Text::new(text_def, label))
    }

    /// Get the underlying ArrowDefinition Rc for rendering this relation.
    pub fn arrow_definition(&self) -> &Rc<draw::ArrowDefinition> {
        &self.arrow_definition
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
/// # use filament::semantic::{Note, NoteAlign};
/// # use filament::identifier::Id;
/// # use filament::draw::NoteDefinition;
/// # use std::rc::Rc;
/// #
/// // Create a margin note (not attached to any elements)
/// let note = Note::new(
///     vec![],  // Empty vec = margin note
///     NoteAlign::Over,
///     "This is a note".to_string(),
///     Rc::new(NoteDefinition::new()),
/// );
/// assert_eq!(note.on().len(), 0);
/// assert_eq!(note.content(), "This is a note");
///
/// // Create a note attached to an element
/// let attached_note = Note::new(
///     vec![Id::new("server")],
///     NoteAlign::Right,
///     "Server note".to_string(),
///     Rc::new(NoteDefinition::new()),
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
    definition: Rc<draw::NoteDefinition>,
}

impl Note {
    /// Create a new Note.
    pub fn new(
        on: Vec<Id>,
        align: NoteAlign,
        content: String,
        definition: Rc<draw::NoteDefinition>,
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
    pub fn definition(&self) -> &Rc<draw::NoteDefinition> {
        &self.definition
    }
}

/// Represents an activation in a sequence diagram.
#[derive(Debug, Clone)]
pub struct Activate {
    /// Component ID being activated
    component: Id,
    /// Styling definition for the activation box
    definition: Rc<draw::ActivationBoxDefinition>,
}

impl Activate {
    /// Create a new Activate.
    pub fn new(component: Id, definition: Rc<draw::ActivationBoxDefinition>) -> Self {
        Self {
            component,
            definition,
        }
    }

    /// Get the component ID being activated.
    pub fn component(&self) -> Id {
        self.component
    }

    /// Borrow the activation box styling definition.
    pub fn definition(&self) -> &Rc<draw::ActivationBoxDefinition> {
        &self.definition
    }
}

/// Represents a fragment block in a sequence diagram.
///
/// Fragments group related interactions into labeled sections, helping structure
/// complex message flows and illustrate alternatives, loops, parallel execution,
/// and other control flow patterns.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// The operation string (e.g., "alt", "opt", "loop", "par")
    operation: String,
    /// The sections within this fragment
    sections: Vec<FragmentSection>,
    /// The fragment definition for this fragment's styling
    definition: Rc<draw::FragmentDefinition>,
}

impl Fragment {
    /// Create a new Fragment.
    pub fn new(
        operation: String,
        sections: Vec<FragmentSection>,
        definition: Rc<draw::FragmentDefinition>,
    ) -> Self {
        Self {
            operation,
            sections,
            definition,
        }
    }

    /// Get the operation string for this fragment.
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Get the sections of this fragment
    pub fn sections(&self) -> &[FragmentSection] {
        &self.sections
    }

    /// Get the fragment definition for this fragment.
    ///
    /// Returns a reference to the `Rc<FragmentDefinition>` allowing shared ownership of the definition.
    pub fn definition(&self) -> &Rc<draw::FragmentDefinition> {
        &self.definition
    }
}

/// Represents a section within a fragment.
///
/// Each section can have an optional title and contains a sequence of elements
/// that represent one phase within the fragment.
#[derive(Debug, Clone)]
pub struct FragmentSection {
    /// Optional title for this section
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

/// Top-level elaborated element within a scope.
///
/// The Element enum represents all possible diagram elements that can appear
/// within a scope.
#[derive(Debug, Clone)]
pub enum Element {
    /// A diagram node
    Node(Node),
    /// A relation between nodes
    Relation(Relation),
    /// Activation start
    Activate(Activate),
    /// Activation end
    Deactivate(Id),
    /// Fragment block
    Fragment(Fragment),
    /// Note annotation
    Note(Note),
}
