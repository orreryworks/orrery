use crate::{ast, error::FilamentError};
use log::{debug, trace};
use petgraph::{
    Direction,
    graph::{DiGraph, EdgeIndex, NodeIndex},
    visit::{DfsPostOrder, Walker},
};
use std::collections::HashMap;

/// Represents a containment scope within a diagram.
///
/// A containment scope groups nodes and relations that belong to the same
/// hierarchical level, optionally within a container node.
#[derive(Debug)]
pub struct ContainmentScope {
    container: Option<NodeIndex>,
    nodes: Vec<NodeIndex>,
    relations: Vec<EdgeIndex>,
}

impl ContainmentScope {
    /// Creates a new containment scope with an optional container node.
    fn new(container: Option<NodeIndex>) -> Self {
        Self {
            container,
            nodes: Vec::new(),
            relations: Vec::new(),
        }
    }

    /// Returns the container node index if this scope is contained within another node.
    pub fn container(&self) -> Option<NodeIndex> {
        self.container
    }

    /// Returns an iterator over all node indices in this containment scope.
    pub fn node_indices(&self) -> impl Iterator<Item = NodeIndex> {
        self.nodes.iter().copied()
    }

    /// Adds a node to this containment scope.
    fn add_node(&mut self, idx: NodeIndex) {
        self.nodes.push(idx);
    }

    /// Adds a relation (edge) to this containment scope.
    fn add_relation(&mut self, idx: EdgeIndex) {
        self.relations.push(idx);
    }
}

/// Represents ordered events.
///
/// # Variants
///
/// * [`Event::Relation`] - A relation between components
/// * [`Event::Activate`] - Start of an activation period for a component
/// * [`Event::Deactivate`] - End of an activation period for a component
#[derive(Debug)]
pub enum Event {
    /// A relation between two components.
    Relation(EdgeIndex),

    /// Start of an activation period for a component.
    ///
    /// Contains the [`NodeIndex`] of the component that becomes active.
    Activate(NodeIndex),

    /// End of an activation period for a component.
    ///
    /// Contains the [`NodeIndex`] of the component that becomes inactive.
    Deactivate(NodeIndex),
}

/// Represents a graph structure for a single diagram.
///
/// This structure contains the graph representation of nodes and relations
/// from a Filament diagram, along with organizational information about
/// containment scopes and ordered events.
#[derive(Debug)]
pub struct Graph<'a> {
    graph: DiGraph<ast::Node, ast::Relation>,
    diagram: &'a ast::Diagram,
    containment_scopes: Vec<ContainmentScope>,
    node_id_map: HashMap<ast::TypeId, NodeIndex>, // Maps node IDs to their indices
    ordered_events: Vec<Event>,
}

/// Represents a collection of interconnected diagrams with hierarchical relationships.
///
/// This structure manages multiple diagrams that may contain embedded sub-diagrams,
/// organizing them into a tree structure.
#[derive(Debug)]
pub struct Collection<'a> {
    diagram_tree: DiGraph<Graph<'a>, ast::TypeId>,
    root_diagram: Option<NodeIndex>,
}

impl<'a> Graph<'a> {
    fn new(diagram: &'a ast::Diagram) -> Self {
        Self {
            graph: DiGraph::new(),
            diagram,
            containment_scopes: Vec::new(),
            node_id_map: HashMap::new(),
            ordered_events: Vec::new(),
        }
    }

    pub fn diagram(&self) -> &ast::Diagram {
        self.diagram
    }

    pub fn node_id_map(&self) -> &HashMap<ast::TypeId, NodeIndex> {
        &self.node_id_map
    }
    pub fn containment_scopes(&self) -> &[ContainmentScope] {
        &self.containment_scopes
    }

    pub fn node_indices(&self) -> impl Iterator<Item = NodeIndex> {
        self.graph.node_indices()
    }

    pub fn nodes_with_indices(&self) -> impl Iterator<Item = (NodeIndex, &ast::Node)> {
        self.graph.node_indices().map(|idx| {
            (
                idx,
                self.graph.node_weight(idx).expect("Node idx should exist"),
            )
        })
    }

    pub fn node_from_idx(&self, node_index: NodeIndex) -> &ast::Node {
        self.graph
            .node_weight(node_index)
            .expect("Node index should exist")
    }

    pub fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex> {
        self.graph.edge_indices()
    }

    pub fn edge_weight(&self, edge_index: EdgeIndex) -> Option<&ast::Relation> {
        self.graph.edge_weight(edge_index)
    }

    pub fn edge_endpoints(&self, edge_index: EdgeIndex) -> Option<(NodeIndex, NodeIndex)> {
        self.graph.edge_endpoints(edge_index)
    }

    /// Returns an iterator over all events in AST order.
    ///
    /// This method provides access to events (relations, activations, deactivations) in the exact
    /// order they appear in the AST.
    ///
    /// # Returns
    /// An iterator yielding `&Event` items in AST order.
    pub fn ordered_events(&self) -> impl Iterator<Item = &Event> {
        self.ordered_events.iter()
    }

    /// Returns an iterator over just relation events in AST order.
    ///
    /// This method filters the ordered events to return only `EdgeIndex` values for relations,
    /// maintaining AST order.
    ///
    /// # Returns
    /// An iterator yielding `EdgeIndex` values for relation events only, in AST order.
    pub fn ordered_relations(&self) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.ordered_events().filter_map(|event| match event {
            Event::Relation(edge_idx) => Some(*edge_idx),
            _ => None,
        })
    }

    /// Extract message information from a relation event.
    ///
    /// Given an `EdgeIndex` from a relation event, this method extracts all the information
    /// needed to create a message: the source and target node indices, and the relation AST node.
    /// This is a convenience method that combines `edge_endpoints()` and `edge_weight()` calls.
    ///
    /// # Parameters
    /// * `edge_idx` - The edge index from an `Event::Relation` event
    ///
    /// # Returns
    /// * `Some((source_node, target_node, relation))` if the edge exists
    /// * `None` if the edge index is invalid
    pub fn relation_message_info(
        &self,
        edge_idx: EdgeIndex,
    ) -> Option<(NodeIndex, NodeIndex, &ast::Relation)> {
        if let (Some(endpoints), Some(relation)) =
            (self.edge_endpoints(edge_idx), self.edge_weight(edge_idx))
        {
            Some((endpoints.0, endpoints.1, relation))
        } else {
            None
        }
    }

    /// Returns an iterator over nodes in a containment scope with their indices.
    ///
    /// Each item in the iterator is a tuple of (NodeIndex, &ast::Node).
    pub fn containment_scope_nodes_with_indices(
        &self,
        containment_scope: &ContainmentScope,
    ) -> impl Iterator<Item = (NodeIndex, &ast::Node)> {
        containment_scope.nodes.iter().map(|&idx| {
            (
                idx,
                self.graph
                    .node_weight(idx)
                    .expect("Node index should exist"),
            )
        })
    }

    /// Returns an iterator over relations in a containment scope.
    pub fn containment_scope_relations(
        &self,
        containment_scope: &ContainmentScope,
    ) -> impl Iterator<Item = &ast::Relation> {
        containment_scope.relations.iter().map(|&idx| {
            self.graph
                .edge_weight(idx)
                .expect("Edge index should exist")
        })
    }

    /// Returns an iterator over relation endpoints in a containment scope.
    ///
    /// Each item in the iterator is a tuple of (EdgeIndex, source_node, target_node).
    pub fn containment_scope_relation_endpoint_indices(
        &self,
        containment_scope: &ContainmentScope,
    ) -> impl Iterator<Item = (EdgeIndex, NodeIndex, NodeIndex)> {
        containment_scope.relations.iter().map(|&idx| {
            let (source, target) = self
                .graph
                .edge_endpoints(idx)
                .expect("Edge index should exist");
            (idx, source, target)
        })
    }

    fn add_node(&mut self, node: &ast::Node) -> NodeIndex {
        self.graph.add_node(node.clone())
    }

    fn add_edge(
        &mut self,
        source: NodeIndex,
        target: NodeIndex,
        relation: &ast::Relation,
    ) -> EdgeIndex {
        self.graph.add_edge(source, target, relation.clone())
    }

    // pub fn container_elements_in_post_order(
    //     &self,
    // ) -> impl Iterator<Item = (Option<&ast::TypeId>, &Graph)> {
    //     DfsPostOrder::new(&self.hierarchy, self.hierarchy_root.unwrap())
    //         .iter(&self.hierarchy)
    //         .map(|idx| {
    //             (
    //                 self.hierarchy
    //                     .first_edge(idx, Direction::Incoming)
    //                     .map(|edge_idx| self.hierarchy.edge_weight(edge_idx).unwrap()),
    //                 self.hierarchy.node_weight(idx).unwrap(),
    //             )
    //         })
    // }
}

impl<'a> Collection<'a> {
    /// Convert a diagram to a graph, recursively processing nested blocks
    pub fn from_diagram(diagram: &'a ast::Diagram) -> Result<Self, FilamentError> {
        let mut collection = Self {
            diagram_tree: DiGraph::new(),
            root_diagram: None,
        };

        // Process all elements in the diagram recursively
        let root_diagram = collection.add_diagram_to_tree(diagram)?;
        collection.root_diagram = Some(root_diagram);

        trace!(collection:?; "Created collection from diagram");

        Ok(collection)
    }

    pub fn diagram_tree_in_post_order(
        &self,
    ) -> impl Iterator<Item = (Option<&ast::TypeId>, &Graph<'_>)> {
        DfsPostOrder::new(&self.diagram_tree, self.root_diagram.unwrap())
            .iter(&self.diagram_tree)
            .map(|idx| {
                (
                    self.diagram_tree
                        .first_edge(idx, Direction::Incoming)
                        .map(|edge_idx| self.diagram_tree.edge_weight(edge_idx).unwrap()),
                    self.diagram_tree.node_weight(idx).unwrap(),
                )
            })
    }

    /// Process a list of elements and add nodes and relations to the graph
    /// Returns processed node indices for the current level and any hierarchy children
    fn process_containment_scope(
        &mut self,
        graph: &mut Graph<'a>,
        elements: &'a [ast::Element],
        container: Option<NodeIndex>,
    ) -> Result<Vec<(ast::TypeId, NodeIndex)>, FilamentError> {
        let mut hierarchy_children = vec![];

        let mut containment_scope = ContainmentScope::new(container);

        // First pass: add all nodes to the graph
        for element in elements {
            if let ast::Element::Node(node) = element {
                let node_idx = graph.add_node(node);
                // Use ToString trait to convert TypeId to String
                graph.node_id_map.insert(node.id.clone(), node_idx);
                containment_scope.add_node(node_idx);

                // Process the node's inner block recursively
                match &node.block {
                    ast::Block::Scope(scope) => {
                        debug!(
                            "Processing nested scope with {} elements",
                            scope.elements.len()
                        );
                        let mut inner_hierarchy_children =
                            self.process_containment_scope(graph, &scope.elements, Some(node_idx))?;
                        hierarchy_children.append(&mut inner_hierarchy_children);
                    }
                    ast::Block::Diagram(inner_diagram) => {
                        debug!("Processing nested diagram of kind {:?}", inner_diagram.kind);
                        let inner_hierarchy_child = self.add_diagram_to_tree(inner_diagram)?;
                        hierarchy_children.push((node.id.clone(), inner_hierarchy_child));
                    }
                    ast::Block::None => {}
                }
            }
        }

        // Second pass: add all relations and activate blocks to the graph
        for element in elements {
            match element {
                ast::Element::Relation(relation) => {
                    if let (Some(&source_idx), Some(&target_idx)) = (
                        graph.node_id_map.get(&relation.source),
                        graph.node_id_map.get(&relation.target),
                    ) {
                        let edge_idx = graph.add_edge(source_idx, target_idx, relation);
                        containment_scope.add_relation(edge_idx);
                        graph.ordered_events.push(Event::Relation(edge_idx))
                    } else {
                        return Err(FilamentError::Graph(format!(
                            "Warning: Relation refers to undefined nodes: {} -> {}",
                            relation.source, relation.target
                        )));
                    }
                }
                ast::Element::ActivateBlock(activate_block) => {
                    let node_idx = *graph
                        .node_id_map()
                        .get(&activate_block.component)
                        .expect("Node map is missing");

                    graph.ordered_events.push(Event::Activate(node_idx));

                    // Recursively process elements within the activate block
                    let mut inner_hierarchy_children = self.process_containment_scope(
                        graph,
                        &activate_block.scope.elements,
                        container,
                    )?;
                    hierarchy_children.append(&mut inner_hierarchy_children);

                    graph.ordered_events.push(Event::Deactivate(node_idx));
                }
                ast::Element::Node(..) => (),
            }
        }

        graph.containment_scopes.push(containment_scope);

        Ok(hierarchy_children)
    }

    fn add_diagram_to_tree(
        &mut self,
        diagram: &'a ast::Diagram,
    ) -> Result<NodeIndex, FilamentError> {
        let mut graph = Graph::new(diagram);
        let hierarchy_children =
            self.process_containment_scope(&mut graph, &diagram.scope.elements, None)?;

        let hierarchy_idx = self.diagram_tree.add_node(graph);
        for child in hierarchy_children {
            self.diagram_tree.add_edge(hierarchy_idx, child.1, child.0);
        }

        Ok(hierarchy_idx)
    }
}
