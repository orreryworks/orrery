use crate::{ast, error::FilamentError};
use log::debug;
use petgraph::{
    Direction,
    graph::{DiGraph, EdgeIndex, NodeIndex},
    visit::{DfsPostOrder, Walker},
};
use std::collections::HashMap;

pub struct Graph<'a> {
    graph: DiGraph<ast::Node, ast::Relation>,
    diagram: &'a ast::Diagram,
}

pub struct Collection<'a> {
    hierarchy: DiGraph<Graph<'a>, ast::TypeId>,
    hierarchy_root: Option<NodeIndex>,
}

impl<'a> Graph<'a> {
    fn new(diagram: &'a ast::Diagram) -> Self {
        Self {
            graph: DiGraph::new(),
            diagram,
        }
    }

    pub fn node_indices(&self) -> impl Iterator<Item = NodeIndex> {
        self.graph.node_indices()
    }

    pub fn node_weight(&self, node_index: NodeIndex) -> Option<&ast::Node> {
        self.graph.node_weight(node_index)
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

    pub fn diagram(&self) -> &ast::Diagram {
        self.diagram
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
}

impl<'a> Collection<'a> {
    /// Convert a diagram to a graph, recursively processing nested blocks
    pub fn from_diagram(diagram: &'a ast::Diagram) -> Result<Self, FilamentError> {
        let mut collection = Self {
            hierarchy: DiGraph::new(),
            hierarchy_root: None,
        };

        // Process all elements in the diagram recursively
        let hierarchy_root = collection.process_diagram_block(diagram)?;
        collection.hierarchy_root = Some(hierarchy_root);

        Ok(collection)
    }

    pub fn hierarchy_in_post_order(&self) -> impl Iterator<Item = (Option<&ast::TypeId>, &Graph)> {
        DfsPostOrder::new(&self.hierarchy, self.hierarchy_root.unwrap())
            .iter(&self.hierarchy)
            .map(|idx| {
                (
                    self.hierarchy
                        .first_edge(idx, Direction::Incoming)
                        .map(|edge_idx| self.hierarchy.edge_weight(edge_idx).unwrap()),
                    self.hierarchy.node_weight(idx).unwrap(),
                )
            })
    }

    /// Process a list of elements and add nodes and relations to the graph
    /// Returns processed node indices for the current level and any hierarchy children
    fn process_elements(
        &mut self,
        graph: &mut Graph,
        node_map: &mut HashMap<String, NodeIndex>,
        elements: &'a [ast::Element],
    ) -> Result<Vec<(ast::TypeId, NodeIndex)>, FilamentError> {
        let mut hierarchy_children = vec![];

        // First pass: add all nodes to the graph
        for element in elements {
            if let ast::Element::Node(node) = element {
                let node_idx = graph.add_node(node);
                // Use ToString trait to convert TypeId to String
                node_map.insert(node.id.to_string(), node_idx);

                // Process the node's inner block recursively
                match &node.block {
                    ast::Block::Scope(scope) => {
                        debug!(
                            "Processing nested scope with {} elements",
                            scope.elements.len()
                        );
                        let mut inner_hierarchy_children =
                            self.process_elements(graph, node_map, &scope.elements)?;
                        hierarchy_children.append(&mut inner_hierarchy_children);
                    }
                    ast::Block::Diagram(inner_diagram) => {
                        debug!("Processing nested diagram of kind {:?}", inner_diagram.kind);
                        let inner_hierarchy_child = self.process_diagram_block(inner_diagram)?;
                        hierarchy_children.push((node.id.clone(), inner_hierarchy_child));
                    }
                    ast::Block::None => {}
                }
            }
        }

        // Second pass: add all relations to the graph
        for element in elements {
            if let ast::Element::Relation(relation) = element {
                if let (Some(&source_idx), Some(&target_idx)) = (
                    node_map.get(&relation.source.to_string()),
                    node_map.get(&relation.target.to_string()),
                ) {
                    graph.add_edge(source_idx, target_idx, relation);
                } else {
                    return Err(FilamentError::Graph(format!(
                        "Warning: Relation refers to undefined nodes: {} -> {}",
                        relation.source, relation.target
                    )));
                }
            }
        }

        Ok(hierarchy_children)
    }

    fn process_diagram_block(
        &mut self,
        diagram: &'a ast::Diagram,
    ) -> Result<NodeIndex, FilamentError> {
        let mut graph = Graph::new(diagram);
        let mut node_map = HashMap::new();
        let hierarchy_children =
            self.process_elements(&mut graph, &mut node_map, &diagram.scope.elements)?;

        let hierarchy_idx = self.hierarchy.add_node(graph);
        for child in hierarchy_children {
            self.hierarchy.add_edge(hierarchy_idx, child.1, child.0);
        }

        Ok(hierarchy_idx)
    }
}
