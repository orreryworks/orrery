//! Core diagram structure types.
//!
//! This module contains the fundamental building blocks of the semantic diagram model:
//! - [`Diagram`] - The root diagram type with kind, scope, and layout configuration
//! - [`Scope`] - Container for diagram elements
//! - [`Block`] - Represents nested content (none, scope, or embedded diagram)
//! - [`LayoutEngine`] - Enumeration of available layout algorithms

pub use crate::ast::DiagramKind;

use std::{
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{color::Color, draw, semantic::element::Element};

/// A scope containing a sequence of diagram elements.
///
/// A scope represents a container for diagram elements (nodes, relations, notes, etc.)
/// and forms the building block for both top-level diagrams and nested structures.
#[derive(Debug, Clone, Default)]
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

/// Available layout engines controlling automatic positioning for diagrams.
///
/// Layout engines determine how diagram elements are arranged spatially.
/// The names match external configuration strings (snake_case).
///
/// # Variants
///
/// - `Basic` - Simple layout algorithm (default)
/// - `Sugiyama` - Hierarchical graph layout using the Sugiyama method
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutEngine {
    /// Basic layout engine (default)
    #[default]
    Basic,
    /// Sugiyama hierarchical layout engine
    Sugiyama,
}

impl FromStr for LayoutEngine {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Self::Basic),
            "sugiyama" => Ok(Self::Sugiyama),
            _ => Err("Unsupported layout engine"),
        }
    }
}

impl From<LayoutEngine> for &'static str {
    fn from(val: LayoutEngine) -> Self {
        match val {
            LayoutEngine::Basic => "basic",
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

/// A fully elaborated diagram with kind, content scope, layout engine, and styling.
///
/// This is the root type of the semantic diagram model, representing a complete
/// diagram after parsing, desugaring, validation, and elaboration. All type
/// references have been resolved, attributes have been processed, and the diagram
/// is ready to be transformed into a hierarchy and laid out for rendering.
///
/// # Fields
///
/// - `kind` - The type of diagram (component, sequence, etc.)
/// - `scope` - The top-level container of diagram elements
/// - `layout_engine` - The algorithm to use for automatic positioning
/// - `background_color` - Optional background color for the diagram
/// - `lifeline_definition` - Optional lifeline styling (for sequence diagrams)
#[derive(Debug, Clone)]
pub struct Diagram {
    kind: DiagramKind,
    scope: Scope,
    layout_engine: LayoutEngine,
    background_color: Option<Color>,
    lifeline_definition: Option<Rc<draw::LifelineDefinition>>,
}

impl Diagram {
    /// Create a new Diagram with its kind, scope, layout engine, and optional background color.
    pub fn new(
        kind: DiagramKind,
        scope: Scope,
        layout_engine: LayoutEngine,
        background_color: Option<Color>,
        lifeline_definition: Option<Rc<draw::LifelineDefinition>>,
    ) -> Self {
        Self {
            kind,
            scope,
            layout_engine,
            background_color,
            lifeline_definition,
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
}

/// A block wrapper representing empty content, a nested scope, or an embedded diagram.
///
/// Blocks are used to represent the nested content within diagram nodes. A node can have:
/// - No nested content (`Block::None`)
/// - A nested scope of elements (`Block::Scope`)
/// - An embedded diagram (`Block::Diagram`)
///
/// This enables hierarchical diagram structures where nodes can contain other diagrams.
#[derive(Debug, Clone)]
pub enum Block {
    /// No nested content
    None,
    /// A nested scope containing elements
    Scope(Scope),
    /// An embedded diagram
    Diagram(Diagram),
}
