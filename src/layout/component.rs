use crate::{
    ast,
    graph::Graph,
    layout::common::{Component, Point, Size, calculate_element_size},
};
use petgraph::{
    Direction,
    graph::{DiGraph, NodeIndex},
};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug)]
pub struct LayoutRelation<'a> {
    pub relation: &'a ast::Relation,
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

    pub fn calculate<'a>(&self, graph: &'a Graph) -> Layout<'a> {
        // First, build a map of parent-child relationships
        // This will help us understand the hierarchy in the graph
        let hierarchy_map = self.build_hierarchy_map(graph);

        // Calculate sizes for all components, adjusting for nested children
        let component_sizes = self.calculate_component_sizes(graph, &hierarchy_map);

        // Calculate positions for all components
        let positions = self.positions(graph, &component_sizes, &hierarchy_map);

        // Build the final component list with proper node references
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

        // Map node IDs to their component indices
        let component_indices: HashMap<_, _> = components
            .iter()
            .enumerate()
            .map(|(idx, component)| (&component.node.id, idx))
            .collect();

        // Build the list of relations between components
        let relations: Vec<LayoutRelation<'a>> = graph
            .edge_indices()
            .filter_map(|edge_idx| {
                let relation = graph.edge_weight(edge_idx).unwrap();

                // Only include relations between visible components
                // (not including relations within inner blocks)
                if let (Some(&source_index), Some(&target_index)) = (
                    component_indices.get(&relation.source),
                    component_indices.get(&relation.target),
                ) {
                    Some(LayoutRelation {
                        relation,
                        source_index,
                        target_index,
                    })
                } else {
                    None
                }
            })
            .collect();

        Layout {
            components,
            relations,
        }
    }

    /// Build a map of parent-child relationships between nodes
    fn build_hierarchy_map(&self, graph: &Graph) -> HashMap<NodeIndex, Vec<NodeIndex>> {
        let mut hierarchy_map: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
        let mut node_id_map: HashMap<String, NodeIndex> = HashMap::new();

        // First, create a mapping of node ID strings to node indices
        for node_idx in graph.node_indices() {
            let node = graph.node_weight(node_idx).unwrap();
            node_id_map.insert(node.id.to_string(), node_idx);
        }

        // Then, for each node, determine its children based on block content
        for node_idx in graph.node_indices() {
            let node = graph.node_weight(node_idx).unwrap();

            // Skip nodes that don't have blocks
            match &node.block {
                ast::Block::None => {}
                ast::Block::Scope(scope) => {
                    // Find all nodes that are part of this node's scope
                    let mut children = Vec::new();

                    for element in &scope.elements {
                        if let ast::Element::Node(inner_node) = element {
                            if let Some(&child_idx) = node_id_map.get(&inner_node.id.to_string()) {
                                children.push(child_idx);
                            }
                        }
                    }

                    if !children.is_empty() {
                        hierarchy_map.insert(node_idx, children);
                    }
                }
                ast::Block::Diagram(diagram) => {
                    // Find all nodes that are part of this node's diagram
                    let mut children = Vec::new();

                    for element in &diagram.scope.elements {
                        if let ast::Element::Node(inner_node) = element {
                            if let Some(&child_idx) = node_id_map.get(&inner_node.id.to_string()) {
                                children.push(child_idx);
                            }
                        }
                    }

                    if !children.is_empty() {
                        hierarchy_map.insert(node_idx, children);
                    }
                }
            }
        }

        hierarchy_map
    }

    /// Calculate component sizes considering nested elements
    fn calculate_component_sizes(
        &self,
        graph: &Graph,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
    ) -> HashMap<NodeIndex, Size> {
        let mut component_sizes: HashMap<NodeIndex, Size> = HashMap::new();

        // First, calculate base sizes for all nodes
        for node_idx in graph.node_indices() {
            let node = graph.node_weight(node_idx).unwrap();
            let size = calculate_element_size(
                node,
                self.min_component_width,
                self.min_component_height,
                self.text_padding,
            );
            component_sizes.insert(node_idx, size);
        }

        // For nodes with children, ensure they're large enough to contain their children
        // Start with leaf nodes and work up
        let mut visited = std::collections::HashSet::new();
        for node_idx in graph.node_indices() {
            self.adjust_container_size(node_idx, hierarchy_map, &mut component_sizes, &mut visited);
        }

        component_sizes
    }

    /// Recursively adjust container sizes to fit their children
    fn adjust_container_size(
        &self,
        node_idx: NodeIndex,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
        sizes: &mut HashMap<NodeIndex, Size>,
        visited: &mut std::collections::HashSet<NodeIndex>,
    ) -> Size {
        // If we've already processed this node, return its size
        if visited.contains(&node_idx) {
            return sizes[&node_idx].clone();
        }

        // If this node has children, adjust its size based on children's sizes
        if let Some(children) = hierarchy_map.get(&node_idx) {
            if children.is_empty() {
                // No children, just mark as visited and return current size
                visited.insert(node_idx);
                return sizes[&node_idx].clone();
            }

            // Process all children first to get their sizes
            let mut max_width = 0.0f32;
            let mut max_height = 0.0f32;

            for &child_idx in children {
                let child_size =
                    self.adjust_container_size(child_idx, hierarchy_map, sizes, visited);
                max_width = max_width.max(child_size.width);
                max_height = max_height.max(child_size.height);
            }

            // Add padding and adjust for layout arrangement
            // Use a simple heuristic - whichever dimension has more elements
            let container_padding = self.padding * 2.0; // Padding on all sides
            let mut required_width = max_width + container_padding;
            let mut required_height = max_height + container_padding;

            // If we have multiple children, consider arranging them in a grid
            if children.len() > 1 {
                let sqrt_count = (children.len() as f64).sqrt().ceil() as usize;
                required_width =
                    max_width * sqrt_count as f32 + self.padding * (sqrt_count + 1) as f32;
                required_height = max_height * children.len().div_ceil(sqrt_count) as f32
                    + self.padding * (children.len().div_ceil(sqrt_count) + 1) as f32;
            }

            // Get the current size and ensure it's big enough
            let current_size = &sizes[&node_idx];
            let new_size = Size {
                width: current_size.width.max(required_width),
                height: current_size.height.max(required_height),
            };

            // Update the size and mark as visited
            sizes.insert(node_idx, new_size.clone());
            visited.insert(node_idx);
            return new_size;
        }

        // No children, just mark as visited and return current size
        visited.insert(node_idx);
        sizes[&node_idx].clone()
    }

    fn positions(
        &self,
        graph: &Graph,
        sizes: &HashMap<NodeIndex, Size>,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
    ) -> HashMap<NodeIndex, Point> {
        let mut positions = HashMap::new();

        // First, identify all top-level nodes (nodes that aren't children of any other node)
        let mut is_child = std::collections::HashSet::new();
        for children in hierarchy_map.values() {
            for &child in children {
                is_child.insert(child);
            }
        }

        let top_level_nodes: Vec<_> = graph
            .node_indices()
            .filter(|&idx| !is_child.contains(&idx))
            .collect();

        // Create layers for top-level nodes only
        let mut filtered_graph = DiGraph::<(), ()>::new();
        let mut node_map = HashMap::new();

        // Add only top-level nodes to filtered graph
        for &node_idx in &top_level_nodes {
            let new_idx = filtered_graph.add_node(());
            node_map.insert(node_idx, new_idx);
        }

        // Add edges between top-level nodes
        for edge_idx in graph.edge_indices() {
            let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
            if top_level_nodes.contains(&source) && top_level_nodes.contains(&target) {
                if let (Some(&src_idx), Some(&tgt_idx)) =
                    (node_map.get(&source), node_map.get(&target))
                {
                    filtered_graph.add_edge(src_idx, tgt_idx, ());
                }
            }
        }

        // Assign layers for top-level nodes
        let layers = self.assign_layers_for_graph(&filtered_graph, &node_map);

        // Calculate max width for each layer
        let layer_widths: Vec<f32> = layers
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|&node_idx| sizes.get(&node_idx).unwrap().width)
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

        // For each layer, calculate positions for top-level nodes
        for (layer_idx, layer_nodes) in layers.iter().enumerate() {
            let x = layer_x_positions[layer_idx];

            // Calculate heights for vertical positioning
            let mut y_pos = 0.0;
            for (j, &node_idx) in layer_nodes.iter().enumerate() {
                let node_height = sizes.get(&node_idx).unwrap().height;

                if j > 0 {
                    y_pos += self.padding; // Space between components
                }

                let y = y_pos + node_height / 2.0;
                positions.insert(node_idx, Point { x, y });

                y_pos += node_height;
            }
        }

        // Create a copy of positions for the recursive calls
        let mut result_positions = positions.clone();

        // Now recursively position children within their parent containers
        for &node_idx in &top_level_nodes {
            if let Some(children) = hierarchy_map.get(&node_idx) {
                // Clone just the positions we need for recursive calls to avoid borrow issues
                let parent_pos = result_positions[&node_idx];

                self.position_children_within_parent(
                    node_idx,
                    parent_pos,
                    children,
                    sizes,
                    hierarchy_map,
                    &mut result_positions,
                );
            }
        }

        result_positions
    }

    /// Helper method to position children within their parent container without borrowing conflicts
    fn position_children_within_parent(
        &self,
        parent_idx: NodeIndex,
        parent_pos: Point,
        children: &[NodeIndex],
        sizes: &HashMap<NodeIndex, Size>,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
        result_positions: &mut HashMap<NodeIndex, Point>,
    ) {
        if children.is_empty() {
            return;
        }

        // Get parent size
        let parent_size = sizes.get(&parent_idx).unwrap();

        // Calculate area available for children
        let container_padding = self.padding;
        let available_width = parent_size.width - (container_padding * 2.0);
        let available_height = parent_size.height - (container_padding * 2.0);

        // Determine layout arrangement (simple grid layout for now)
        let sqrt_count = (children.len() as f64).sqrt().ceil() as usize;
        let cols = sqrt_count;
        let rows = children.len().div_ceil(cols);

        // Calculate cell dimensions
        let cell_width = available_width / cols as f32;
        let cell_height = available_height / rows as f32;

        // Calculate starting point (top-left of container area)
        let start_x = parent_pos.x - (parent_size.width / 2.0) + container_padding;
        let start_y = parent_pos.y - (parent_size.height / 2.0) + container_padding;

        // Position each child
        for (i, &child_idx) in children.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;

            // Calculate center position of this cell
            let cell_center_x = start_x + (col as f32 * cell_width) + (cell_width / 2.0);
            let cell_center_y = start_y + (row as f32 * cell_height) + (cell_height / 2.0);

            // Position the child at the cell center
            let child_pos = Point {
                x: cell_center_x,
                y: cell_center_y,
            };
            result_positions.insert(child_idx, child_pos);

            // Recursively position this child's children (if any)
            if let Some(grandchildren) = hierarchy_map.get(&child_idx) {
                self.position_children_within_parent(
                    child_idx,
                    child_pos,
                    grandchildren,
                    sizes,
                    hierarchy_map,
                    result_positions,
                );
            }
        }
    }

    /// Helper method to assign layers for a specific graph
    fn assign_layers_for_graph(
        &self,
        graph: &DiGraph<(), ()>,
        node_map: &HashMap<NodeIndex, NodeIndex>,
    ) -> Vec<Vec<NodeIndex>> {
        let mut layers = Vec::new();
        let mut visited = HashSet::new();
        let reverse_map: HashMap<_, _> = node_map.iter().map(|(&k, &v)| (v, k)).collect();

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

            // Map back to original node index
            if let Some(&original_idx) = reverse_map.get(&node_idx) {
                layers[layer].push(original_idx);
            }

            for child in graph.neighbors(node_idx) {
                if !visited.contains(&child) {
                    queue.push_back((child, layer + 1));
                }
            }
        }

        layers
    }
}
