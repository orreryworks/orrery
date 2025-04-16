use crate::ast::elaborate::{Diagram, Element, Node, Relation};
use crate::error::FilamentError;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

pub fn diagram_to_graph(diagram: &Diagram) -> Result<DiGraph<Node, Relation>, FilamentError> {
    let mut graph = DiGraph::new();
    let mut node_map = HashMap::new();
    for element in &diagram.scope.elements {
        match element {
            Element::Node(node) => {
                let node_idx = graph.add_node(node.clone());
                node_map.insert(&node.id, node_idx);
            }
            Element::Relation(relation) => {
                if let (Some(&source_idx), Some(&target_idx)) = (
                    node_map.get(&relation.source),
                    node_map.get(&relation.target),
                ) {
                    graph.add_edge(source_idx, target_idx, relation.clone());
                } else {
                    return Err(FilamentError::GraphError(format!(
                        "Warning: Relation refers to undefined nodes: {:?} -> {:?}",
                        relation.source, relation.target
                    )));
                }
            }
        }
    }

    Ok(graph)
}
