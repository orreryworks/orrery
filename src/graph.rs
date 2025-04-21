use crate::{
    ast::elaborate::{Block, Diagram, Element, Node, Relation},
    error::FilamentError,
};
use log::debug;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use std::collections::HashMap;

pub struct Graph {
    graph: DiGraph<Node, Relation>,
}

impl Graph {
    /// Convert a diagram to a graph, recursively processing nested blocks
    pub fn from_diagram(diagram: &Diagram) -> Result<Self, FilamentError> {
        let mut graph = Graph {
            graph: DiGraph::new(),
        };
        let mut node_map = HashMap::new();

        // Process all elements in the diagram recursively
        graph.process_elements(&mut node_map, &diagram.scope.elements)?;

        Ok(graph)
    }

    pub fn node_indices(&self) -> impl Iterator<Item = NodeIndex> {
        self.graph.node_indices()
    }

    pub fn node_weight(&self, node_index: NodeIndex) -> Option<&Node> {
        self.graph.node_weight(node_index)
    }

    pub fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex> {
        self.graph.edge_indices()
    }

    pub fn edge_weight(&self, edge_index: EdgeIndex) -> Option<&Relation> {
        self.graph.edge_weight(edge_index)
    }

    pub fn edge_endpoints(&self, edge_index: EdgeIndex) -> Option<(NodeIndex, NodeIndex)> {
        self.graph.edge_endpoints(edge_index)
    }

    /// Process a list of elements and add them to the graph
    fn process_elements(
        &mut self,
        node_map: &mut HashMap<String, NodeIndex>,
        elements: &[Element],
    ) -> Result<(), FilamentError> {
        // First pass: add all nodes to the graph
        for element in elements {
            if let Element::Node(node) = element {
                let node_idx = self.graph.add_node(node.clone());
                // Use ToString trait to convert TypeId to String
                node_map.insert(node.id.to_string(), node_idx);

                // Process the node's inner block recursively
                match &node.block {
                    Block::Scope(scope) => {
                        debug!(
                            "Processing nested scope with {} elements",
                            scope.elements.len()
                        );
                        self.process_elements(node_map, &scope.elements)?;
                    }
                    Block::Diagram(inner_diagram) => {
                        debug!("Processing nested diagram of kind {:?}", inner_diagram.kind);
                        self.process_elements(node_map, &inner_diagram.scope.elements)?;
                    }
                    Block::None => {}
                }
            }
        }

        // Second pass: add all relations to the graph
        for element in elements {
            if let Element::Relation(relation) = element {
                if let (Some(&source_idx), Some(&target_idx)) = (
                    node_map.get(&relation.source.to_string()),
                    node_map.get(&relation.target.to_string()),
                ) {
                    self.graph
                        .add_edge(source_idx, target_idx, relation.clone());
                } else {
                    return Err(FilamentError::GraphError(format!(
                        "Warning: Relation refers to undefined nodes: {} -> {}",
                        relation.source, relation.target
                    )));
                }
            }
        }

        Ok(())
    }
}
