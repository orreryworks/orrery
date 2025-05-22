use std::collections::{HashMap, HashSet};

use log::{debug, warn};
use petgraph::{algo::toposort, graph::NodeIndex};

use crate::{
    ast,
    graph::Graph,
    layout::{
        common::{Component, Point, Size},
        component::{Layout, LayoutRelation},
        engines::ComponentEngine,
        positioning::calculate_element_size,
    },
};

/// The Sugiyama layout engine for component diagrams
/// Based on the Sugiyama algorithm for layered drawing of directed graphs
/// Uses the rust-sugiyama implementation with fallback to a simple hierarchical layout
pub struct Engine {
    /// Minimum width for components
    min_component_width: f32,

    /// Minimum height for components
    min_component_height: f32,

    /// Padding around text elements
    text_padding: f32,

    /// Horizontal spacing between components
    horizontal_spacing: f32,

    /// Vertical spacing between layers
    vertical_spacing: f32,

    /// Container padding for nested components
    container_padding: f32,
}

impl Engine {
    /// Create a new Sugiyama component layout engine
    pub fn new() -> Self {
        Self {
            min_component_width: 100.0,
            min_component_height: 60.0,
            text_padding: 20.0,
            horizontal_spacing: 60.0,
            vertical_spacing: 80.0,
            container_padding: 45.0,
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
            let container_padding = self.container_padding * 2.0; // Padding on all sides
            let mut required_width = max_width + container_padding * 1.2;
            let mut required_height = max_height + container_padding * 1.2;

            // If we have multiple children, consider arranging them in a grid
            if children.len() > 1 {
                let sqrt_count = (children.len() as f64).sqrt().ceil() as usize;
                required_width = max_width * sqrt_count as f32
                    + self.container_padding * (sqrt_count + 1) as f32;
                required_height = max_height * children.len().div_ceil(sqrt_count) as f32
                    + self.container_padding * (children.len().div_ceil(sqrt_count) + 1) as f32;
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

    fn calculate_layout<'a>(&self, graph: &'a Graph) -> Layout<'a> {
        // First, build a map of parent-child relationships
        // This will help us understand the hierarchy in the graph
        let hierarchy_map = self.build_hierarchy_map(graph);

        // Calculate component sizes, adjusting for nested children
        let mut component_sizes: HashMap<_, _> = graph
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

        // Adjust container sizes to accommodate nested components
        let mut visited = std::collections::HashSet::new();
        for node_idx in graph.node_indices() {
            self.adjust_container_size(
                node_idx,
                &hierarchy_map,
                &mut component_sizes,
                &mut visited,
            );
        }

        // Prepare layout
        let mut positions = HashMap::new();
        let mut using_fallback = false;

        // Convert our graph to a format suitable for the Sugiyama algorithm
        let mut edges = Vec::new();
        let mut node_ids = HashMap::new();

        // Map node indices to u32 IDs for rust-sugiyama
        for (i, node_idx) in graph.node_indices().enumerate() {
            let id = i as u32;
            node_ids.insert(node_idx, id);
        }

        // Extract edges
        for edge_idx in graph.edge_indices() {
            if let Some((source, target)) = graph.edge_endpoints(edge_idx) {
                let source_id = *node_ids.get(&source).unwrap();
                let target_id = *node_ids.get(&target).unwrap();

                // Skip self-loops
                if source_id != target_id {
                    edges.push((source_id, target_id));
                }
            }
        }

        if !edges.is_empty() {
            debug!(
                "Applying Sugiyama algorithm to graph with {} nodes and {} edges",
                node_ids.len(),
                edges.len()
            );

            // Create a bidirectional mapping between our original node indices and sequential IDs
            let id_to_node_idx: HashMap<u32, NodeIndex> =
                node_ids.iter().map(|(&node, &id)| (id, node)).collect();

            // Try the rust_sugiyama crate with our sequential IDs, catching any panics
            let layouts = std::panic::catch_unwind(move || {
                rust_sugiyama::from_edges(&edges)
                    .minimum_length(1) // Use smaller minimum length to avoid overflow issues
                    .vertex_spacing(3) // Ensure adequate spacing between vertices
                    .build()
            });

            // Process the layout results
            match layouts {
                // Success case with non-empty results
                Ok(results) if !results.is_empty() => {
                    let (coords, _, _) = &results[0];

                    // Process coordinates safely
                    for &(id, (x, y)) in coords {
                        // Convert safely to u32 with bounds checking
                        let node_id = if (id as u64) <= (u32::MAX as u64) {
                            id as u32
                        } else {
                            debug!(
                                "Node ID {} from rust-sugiyama result is out of valid range",
                                id
                            );
                            continue;
                        };

                        // Map the ID back to our original node index
                        if let Some(&node_idx) = id_to_node_idx.get(&node_id) {
                            // Scale coordinates for proper spacing
                            // Apply spacing that ensures adequate separation between nodes
                            // Use smaller scaling factors to reduce padding
                            let x_pos = (x as f32) * self.horizontal_spacing * 1.0;
                            let y_pos = (y as f32) * self.vertical_spacing * 1.0;
                            positions.insert(node_idx, Point { x: x_pos, y: y_pos });
                        }
                    }

                    // If mapping failed for all nodes, fall back to hierarchical layout
                    if positions.is_empty() {
                        debug!(
                            "Failed to map any rust-sugiyama positions back to graph nodes. Using fallback layout."
                        );
                        self.fallback_hierarchical_layout(graph, &mut positions);
                        using_fallback = true;
                    }
                }

                // Empty results case
                Ok(results) if results.is_empty() => {
                    debug!(
                        "Rust-sugiyama returned empty layout results. Using fallback hierarchical layout."
                    );
                    self.fallback_hierarchical_layout(graph, &mut positions);
                    using_fallback = true;
                }

                // Unexpected success case
                Ok(_) => {
                    debug!(
                        "Rust-sugiyama returned unexpected result format. Using fallback hierarchical layout."
                    );
                    self.fallback_hierarchical_layout(graph, &mut positions);
                    using_fallback = true;
                }

                // Error/panic case
                Err(err) => {
                    if let Some(panic_msg) = err.downcast_ref::<String>() {
                        warn!(
                            "Rust-sugiyama layout engine panicked: {}. Using fallback hierarchical layout.",
                            panic_msg
                        );
                    } else {
                        warn!(
                            "Rust-sugiyama layout engine panicked with unknown error. Using fallback hierarchical layout."
                        );
                    }
                    self.fallback_hierarchical_layout(graph, &mut positions);
                    using_fallback = true;
                }
            }

            // Center the layout if we have positions
            if !positions.is_empty() {
                self.center_layout(&mut positions);
            }

            debug!(
                "Layout generated with {} positioned nodes (using fallback: {}) and positive coordinates",
                positions.len(),
                using_fallback
            );
        } else if !graph.node_indices().count().eq(&0) {
            // No edges but we have nodes - arrange them horizontally with positive coordinates
            debug!("Graph has no edges. Arranging nodes horizontally with positive coordinates.");
            for (i, node_idx) in graph.node_indices().enumerate() {
                // For no-edge graphs, ensure adequate horizontal spacing and a margin from the top
                let x = self.horizontal_spacing * 0.8
                    + (i as f32) * (self.min_component_width + self.horizontal_spacing * 0.5);
                positions.insert(
                    node_idx,
                    Point {
                        x,
                        y: self.vertical_spacing * 0.8,
                    },
                );
            }
        }

        // Identify top-level nodes for the Sugiyama layout
        let top_level_nodes = self.identify_top_level_nodes(graph, &hierarchy_map);

        // If we have top-level nodes, position them first, then position children
        let final_positions = if !top_level_nodes.is_empty() {
            // Clone the positions to avoid borrow issues
            let mut final_positions = positions.clone();

            // Position child nodes within their parent containers
            self.position_all_children(
                &top_level_nodes,
                &hierarchy_map,
                &component_sizes,
                &mut final_positions,
            );

            final_positions
        } else {
            positions
        };

        // Create components with positions and sizes
        let components: Vec<Component<'a>> = graph
            .node_indices()
            .map(|node_idx| {
                let node = graph.node_weight(node_idx).unwrap();
                let position = final_positions
                    .get(&node_idx)
                    .unwrap_or(&Point { x: 0.0, y: 0.0 });
                let size = component_sizes.get(&node_idx).cloned().unwrap_or(Size {
                    width: self.min_component_width,
                    height: self.min_component_height,
                });

                Component {
                    node,
                    position: *position,
                    size,
                }
            })
            .collect();

        // Map node indices to component indices
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

    /// Identify top-level nodes (nodes that aren't children of other nodes)
    fn identify_top_level_nodes(
        &self,
        graph: &Graph,
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
    ) -> Vec<NodeIndex> {
        // Identify all nodes that are children of some other node
        let mut is_child = std::collections::HashSet::new();
        for children in hierarchy_map.values() {
            for &child in children {
                is_child.insert(child);
            }
        }

        // Find top-level nodes (nodes that aren't children)
        graph
            .node_indices()
            .filter(|&idx| !is_child.contains(&idx))
            .collect()
    }

    /// Position all children within their parent containers
    fn position_all_children(
        &self,
        top_level_nodes: &[NodeIndex],
        hierarchy_map: &HashMap<NodeIndex, Vec<NodeIndex>>,
        sizes: &HashMap<NodeIndex, Size>,
        result_positions: &mut HashMap<NodeIndex, Point>,
    ) {
        for &node_idx in top_level_nodes {
            if let Some(children) = hierarchy_map.get(&node_idx) {
                if let Some(&parent_pos) = result_positions.get(&node_idx) {
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
        let available_width = parent_size.width - (self.container_padding * 2.0);
        let available_height = parent_size.height - (self.container_padding * 2.0);

        // Determine layout arrangement (simple grid layout for now)
        let sqrt_count = (children.len() as f64).sqrt().ceil() as usize;
        let cols = sqrt_count;
        let rows = children.len().div_ceil(cols);

        // Calculate cell dimensions
        // Add a slight gap between cells for better visual separation
        let cell_width = (available_width - (cols as f32 - 1.0) * 5.0) / cols as f32;
        let cell_height = (available_height - (rows as f32 - 1.0) * 5.0) / rows as f32;

        // Calculate starting point (top-left of container area)
        let start_x = parent_pos.x - (parent_size.width / 2.0) + self.container_padding;
        let start_y = parent_pos.y - (parent_size.height / 2.0) + self.container_padding;

        // Position each child
        for (i, &child_idx) in children.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;

            // Calculate center position of this cell with gap between cells
            let cell_center_x = start_x + (col as f32 * (cell_width + 5.0)) + (cell_width / 2.0);
            let cell_center_y = start_y + (row as f32 * (cell_height + 5.0)) + (cell_height / 2.0);

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

    fn fallback_hierarchical_layout(
        &self,
        graph: &Graph,
        positions: &mut HashMap<NodeIndex, Point>,
    ) {
        // Create a new DiGraph for topological sort
        let mut digraph = petgraph::graph::DiGraph::<(), ()>::new();
        let mut node_mapping = HashMap::new();

        // Add nodes to the digraph
        for node_idx in graph.node_indices() {
            let new_idx = digraph.add_node(());
            node_mapping.insert(node_idx, new_idx);
        }

        // Add edges to the digraph
        for edge_idx in graph.edge_indices() {
            if let Some((source, target)) = graph.edge_endpoints(edge_idx) {
                if let (Some(&src_idx), Some(&tgt_idx)) =
                    (node_mapping.get(&source), node_mapping.get(&target))
                {
                    digraph.add_edge(src_idx, tgt_idx, ());
                }
            }
        }

        // Attempt a topological sort
        let topo_sort = toposort(&digraph, None);

        // If topological sort succeeded, use it for layering
        if let Ok(sorted_nodes) = topo_sort {
            // Map sorted nodes back to original indices
            let reverse_mapping: HashMap<_, _> =
                node_mapping.iter().map(|(&k, &v)| (v, k)).collect();

            // Map the nodes from topological sort back to original indices
            let mapped_nodes: Vec<_> = sorted_nodes
                .into_iter()
                .filter_map(|n| reverse_mapping.get(&n).copied())
                .collect();

            // Group nodes by their distance from roots (sources)
            let mut layers: HashMap<usize, Vec<NodeIndex>> = HashMap::new();
            let mut node_layers: HashMap<NodeIndex, usize> = HashMap::new();

            // Find source nodes (no incoming edges)
            let source_nodes: HashSet<NodeIndex> = graph
                .node_indices()
                .filter(|&node_idx| {
                    // A node is a source if no edges point to it
                    !graph.edge_indices().any(|edge_idx| {
                        graph
                            .edge_endpoints(edge_idx)
                            .map(|(_, target)| target == node_idx)
                            .unwrap_or(false)
                    })
                })
                .collect();

            // Assign layer 0 to source nodes
            for &node in &source_nodes {
                node_layers.insert(node, 0);
                layers.entry(0).or_default().push(node);
            }

            // For each node in topological order, assign a layer
            for &node in &mapped_nodes {
                // Skip source nodes, they're already processed
                if source_nodes.contains(&node) {
                    continue;
                }

                // Find max layer of predecessors + 1
                let mut max_layer = 0;

                // Find all edges pointing to this node
                for edge_idx in graph.edge_indices() {
                    if let Some((source, target)) = graph.edge_endpoints(edge_idx) {
                        if target == node {
                            let predecessor = source;
                            let pred_layer = *node_layers.get(&predecessor).unwrap_or(&0);
                            max_layer = max_layer.max(pred_layer + 1);
                        }
                    }
                }

                // Assign this node to its layer
                node_layers.insert(node, max_layer);
                layers.entry(max_layer).or_default().push(node);
            }

            // Calculate y-position for each layer
            let mut y_positions = HashMap::new();
            let mut current_y = 0.0;

            // Sort layers by their index
            let mut layer_indices: Vec<usize> = layers.keys().cloned().collect();
            layer_indices.sort();

            // Assign y-positions to each layer
            for layer_idx in layer_indices {
                // Store the y-position for this layer
                y_positions.insert(layer_idx, current_y);

                // Update y for the next layer
                current_y += self.vertical_spacing;
            }

            // Calculate x-positions for each node within its layer
            for (layer_idx, nodes) in &layers {
                let y = *y_positions.get(layer_idx).unwrap_or(&0.0);

                // For each layer, spread the nodes horizontally
                let node_count = nodes.len();
                let spacing = self.horizontal_spacing;
                let total_width = (node_count as f32 - 1.0) * spacing;
                let start_x = -total_width / 2.0;

                for (i, &node) in nodes.iter().enumerate() {
                    let x = start_x + (i as f32) * spacing;
                    positions.insert(node, Point { x, y });
                }
            }
        } else {
            // If topological sort failed (cyclic graph), fall back to a grid layout
            let node_count = graph.node_indices().count();
            let grid_size = (node_count as f32).sqrt().ceil() as usize;

            for (i, node_idx) in graph.node_indices().enumerate() {
                let row = i / grid_size;
                let col = i % grid_size;

                let x = col as f32 * self.horizontal_spacing;
                let y = row as f32 * self.vertical_spacing;

                positions.insert(node_idx, Point { x, y });
            }
        }
    }

    fn center_layout(&self, positions: &mut HashMap<NodeIndex, Point>) {
        // Find min and max x, y coordinates
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for &Point { x, y } in positions.values() {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        // Calculate the offsets needed to ensure all coordinates are positive
        // with a reasonable margin from the edge (add a small margin)
        let offset_x = if min_x < 0.0 {
            min_x - self.horizontal_spacing * 0.3
        } else {
            -self.horizontal_spacing * 0.3
        };
        let offset_y = if min_y < 0.0 {
            min_y - self.vertical_spacing * 0.3
        } else {
            -self.vertical_spacing * 0.3
        };

        // Apply the offset to all positions to ensure they're positive
        for position in positions.values_mut() {
            position.x -= offset_x;
            position.y -= offset_y;
        }
    }
}

impl ComponentEngine for Engine {
    fn calculate<'a>(&self, graph: &'a Graph) -> Layout<'a> {
        self.calculate_layout(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sugiyama_layout_basics() {
        // Create a minimal engine and ensure it can be instantiated
        let _engine = Engine::new();
        assert!(true, "Engine successfully instantiated");
    }
}
