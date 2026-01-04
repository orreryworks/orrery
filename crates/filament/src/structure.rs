//! Graph structure representations for Filament diagrams.
//!
//! This module provides the core graph data structures used to represent parsed and elaborated
//! Filament diagrams. It transforms the AST representation into graph structures that are
//! optimized for layout and rendering operations.
//!
//! The module is organized into several layers:
//! - **Graph types**: Unified enum for different diagram graph types [`GraphKind`]
//! - **Diagram wrappers**: Container for a diagram's AST and its graph representation [`GraphedDiagram`]
//! - **Hierarchy management**: Tree structure for nested diagrams [`DiagramHierarchy`], [`HierarchyNode`]
//! - **Specialized graphs**: Type-specific graph implementations for component and sequence diagrams

use log::trace;

use crate::{FilamentError, ast, identifier::Id, semantic};

mod component;
mod graph_base;
mod sequence;

pub use component::{ComponentGraph, ContainmentScope};
pub use sequence::{SequenceEvent, SequenceGraph};

/// Unified representation of different diagram graph types.
///
/// # Variants
///
/// * `ComponentGraph` - Graph structure for component diagrams with containment scopes
/// * `SequenceGraph` - Graph structure for sequence diagrams with ordered events
#[derive(Debug)]
pub enum GraphKind<'a, 'idx> {
    ComponentGraph(ComponentGraph<'a, 'idx>),
    SequenceGraph(SequenceGraph<'a>),
}

impl<'a, 'idx> GraphKind<'a, 'idx> {
    /// Builds a component graph from AST elements.
    ///
    /// Processes the provided AST elements to construct a component graph with its
    /// containment scopes and hierarchical structure.
    ///
    /// # Arguments
    /// * `elements` - The AST elements to process into a component graph
    ///
    /// # Returns
    /// A tuple containing:
    /// - The constructed [`GraphKind::ComponentGraph`] variant
    /// - A vector of [`HierarchyNode`] representing any embedded diagrams found
    fn build_component(
        elements: &'a [semantic::Element],
    ) -> Result<(Self, Vec<HierarchyNode<'a, 'idx>>), FilamentError> {
        let (graph, children) = ComponentGraph::new_from_elements(elements)?;
        Ok((Self::ComponentGraph(graph), children))
    }

    /// Builds a sequence graph from AST elements.
    ///
    /// Processes the provided AST elements to construct a sequence graph with its
    /// participants and temporally ordered events.
    ///
    /// # Arguments
    /// * `elements` - The AST elements to process into a sequence graph
    ///
    /// # Returns
    /// A tuple containing:
    /// - The constructed [`GraphKind::SequenceGraph`] variant
    /// - A vector of [`HierarchyNode`] representing any embedded diagrams found
    fn build_sequence(
        elements: &'a [semantic::Element],
    ) -> Result<(Self, Vec<HierarchyNode<'a, 'idx>>), FilamentError> {
        let (graph, children) = SequenceGraph::new_from_elements(elements)?;
        Ok((Self::SequenceGraph(graph), children))
    }
}

/// Container that pairs an AST diagram with its graph representation.
#[derive(Debug)]
pub struct GraphedDiagram<'a, 'idx> {
    ast_diagram: &'a semantic::Diagram,
    graph_kind: GraphKind<'a, 'idx>,
}

impl<'a, 'idx> GraphedDiagram<'a, 'idx> {
    /// Returns a reference to the underlying AST diagram.
    pub fn ast_diagram(&self) -> &semantic::Diagram {
        self.ast_diagram
    }

    /// Returns a reference to the graph representation of this diagram.
    ///
    /// The graph kind determines whether this is a component or sequence
    /// diagram and provides access to the appropriate graph structure.
    pub fn graph_kind(&self) -> &GraphKind<'a, 'idx> {
        &self.graph_kind
    }

    /// Creates a new graphed diagram from an AST diagram and its graph representation.
    fn new(ast_diagram: &'a semantic::Diagram, graph_kind: GraphKind<'a, 'idx>) -> Self {
        Self {
            ast_diagram,
            graph_kind,
        }
    }
}

// =============================================================================
// Hierarchy structures (for nested diagrams)
// =============================================================================

/// Internal node in the diagram hierarchy tree.
///
/// Represents a single diagram within the hierarchy, tracking its graph representation,
/// optional container (for embedded diagrams), and any child diagrams it contains.
/// This structure enables recursive processing of nested diagram structures.
#[derive(Debug)]
struct HierarchyNode<'a, 'idx> {
    graphed_diagram: GraphedDiagram<'a, 'idx>,
    container_id: Option<Id>,
    children: Vec<HierarchyNode<'a, 'idx>>,
}

impl<'a, 'idx> HierarchyNode<'a, 'idx> {
    /// Creates a new hierarchy node with its diagram and children.
    ///
    /// # Arguments
    /// * `graphed_diagram` - The diagram with its graph representation
    /// * `container_id` - Optional ID of the container component (for embedded diagrams)
    /// * `children` - Vector of child hierarchy nodes for nested diagrams
    fn new(
        graphed_diagram: GraphedDiagram<'a, 'idx>,
        container_id: Option<Id>,
        children: Vec<HierarchyNode<'a, 'idx>>,
    ) -> Self {
        HierarchyNode {
            graphed_diagram,
            container_id,
            children,
        }
    }

    /// Builds a hierarchy node from an AST diagram.
    ///
    /// Recursively processes the diagram's elements to create the appropriate graph
    /// structure (component or sequence) and identifies any nested diagrams that
    /// need to be processed as children in the hierarchy.
    ///
    /// # Arguments
    /// * `ast_diagram` - The AST diagram to process
    /// * `container_id` - Optional ID of the container component (if this is an embedded diagram)
    ///
    /// # Returns
    /// A constructed `HierarchyNode` with its graph and children, or an error if
    /// graph construction fails.
    fn build_from_ast_diagram(
        ast_diagram: &'a semantic::Diagram,
        container_id: Option<Id>,
    ) -> Result<Self, FilamentError> {
        let (graph, children) = match ast_diagram.kind() {
            ast::DiagramKind::Component => {
                GraphKind::build_component(ast_diagram.scope().elements())?
            }
            ast::DiagramKind::Sequence => {
                GraphKind::build_sequence(ast_diagram.scope().elements())?
            }
        };
        let graphed_diagram = GraphedDiagram::new(ast_diagram, graph);

        Ok(Self::new(graphed_diagram, container_id, children))
    }
}

// =============================================================================
// Top-level public API
// =============================================================================

/// Top-level container for a diagram and all its nested sub-diagrams.
///
/// This structure manages the hierarchical relationships between diagrams when
/// components contain embedded diagrams. It provides methods to traverse the
/// hierarchy in post-order, ensuring that nested diagrams are processed before
/// their containers during layout and rendering operations.
///
/// # Example Structure
///
/// ```text
/// Root Diagram (Component)
/// ├── Component A
/// ├── Component B (with embedded sequence diagram)
/// │   └── Embedded Sequence Diagram
/// └── Component C
/// ```
#[derive(Debug)]
pub struct DiagramHierarchy<'a, 'idx> {
    root: HierarchyNode<'a, 'idx>,
}

impl<'a, 'idx> DiagramHierarchy<'a, 'idx> {
    /// Creates a hierarchical graph structure from an AST diagram.
    ///
    /// Recursively processes the diagram and all its nested sub-diagrams,
    /// building appropriate graph representations for each diagram type
    /// (component or sequence) and organizing them into a tree structure.
    ///
    /// # Arguments
    ///
    /// * `diagram` - The root AST diagram to convert
    ///
    /// # Returns
    ///
    /// A [`DiagramHierarchy`] containing the graph representations of all diagrams
    /// in the hierarchy, or a [`FilamentError`] if graph construction fails.
    pub fn from_diagram(diagram: &'a semantic::Diagram) -> Result<Self, FilamentError> {
        // Process all elements in the diagram recursively
        let root_diagram = HierarchyNode::build_from_ast_diagram(diagram, None)?;

        let hierarchy = DiagramHierarchy { root: root_diagram };

        trace!(hierarchy:?; "Created diagram hierarchy");

        Ok(hierarchy)
    }

    /// Returns an iterator that traverses the diagram hierarchy in post-order.
    ///
    /// Post-order traversal ensures that nested diagrams are visited before their
    /// containers, which is essential for bottom-up processing during layout and
    /// rendering. Each item in the iterator contains:
    /// - The optional container ID (None for the root diagram)
    /// - A reference to the graphed diagram
    // PERF: This allocates extra queue.
    pub fn iter_post_order(&self) -> impl Iterator<Item = (Option<Id>, &GraphedDiagram<'_, '_>)> {
        let mut stack = Vec::new();
        stack.push(&self.root);
        let mut i = 0;
        while i < stack.len() {
            let node = stack[i];
            for child in node.children.iter().rev() {
                stack.push(child);
            }
            i += 1;
        }
        stack
            .into_iter()
            .rev()
            .map(|node| (node.container_id, &node.graphed_diagram))
    }
}
