use crate::{
    ast::elaborate::{Node, Relation},
    layout::common::{calculate_element_size, Component, Point, Size},
};
use petgraph::{
    graph::{DiGraph, NodeIndex},
    Direction,
};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug)]
pub struct LayoutRelation<'a> {
    pub relation: &'a Relation,
    source_index: usize,
    target_index: usize,
}

#[derive(Debug)]
pub struct Layout<'a> {
    pub components: Vec<Component<'a>>,
    pub relations: Vec<LayoutRelation<'a>>,
}

impl<'a> Layout<'a> {
    pub fn source(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.source_index]
    }

    pub fn target(&self, lr: &LayoutRelation<'a>) -> &Component<'a> {
        &self.components[lr.target_index]
    }
}

pub struct Engine {
    padding: f32,
    min_component_width: f32,
    min_component_height: f32,
    text_padding: f32,
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            padding: 40.0,
            min_component_width: 100.0,
            min_component_height: 60.0,
            text_padding: 20.0,
        }
    }

    pub fn calculate<'a>(&self, graph: &'a DiGraph<Node, Relation>) -> Layout<'a> {
        let component_sizes: HashMap<_, _> = graph
            .node_indices()
            .map(|node_idx| {
                let node = graph.node_weight(node_idx).unwrap();
                let size = calculate_element_size(
                    node,
                    self.min_component_width,
                    self.min_component_height,
                    self.text_padding,
                );
                (node_idx, size)
            })
            .collect();

        let positions = self.positions(graph, &component_sizes);

        let components: Vec<Component<'a>> = graph
            .node_indices()
            .map(|node_idx| {
                let position = positions.get(&node_idx).unwrap();
                let node = graph.node_weight(node_idx).unwrap();
                let size = component_sizes.get(&node_idx).unwrap().clone();
                Component {
                    node,
                    position: *position,
                    size,
                }
            })
            .collect();

        let component_indices: HashMap<_, _> = components
            .iter()
            .enumerate()
            .map(|(idx, component)| (&component.node.id, idx))
            .collect();

        let relations: Vec<LayoutRelation<'a>> = graph
            .edge_indices()
            .map(|edge_idx| {
                let relation = graph.edge_weight(edge_idx).unwrap();
                let source_index = *component_indices.get(&relation.source).unwrap();
                let target_index = *component_indices.get(&relation.target).unwrap();
                LayoutRelation {
                    source_index,
                    target_index,
                    relation,
                }
            })
            .collect();

        Layout {
            components,
            relations,
        }
    }

    fn positions(
        &self,
        graph: &DiGraph<Node, Relation>,
        sizes: &HashMap<NodeIndex, Size>,
    ) -> HashMap<NodeIndex, Point> {
        let mut positions = HashMap::new();
        let layers = self.assign_layers(graph);

        // Calculate max width for each layer
        let layer_widths: Vec<f32> = layers
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|node_idx| sizes.get(node_idx).unwrap().width)
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(self.min_component_width)
            })
            .collect();

        // Calculate starting x position for each layer
        let mut layer_x_positions = Vec::with_capacity(layers.len());
        let mut x_pos = 0.0;
        for width in &layer_widths {
            layer_x_positions.push(x_pos + width / 2.0);
            x_pos += width + self.padding;
        }

        // For each layer, calculate positions
        for (layer_idx, layer_nodes) in layers.iter().enumerate() {
            let x = layer_x_positions[layer_idx];

            // Calculate heights for vertical positioning
            let mut y_pos = 0.0;
            for (j, node_idx) in layer_nodes.iter().enumerate() {
                let node_height = sizes.get(node_idx).unwrap().height;

                if j > 0 {
                    y_pos += self.padding; // Space between components
                }

                let y = y_pos + node_height / 2.0;
                positions.insert(*node_idx, Point { x, y });

                y_pos += node_height;
            }
        }

        positions
    }

    fn assign_layers(&self, graph: &DiGraph<Node, Relation>) -> Vec<Vec<NodeIndex>> {
        let mut layers = Vec::new();
        let mut visited = HashSet::new();

        // Find root nodes
        let root_nodes: Vec<_> = graph
            .node_indices()
            .filter(|&idx| graph.neighbors_directed(idx, Direction::Incoming).count() == 0)
            .collect();
        let start_nodes = if root_nodes.is_empty() {
            graph.node_indices().take(1).collect()
        } else {
            root_nodes
        };

        // Perform BFS to assign layers
        let mut queue = VecDeque::new();
        for node in start_nodes {
            queue.push_back((node, 0));
        }

        while let Some((node_idx, layer)) = queue.pop_front() {
            if visited.contains(&node_idx) {
                continue;
            }
            visited.insert(node_idx);
            while layers.len() <= layer {
                layers.push(Vec::new());
            }

            layers[layer].push(node_idx);

            for child in graph.neighbors(node_idx) {
                if !visited.contains(&child) {
                    queue.push_back((child, layer + 1));
                }
            }
        }

        layers
    }
}
