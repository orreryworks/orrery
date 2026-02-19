//! Component diagram graph structures and containment scope management.
//!
//! This module provides the graph representation for component diagrams, which emphasizes
//! hierarchical containment and spatial relationships between components. The key abstractions
//! are:
//!
//! - [`ComponentGraph`]: The main graph structure that stores nodes (components) and edges (relations)
//! - [`ContainmentScope`]: Groups components at the same hierarchical level within their containers
//!
//! # Architecture
//!
//! Component diagrams support nested containment where components can contain other components
//! and even embedded diagrams. This creates a hierarchical structure that is represented through
//! containment scopes. Each scope tracks:
//! - The components at that level
//! - The relations between those components
//! - The optional container component (for nested scopes)
//!
//! Relations can cross containment boundaries (e.g., a component at the root level can have
//! a relation to a component nested inside another component).

use log::debug;

use filament_core::{identifier::Id, semantic};

use super::{
    HierarchyNode,
    graph_base::{EdgeIndex, GraphInternal},
};
use crate::FilamentError;

/// A containment scope within a component diagram.
///
/// A containment scope groups nodes and relations that belong to the same
/// hierarchical level, optionally within a container node. This structure
/// is essential for maintaining the spatial hierarchy of component diagrams,
/// where components can be nested within other components.
///
/// - Root level scope: Components at the top level of the diagram (container = None)
/// - Nested scope: Components inside another component (container = Some(parent_id))
#[derive(Debug)]
pub struct ContainmentScope<'a, 'idx> {
    container: Option<Id>,
    graph: GraphInternal<'a, Id, EdgeIndex<'idx>>,
}

impl<'a, 'idx> ContainmentScope<'a, 'idx> {
    /// Returns the ID of the container component if this scope is nested.
    ///
    /// Returns `None` for root-level scopes, `Some(id)` for scopes nested within a component.
    pub fn container(&self) -> Option<Id> {
        self.container
    }

    /// Returns an iterator over all component IDs in this containment scope.
    ///
    /// This includes only the components at this specific hierarchical level,
    /// not components in nested scopes.
    pub fn node_ids(&self) -> impl Iterator<Item = Id> {
        self.graph.nodes()
    }

    /// Returns the number of components in this containment scope.
    pub fn nodes_count(&self) -> usize {
        self.graph.nodes_count()
    }

    /// Creates a new containment scope with an optional container component.
    fn new(container: Option<Id>) -> Self {
        ContainmentScope {
            container,
            graph: GraphInternal::new(),
        }
    }

    /// Adds a component node to this containment scope.
    fn add_node(&mut self, node: &semantic::Node) {
        let id = node.id();
        self.graph.add_node(id, id);
    }

    /// Checks if a component with the given ID exists in this containment scope.
    fn contains_node(&self, id: Id) -> bool {
        self.graph.contains_node(id)
    }

    /// Returns an iterator over root component IDs (components with no incoming relations).
    fn root_ids(&self) -> impl Iterator<Item = Id> {
        self.graph.roots()
    }

    /// Returns an iterator over IDs of components that are targets of relations from the given source.
    fn outgoing_node_ids(&self, source_id: Id) -> impl Iterator<Item = Id> {
        self.graph.outgoing_nodes(source_id)
    }

    /// Adds a relation edge to this containment scope.
    ///
    /// This tracks relations between components at the same hierarchical level.
    fn add_relation(&mut self, source_id: Id, target_id: Id, relation_idx: EdgeIndex<'idx>) {
        self.graph.add_edge(source_id, target_id, relation_idx);
    }

    /// Returns an iterator over relation edge indices in this scope.
    fn relation_indices(&self) -> impl Iterator<Item = EdgeIndex<'idx>> {
        self.graph.edges()
    }

    /// Recursively processes AST elements to build the component graph structure.
    ///
    /// This method traverses the AST elements, creating containment scopes for each
    /// hierarchical level and building the graph structure. It handles:
    /// - Adding component nodes to the graph
    /// - Processing nested scopes within components
    /// - Identifying embedded diagrams for separate processing
    /// - Adding relations between components
    ///
    /// # Returns
    /// A vector of `HierarchyNode`s representing any embedded diagrams found during processing.
    fn populate_component_graph(
        graph: &mut ComponentGraph<'a, '_>,
        elements: &'a [semantic::Element],
        container: Option<Id>,
    ) -> Result<Vec<HierarchyNode<'a, 'idx>>, FilamentError> {
        let mut child_diagrams = vec![];

        let mut containment_scope = ContainmentScope::new(container);

        // First pass: add all nodes to the graph
        for element in elements {
            if let semantic::Element::Node(node) = element {
                graph.add_node(&mut containment_scope, node);

                // Process the node's inner block recursively
                match node.block() {
                    semantic::Block::Scope(scope) => {
                        debug!(
                            "Processing nested scope with {} elements",
                            scope.elements().len()
                        );
                        let mut inner_child_diagrams = Self::populate_component_graph(
                            graph,
                            scope.elements(),
                            Some(node.id()),
                        )?;
                        child_diagrams.append(&mut inner_child_diagrams);
                    }
                    semantic::Block::Diagram(inner_diagram) => {
                        debug!(
                            "Processing nested diagram of kind {:?}",
                            inner_diagram.kind()
                        );
                        let inner_hierarchy_child =
                            HierarchyNode::build_from_ast_diagram(inner_diagram, Some(node.id()))?;
                        child_diagrams.push(inner_hierarchy_child);
                    }
                    semantic::Block::None => {}
                }
            }
        }

        // Second pass: add all relations and activation statements to the graph
        for element in elements {
            match element {
                semantic::Element::Relation(relation) => {
                    graph.add_relation(&mut containment_scope, relation);
                }
                semantic::Element::Node(..) => {}
                semantic::Element::Activate(..)
                | semantic::Element::Deactivate(..)
                | semantic::Element::Fragment(..)
                | semantic::Element::Note(..) => {
                    unreachable!("Unexpected element type")
                }
            }
        }

        graph.containment_scopes.push(containment_scope);

        Ok(child_diagrams)
    }
}

/// Main graph structure for component diagrams.
///
/// This structure maintains the complete graph representation of a component diagram,
/// including all nodes (components), edges (relations), and the hierarchical organization
/// through containment scopes. It provides methods to query and traverse the graph
/// structure during layout and rendering operations.
///
/// The graph uses a two-level organization:
/// 1. A global graph containing all nodes and relations
/// 2. Containment scopes that organize nodes by their hierarchical level
#[derive(Debug)]
pub struct ComponentGraph<'a, 'idx> {
    graph: GraphInternal<'idx, &'a semantic::Node, &'a semantic::Relation>,
    containment_scopes: Vec<ContainmentScope<'a, 'idx>>,
}

impl<'a, 'idx> ComponentGraph<'a, 'idx> {
    /// Returns the AST node for a component with the given ID, if it exists.
    pub fn node_by_id(&self, id: Id) -> Option<&semantic::Node> {
        self.graph.node(id)
    }

    /// Returns an iterator over all containment scopes in the graph.
    ///
    /// Scopes are ordered from outermost (root) to innermost (most deeply nested).
    pub fn containment_scopes(&self) -> std::slice::Iter<'_, ContainmentScope<'a, 'idx>> {
        self.containment_scopes.iter()
    }

    /// Returns an iterator over component nodes in a specific containment scope.
    ///
    /// This provides access to the AST nodes for all components at a particular
    /// hierarchical level in the diagram.
    pub fn scope_nodes(
        &self,
        containment_scope: &ContainmentScope,
    ) -> impl Iterator<Item = &semantic::Node> {
        containment_scope
            .node_ids()
            .map(|id| self.graph.node_unchecked(id))
    }

    /// Returns an iterator over relations in a specific containment scope.
    ///
    /// This provides access to the AST relation nodes for all relations between
    /// components at the same hierarchical level.
    pub fn scope_relations(
        &self,
        containment_scope: &ContainmentScope,
    ) -> impl Iterator<Item = &semantic::Relation> {
        containment_scope
            .relation_indices()
            .map(|idx| self.graph.edge_unchecked(idx))
    }

    /// Returns an iterator over root nodes in a containment scope.
    ///
    /// Root nodes are components that have no incoming relations within the scope,
    /// typically representing top-level components in the hierarchy.
    pub fn scope_roots(
        &self,
        containment_scope: &ContainmentScope,
    ) -> impl Iterator<Item = &semantic::Node> {
        containment_scope
            .root_ids()
            .map(|id| self.graph.node_unchecked(id))
    }

    /// Returns an iterator over nodes that are targets of relations from a source node.
    ///
    /// Given a source component ID and a containment scope, this returns all components
    /// that are connected via outgoing relations from the source within that scope.
    pub fn scope_outgoing_neighbors(
        &self,
        containment_scope: &ContainmentScope,
        source_id: Id,
    ) -> impl Iterator<Item = &semantic::Node> {
        containment_scope
            .outgoing_node_ids(source_id)
            .map(|id| self.graph.node_unchecked(id))
    }

    /// Creates a component graph from AST elements.
    ///
    /// Processes the elements to build the graph structure and identify any
    /// embedded diagrams that need separate processing.
    pub(super) fn new_from_elements(
        elements: &'a [semantic::Element],
    ) -> Result<(Self, Vec<HierarchyNode<'a, 'idx>>), FilamentError> {
        let mut graph = Self::new();
        let children = ContainmentScope::populate_component_graph(&mut graph, elements, None)?;
        Ok((graph, children))
    }

    /// Creates a new empty component graph.
    fn new() -> Self {
        Self {
            graph: GraphInternal::new(),
            containment_scopes: Vec::new(),
        }
    }

    /// Adds a component node to the graph and its containment scope.
    fn add_node(
        &mut self,
        containment_scope: &mut ContainmentScope<'a, 'idx>,
        node: &'a semantic::Node,
    ) {
        self.graph.add_node(node.id(), node);
        containment_scope.add_node(node);
    }

    /// Adds a relation to the graph and potentially to its containment scope.
    ///
    /// Relations are always added to the global graph, but only added to the
    /// containment scope if both endpoints exist in that scope. This handles
    /// cross-scope relations where components at different hierarchical levels
    /// are connected.
    fn add_relation(
        &mut self,
        containment_scope: &mut ContainmentScope<'a, 'idx>,
        relation: &'a semantic::Relation,
    ) {
        let source_id = relation.source();
        let target_id = relation.target();
        let idx = self.graph.add_edge(source_id, target_id, relation);

        // Only add relation to containment scope if both nodes exist in that scope
        // This handles cross-scope relations (e.g., metrics -> backend::user_db)
        // where the target node may be in a nested scope
        // TODO: Store cross containment in another variable
        if containment_scope.contains_node(source_id) && containment_scope.contains_node(target_id)
        {
            containment_scope.add_relation(source_id, target_id, idx);
        }
    }
}
