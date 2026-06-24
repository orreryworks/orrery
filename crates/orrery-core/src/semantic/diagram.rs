//! Core diagram structure types.
//!
//! This module contains the fundamental building blocks of the semantic diagram model:
//! - [`DiagramKind`] - The type of diagram (component or sequence)
//! - [`Diagram`] - The root diagram type with kind, scope, and layout configuration
//! - [`Scope`] - Container for diagram elements
//! - [`Block`] - Represents nested content (none, scope, or embedded diagram)
//! - [`LayoutEngine`] - Enumeration of available layout algorithms

use std::{fmt, rc::Rc, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{draw::DiagramDefinition, semantic::element::Element};

/// The kind of a diagram: component or sequence.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum DiagramKind {
    /// A component diagram showing structural relationships
    Component,
    /// A sequence diagram showing interactions over time
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
/// - [`Basic`](Self::Basic) - Simple layout algorithm (default without `graphviz` feature).
/// - [`Sugiyama`](Self::Sugiyama) - Hierarchical layered layout using the Sugiyama method.
/// - `Graphviz` - Graphviz-backed layout engine. Only present when the
///   `graphviz` Cargo feature is enabled. When enabled, this becomes the default layout engine.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutEngine {
    /// Simple built-in layout algorithm.
    ///
    /// Default when the `graphviz` feature is disabled.
    #[cfg_attr(not(feature = "graphviz"), default)]
    Basic,
    /// Hierarchical layered layout using the Sugiyama method.
    Sugiyama,
    /// Graphviz-backed layout engine.
    ///
    /// Gated by the `graphviz` Cargo feature. When enabled, this becomes the default layout engine.
    #[cfg(feature = "graphviz")]
    #[cfg_attr(feature = "graphviz", default)]
    Graphviz,
}

impl FromStr for LayoutEngine {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Self::Basic),
            "sugiyama" => Ok(Self::Sugiyama),
            #[cfg(feature = "graphviz")]
            "graphviz" => Ok(Self::Graphviz),
            _ => Err("Unsupported layout engine"),
        }
    }
}

impl From<LayoutEngine> for &'static str {
    fn from(val: LayoutEngine) -> Self {
        match val {
            LayoutEngine::Basic => "basic",
            LayoutEngine::Sugiyama => "sugiyama",
            #[cfg(feature = "graphviz")]
            LayoutEngine::Graphviz => "graphviz",
        }
    }
}

impl fmt::Display for LayoutEngine {
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
#[derive(Debug, Clone)]
pub struct Diagram {
    kind: DiagramKind,
    scope: Scope,
    layout_engine: LayoutEngine,
    definition: Rc<DiagramDefinition>,
}

impl Diagram {
    /// Creates a diagram.
    pub fn new(
        kind: DiagramKind,
        scope: Scope,
        layout_engine: LayoutEngine,
        definition: Rc<DiagramDefinition>,
    ) -> Self {
        Self {
            kind,
            scope,
            layout_engine,
            definition,
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

    /// Borrow the diagram's [`DiagramDefinition`].
    pub fn definition(&self) -> &Rc<DiagramDefinition> {
        &self.definition
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagram_kind_display() {
        assert_eq!(DiagramKind::Component.to_string(), "component");
        assert_eq!(DiagramKind::Sequence.to_string(), "sequence");
    }

    #[test]
    fn test_layout_engine_from_str() {
        assert_eq!(
            "basic".parse::<LayoutEngine>().unwrap(),
            LayoutEngine::Basic
        );
        assert_eq!(
            "sugiyama".parse::<LayoutEngine>().unwrap(),
            LayoutEngine::Sugiyama
        );
        #[cfg(feature = "graphviz")]
        assert_eq!(
            "graphviz".parse::<LayoutEngine>().unwrap(),
            LayoutEngine::Graphviz
        );

        let result: Result<LayoutEngine, _> = "invalid".parse();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Unsupported layout engine");
    }

    #[test]
    fn test_layout_engine_default() {
        #[cfg(feature = "graphviz")]
        assert_eq!(LayoutEngine::default(), LayoutEngine::Graphviz);
        #[cfg(not(feature = "graphviz"))]
        assert_eq!(LayoutEngine::default(), LayoutEngine::Basic);
    }

    #[test]
    fn test_layout_engine_display() {
        assert_eq!(LayoutEngine::Basic.to_string(), "basic");
        assert_eq!(LayoutEngine::Sugiyama.to_string(), "sugiyama");
        #[cfg(feature = "graphviz")]
        assert_eq!(LayoutEngine::Graphviz.to_string(), "graphviz");
    }
}
