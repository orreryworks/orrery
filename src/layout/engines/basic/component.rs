//! Basic component layout engine
//!
//! This module provides a layout engine for component diagrams
//! using a simple, deterministic algorithm.

use crate::{
    ast,
    graph::Graph,
    layout::{
        common::{Component, Point, Size},
        component::{Layout, LayoutRelation},
        engines::{ComponentEngine, EmbeddedLayouts, LayoutResult},
        positioning::calculate_element_size,
        text,
    },
};
use petgraph::{
    Direction,
    graph::{DiGraph, NodeIndex},
};
use std::collections::{HashMap, HashSet, VecDeque};

/// Basic component layout engine implementation that implements the ComponentLayoutEngine trait
pub struct Engine {
    padding: f32,
    min_component_width: f32,
    min_component_height: f32,
    text_padding: f32,
    min_spacing: f32,
}

impl Engine {
    /// Create a new basic component layout engine
    pub fn new() -> Self {
        Self {
            padding: 40.0,
            min_component_width: 100.0,
            min_component_height: 60.0,
            text_padding: 20.0,
            min_spacing: 40.0,
        }
    }
    
    /// Set the padding around components
    pub fn set_padding(&mut self, padding: f32) -> &mut Self {
        self.padding = padding;
        self
    }
    
    /// Set the minimum width for components
    #[allow(dead_code)]
    pub fn set_min_width(&mut self, width: f32) -> &mut Self {
        self.min_component_width = width;
        self
    }
    
    /// Set the minimum height for components
    #[allow(dead_code)]
    pub fn set_min_height(&mut self, height: f32) -> &mut Self {
        self.min_component_height = height;
        self
    }
    
    /// Set the padding for text elements
    #[allow(dead_code)]
    pub fn set_text_padding(&mut self, padding: f32) -> &mut Self {
        self.text_padding = padding;
        self
    }
    
    /// Set the minimum spacing between components
    pub fn set_min_spacing(&mut self, spacing: f32) -> &mut Self {
        self.min_spacing = spacing;
        self
    }

    /// Calculate the layout for a component diagram
    pub fn calculate_layout<'a>(&self, graph: &'a Graph<'a>, embedded_layouts: &EmbeddedLayouts<'a>) -> Layout<'a> {
        // First, build a map of parent-child relationships
        // This will help us understand the hierarchy in the graph
        let hierarchy_map = self.build_hierarchy_map(graph);

        // Calculate sizes for all components, adjusting for nested children and embedded diagrams
        let component_sizes = self.calculate_component_sizes(graph, &hierarchy_map, embedded_layouts);

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
    fn build_hierarchy_map<'a>(&self, graph: &Graph<'a>) -> HashMap<NodeIndex, Vec<NodeIndex>> {
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
    /// Calculate sizes for all components, adjusting for nested children and embedded diagrams
    fn calculate_component_sizes<'a>(
        &self,
        graph: &Graph<'a>,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
        embedded_layouts: &EmbeddedLayouts<'_>,
    ) -> HashMap<NodeIndex, Size> {
        let mut component_sizes: HashMap<NodeIndex, Size> = HashMap::new();

        // First, calculate base sizes for all nodes
        for node_idx in graph.node_indices() {
            let node = graph.node_weight(node_idx).unwrap();
            
            // Check if this node has an embedded diagram
            let size = if let ast::Block::Diagram(_) = &node.block {
                // Since we process in post-order (innermost to outermost),
                // embedded diagram layouts should already be calculated and available
                if let Some(layout) = embedded_layouts.get(&node.id) {
                    // Get the bounding box for the embedded layout
                    self.get_layout_size(layout)
                } else {
                    // Fallback if no embedded layout is found (shouldn't happen in normal flow)
                    calculate_element_size(
                        node,
                        self.min_component_width,
                        self.min_component_height,
                        self.text_padding,
                    )
                }
            } else {
                // Standard size calculation for regular nodes
                calculate_element_size(
                    node,
                    self.min_component_width,
                    self.min_component_height,
                    self.text_padding,
                )
            };
            
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

    fn positions<'a>(
        &self,
        graph: &Graph<'a>,
        sizes: &HashMap<NodeIndex, Size>,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
    ) -> HashMap<NodeIndex, Point> {
        // Step 1: Find top-level nodes and create a simplified graph
        let (top_level_nodes, filtered_graph, node_map) =
            self.identify_top_level_nodes(graph, hierarchy_map);

        // Step 2: Assign layers for the top-level nodes
        let layers = self.assign_layers_for_graph(&filtered_graph, &node_map);

        // Step 3: Calculate layer metrics (widths and spacings)
        let (layer_widths, layer_spacings) =
            self.calculate_layer_metrics(graph, &layers, &top_level_nodes, sizes);

        // Step 4: Calculate X positions for each layer
        let layer_x_positions = self.calculate_layer_x_positions(&layer_widths, &layer_spacings);

        // Step 5: Position top-level nodes within their layers
        let positions = self.position_nodes_in_layers(&layers, &layer_x_positions, sizes);

        // Step 6: Position child nodes within their parent containers
        let mut result_positions = positions.clone();
        self.position_all_children(
            hierarchy_map,
            &top_level_nodes,
            sizes,
            &mut result_positions,
        );

        result_positions
    }

    /// Identify top-level nodes and create a simplified graph containing only those nodes
    fn identify_top_level_nodes<'a>(
        &self,
        graph: &Graph<'a>,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
    ) -> (
        Vec<NodeIndex>,
        DiGraph<(), ()>,
        HashMap<NodeIndex, NodeIndex>,
    ) {
        // Identify all nodes that are children of some other node
        let mut is_child = std::collections::HashSet::new();
        for children in hierarchy_map.values() {
            for &child in children {
                is_child.insert(child);
            }
        }

        // Find top-level nodes (nodes that aren't children)
        let top_level_nodes: Vec<_> = graph
            .node_indices()
            .filter(|&idx| !is_child.contains(&idx))
            .collect();

        // Create simplified graph with only top-level nodes
        let mut filtered_graph = DiGraph::<(), ()>::new();
        let mut node_map = HashMap::new();

        // Add top-level nodes to filtered graph
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

        (top_level_nodes, filtered_graph, node_map)
    }

    /// Calculate metrics for each layer: widths and spacings between layers
    fn calculate_layer_metrics(
        &self,
        graph: &Graph,
        layers: &[Vec<NodeIndex>],
        top_level_nodes: &[NodeIndex],
        sizes: &HashMap<NodeIndex, Size>,
    ) -> (Vec<f32>, Vec<f32>) {
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

        // Initialize spacings with default padding
        let mut layer_spacings = vec![self.padding; layers.len().saturating_sub(1)];

        // Collect relations between top-level nodes to consider label spacing
        let top_level_relations = graph.edge_indices().filter_map(|edge_idx| {
            let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
            if top_level_nodes.contains(&source) && top_level_nodes.contains(&target) {
                Some(graph.edge_weight(edge_idx).unwrap())
            } else {
                None
            }
        });

        // Adjust spacings based on relation labels
        for relation in top_level_relations {
            if let Some(label) = &relation.label {
                let label_width = text::calculate_text_size(label, 14).width;

                // Find layers for source and target nodes
                let (source_layer, target_layer) = self.find_node_layers(graph, relation, layers);

                if let (Some(src), Some(tgt)) = (source_layer, target_layer) {
                    if src != tgt {
                        // Only adjust spacing for relations between different layers
                        let min_layer = src.min(tgt);
                        let needed_spacing = label_width + 30.0; // Add some padding

                        // Update spacing if label requires more space
                        if min_layer < layer_spacings.len() {
                            layer_spacings[min_layer] =
                                layer_spacings[min_layer].max(needed_spacing);
                        }
                    }
                }
            }
        }

        (layer_widths, layer_spacings)
    }

    /// Find which layer contains nodes for a given relation
    fn find_node_layers(
        &self,
        graph: &Graph,
        relation: &ast::Relation,
        layers: &[Vec<NodeIndex>],
    ) -> (Option<usize>, Option<usize>) {
        let mut source_layer = None;
        let mut target_layer = None;

        for (layer_idx, layer_nodes) in layers.iter().enumerate() {
            for node_idx in layer_nodes {
                let node = graph.node_weight(*node_idx).unwrap();
                if node.id == relation.source {
                    source_layer = Some(layer_idx);
                }
                if node.id == relation.target {
                    target_layer = Some(layer_idx);
                }
            }
        }

        (source_layer, target_layer)
    }

    /// Calculate X positions for each layer based on widths and spacings
    fn calculate_layer_x_positions(
        &self,
        layer_widths: &[f32],
        layer_spacings: &[f32],
    ) -> Vec<f32> {
        let mut layer_x_positions = Vec::with_capacity(layer_widths.len());
        let mut x_pos = 0.0;

        for (i, width) in layer_widths.iter().enumerate() {
            layer_x_positions.push(x_pos + width / 2.0);
            let spacing = if i < layer_spacings.len() {
                layer_spacings[i]
            } else {
                self.padding
            };
            x_pos += width + spacing;
        }

        layer_x_positions
    }

    /// Position nodes within their layers
    fn position_nodes_in_layers(
        &self,
        layers: &[Vec<NodeIndex>],
        layer_x_positions: &[f32],
        sizes: &HashMap<NodeIndex, Size>,
    ) -> HashMap<NodeIndex, Point> {
        let mut positions = HashMap::new();

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

        positions
    }

    /// Position all children nodes within their parent containers
    fn position_all_children(
        &self,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
        top_level_nodes: &[NodeIndex],
        sizes: &HashMap<NodeIndex, Size>,
        result_positions: &mut HashMap<NodeIndex, Point>,
    ) {
        for &node_idx in top_level_nodes {
            if let Some(children) = hierarchy_map.get(&node_idx) {
                let parent_pos = result_positions[&node_idx];
                self.position_children_within_parent(
                    node_idx,
                    parent_pos,
                    children,
                    sizes,
                    hierarchy_map,
                    result_positions,
                );
            }
        }
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
        let available_width = container_padding.mul_add(-2.0, parent_size.width); // parent_size.width - (container_padding * 2.0)
        let available_height = container_padding.mul_add(-2.0, parent_size.height); // parent_size.height - (container_padding * 2.0)

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
    
    /// Calculate the bounding box size for an embedded layout
    fn get_layout_size(&self, layout: &LayoutResult<'_>) -> Size {
        match layout {
            LayoutResult::Component(component_layout) => {
                // Calculate bounding box for all components
                let mut max_width: f32 = 0.0;
                let mut max_height: f32 = 0.0;
                
                for component in &component_layout.components {
                    let right = component.position.x + component.size.width;
                    let bottom = component.position.y + component.size.height;
                    
                    max_width = max_width.max(right);
                    max_height = max_height.max(bottom);
                }
                
                // Add padding for the container
                Size {
                    width: max_width + self.padding * 2.0,
                    height: max_height + self.padding * 2.0,
                }
            },
            LayoutResult::Sequence(sequence_layout) => {
                // For sequence diagrams, calculate width based on participants
                // and height based on the last message
                let mut max_width: f32 = 0.0;
                let mut max_height: f32 = 0.0;
                
                // Find the rightmost participant
                for participant in &sequence_layout.participants {
                    let right = participant.component.position.x + participant.component.size.width;
                    max_width = max_width.max(right);
                    
                    // Use the maximum lifeline height
                    max_height = max_height.max(participant.lifeline_end);
                }
                
                // Add padding for the container
                Size {
                    width: max_width + self.padding * 2.0,
                    height: max_height + self.padding * 2.0,
                }
            }
        }
    }
}

impl ComponentEngine for Engine {
    fn calculate<'a>(&self, graph: &'a Graph<'a>, embedded_layouts: &EmbeddedLayouts<'a>) -> Layout<'a> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
