//! Basic component layout engine
//!
//! This module provides a layout engine for component diagrams
//! using a simple, deterministic algorithm.

use crate::{
    ast,
    graph::{ContainmentScope, Graph},
    layout::{
        component::{Layout, LayoutRelation, adjust_positioned_contents_offset},
        engines::{ComponentEngine, EmbeddedLayouts},
        geometry::{Component, Point, Size},
        layer::{ContentStack, PositionedContent},
        positioning::calculate_bounded_text_size,
    },
    shape::{self, Shape},
};
use petgraph::{
    Direction,
    graph::{DiGraph, EdgeIndex, NodeIndex},
};
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

/// Basic component layout engine implementation that implements the ComponentLayoutEngine trait
#[derive(Default)]
pub struct Engine {
    padding: f32,
    text_padding: f32,
    min_spacing: f32,
}

impl Engine {
    /// Create a new basic component layout engine
    pub fn new() -> Self {
        Self {
            text_padding: 20.0,
            ..Self::default()
        }
    }

    /// Set the padding around components
    pub fn set_padding(&mut self, padding: f32) -> &mut Self {
        self.padding = padding;
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
    pub fn calculate_layout<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        let mut content_stack = ContentStack::<Layout<'a>>::new();
        let mut positioned_content_sizes = HashMap::<NodeIndex, Size>::new();

        for containment_scope in graph.containment_scopes() {
            // Calculate component shapes - they contain all sizing information
            let mut component_shapes = self.calculate_component_shapes(
                graph,
                containment_scope,
                &positioned_content_sizes,
                embedded_layouts,
            );

            // Extract sizes from shapes for position calculation
            let component_sizes: HashMap<NodeIndex, Size> = component_shapes
                .iter()
                .map(|(idx, shape)| (*idx, shape.shape_size()))
                .collect();

            // Calculate positions for components
            let positions = self.positions(graph, containment_scope, &component_sizes);

            // Build the final component list using the pre-configured shapes
            let components: Vec<Component<'a>> = graph
                .containment_scope_nodes_with_indices(containment_scope)
                .map(|(node_idx, node)| {
                    let position = *positions.get(&node_idx).unwrap();
                    let shape = component_shapes.remove(&node_idx).unwrap();

                    Component {
                        node,
                        shape,
                        position,
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
                .containment_scope_relations(containment_scope)
                .filter_map(|relation| {
                    // Only include relations between visible components
                    // (not including relations within inner blocks)
                    if let (Some(&source_index), Some(&target_index)) = (
                        component_indices.get(&relation.source),
                        component_indices.get(&relation.target),
                    ) {
                        Some(LayoutRelation::new(relation, source_index, target_index))
                    } else {
                        None
                    }
                })
                .collect();

            let positioned_content = PositionedContent::new(Layout {
                components,
                relations,
            });

            if let Some(container) = containment_scope.container() {
                // If this layer is a container, we need to adjust its size based on its contents
                let size = positioned_content.layout_size();
                positioned_content_sizes.insert(container, size);
            }
            content_stack.push(positioned_content);
        }

        adjust_positioned_contents_offset(&mut content_stack, graph);

        content_stack
    }

    /// Calculate component shapes with proper content size and padding
    fn calculate_component_shapes<'a>(
        &self,
        graph: &Graph<'a>,
        containment_scope: &ContainmentScope,
        positioned_content_sizes: &HashMap<NodeIndex, Size>,
        embedded_layouts: &EmbeddedLayouts<'_>,
    ) -> HashMap<NodeIndex, Shape> {
        let mut component_shapes: HashMap<NodeIndex, Shape> = HashMap::new();

        for (node_idx, node) in graph.containment_scope_nodes_with_indices(containment_scope) {
            let mut shape = Shape::new(Rc::clone(&node.type_definition.shape_definition));

            let content_size = match node.block {
                ast::Block::Diagram(_) => {
                    // Since we process in post-order (innermost to outermost),
                    // embedded diagram layouts should already be calculated and available
                    let layout = embedded_layouts
                        .get(&node.id)
                        .expect("Embedded layout not found");

                    let embedded_size = layout.calculate_size();
                    let text_size = calculate_bounded_text_size(node, self.text_padding);

                    Size::new(
                        text_size.width().max(embedded_size.width()),
                        text_size.height() + embedded_size.height(),
                    )
                }
                ast::Block::Scope(_) => {
                    let positioned_content_size = *positioned_content_sizes
                        .get(&node_idx)
                        .expect("Scope size not found");

                    let text_size = calculate_bounded_text_size(node, self.text_padding);

                    Size::new(
                        text_size.width().max(positioned_content_size.width()),
                        text_size.height() + positioned_content_size.height(),
                    )
                }
                ast::Block::None => calculate_bounded_text_size(node, self.text_padding),
            };

            shape.expand_content_size_to(content_size);
            shape.set_padding(self.text_padding);
            component_shapes.insert(node_idx, shape);
        }

        component_shapes
    }

    /// Calculate positions for components in a containment scope
    fn positions<'a>(
        &self,
        graph: &Graph<'a>,
        containment_scope: &ContainmentScope,
        sizes: &HashMap<NodeIndex, Size>,
    ) -> HashMap<NodeIndex, Point> {
        // Step 1: Create a simplified graph
        let containment_scope_graph = Self::containment_scope_to_graph(graph, containment_scope);

        // Step 2: Assign layers for the top-level nodes
        let layers = Self::assign_layers_for_containment_scope_graph(&containment_scope_graph);

        // Step 3: Calculate layer metrics (widths and spacings)
        let (layer_widths, layer_spacings) =
            self.calculate_layer_metrics(graph, containment_scope, &layers, sizes);

        // Step 4: Calculate X positions for each layer
        let layer_x_positions = self.calculate_layer_x_positions(&layer_widths, &layer_spacings);

        // Step 5: Position top-level nodes within their layers
        self.position_nodes_in_layers(&layers, &layer_x_positions, sizes)
    }

    /// Calculate metrics for each layer: widths and spacings between layers
    fn calculate_layer_metrics(
        &self,
        graph: &Graph,
        containment_scope: &ContainmentScope,
        layers: &[Vec<NodeIndex>],
        sizes: &HashMap<NodeIndex, Size>,
    ) -> (Vec<f32>, Vec<f32>) {
        // Calculate max width for each layer
        let layer_widths: Vec<f32> = layers
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|&node_idx| sizes.get(&node_idx).unwrap().width())
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
                    .unwrap_or_default()
            })
            .collect();

        // Initialize spacings with default padding
        let mut layer_spacings = vec![self.padding; layers.len().saturating_sub(1)];

        // HACK: fix it.
        let mut text_def = shape::TextDefinition::new();
        text_def.set_font_size(14);
        let text_def = Rc::new(RefCell::new(text_def));

        // Adjust spacings based on relation labels
        for relation in graph.containment_scope_relations(containment_scope) {
            if let Some(label) = &relation.label {
                let text = shape::Text::new(Rc::clone(&text_def), label.clone());
                let label_width = text.calculate_size().width();

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
    // PERF: Depricate this method in favor of a more efficient approach.
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
                let node = graph.node_from_idx(*node_idx);
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
                let node_height = sizes.get(&node_idx).unwrap().height();

                if j > 0 {
                    y_pos += self.padding; // Space between components
                }

                let y = y_pos + node_height / 2.0;
                positions.insert(node_idx, Point::new(x, y));

                y_pos += node_height;
            }
        }

        positions
    }

    /// Helper method to assign layers for a specific graph
    fn assign_layers_for_containment_scope_graph(
        containment_scope_graph: &DiGraph<NodeIndex, EdgeIndex>,
    ) -> Vec<Vec<NodeIndex>> {
        let mut layers = Vec::new();
        let mut visited = HashSet::new();

        // Find root nodes
        let root_nodes: Vec<_> = containment_scope_graph
            .node_indices()
            .filter(|&idx| {
                containment_scope_graph
                    .neighbors_directed(idx, Direction::Incoming)
                    .count()
                    == 0
            })
            .map(|idx| (idx, containment_scope_graph.node_weight(idx).unwrap()))
            .collect();

        let start_nodes = if root_nodes.is_empty() {
            containment_scope_graph
                .node_indices()
                .take(1)
                .map(|idx| (idx, containment_scope_graph.node_weight(idx).unwrap()))
                .collect()
        } else {
            root_nodes
        };

        // Perform BFS to assign layers
        let mut queue = VecDeque::new();
        for node in start_nodes {
            queue.push_back((node, 0));
        }

        while let Some(((layer_idx, &original_idx), layer)) = queue.pop_front() {
            if visited.contains(&layer_idx) {
                continue;
            }
            visited.insert(layer_idx);
            while layers.len() <= layer {
                layers.push(Vec::new());
            }

            layers[layer].push(original_idx);

            for child in containment_scope_graph.neighbors(layer_idx) {
                if !visited.contains(&child) {
                    let child_original_idx = containment_scope_graph.node_weight(child).unwrap();
                    queue.push_back(((child, child_original_idx), layer + 1));
                }
            }
        }

        layers
    }

    fn containment_scope_to_graph(
        graph: &Graph,
        containment_scope: &ContainmentScope,
    ) -> DiGraph<NodeIndex, EdgeIndex> {
        let mut layer_graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add nodes from the layer to the filtered graph
        for node_idx in containment_scope.node_indices() {
            let new_idx = layer_graph.add_node(node_idx);
            node_map.insert(node_idx, new_idx);
        }

        // Add edges between nodes in the layer
        for (edge_idx, source, target) in
            graph.containment_scope_relation_endpoint_indices(containment_scope)
        {
            if let (Some(&src_idx), Some(&tgt_idx)) = (node_map.get(&source), node_map.get(&target))
            {
                layer_graph.add_edge(src_idx, tgt_idx, edge_idx);
            }
        }

        layer_graph
    }
}

impl ComponentEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
