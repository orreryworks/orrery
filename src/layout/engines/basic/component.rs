//! Basic component layout engine
//!
//! This module provides a layout engine for component diagrams
//! using a simple, deterministic algorithm.

use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{HashMap, HashSet, VecDeque},
};

use crate::{
    ast,
    draw::{self, Drawable},
    geometry::{Insets, Point, Size},
    identifier::Id,
    layout::{
        component::{Component, Layout, LayoutRelation, adjust_positioned_contents_offset},
        engines::{ComponentEngine, EmbeddedLayouts},
        layer::{ContentStack, PositionedContent},
    },
    structure::{ComponentGraph, ContainmentScope},
};

/// Basic component layout engine implementation that implements the ComponentLayoutEngine trait
#[derive(Default)]
pub struct Engine {
    padding: Insets,
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
    pub fn set_padding(&mut self, padding: Insets) -> &mut Self {
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
        graph: &'a ComponentGraph<'a, '_>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        let mut content_stack = ContentStack::<Layout<'a>>::new();
        let mut positioned_content_sizes = HashMap::<Id, Size>::new();

        for containment_scope in graph.containment_scopes() {
            // Calculate component shapes - they contain all sizing information
            let mut component_shapes = self.calculate_component_shapes(
                graph,
                containment_scope,
                &positioned_content_sizes,
                embedded_layouts,
            );

            // Calculate positions for components
            let positions = self.positions(graph, containment_scope, &component_shapes);

            // Build the final component list using the pre-configured shapes
            let components: Vec<Component> = graph
                .scope_nodes(containment_scope)
                .map(|node| {
                    let position = *positions.get(&node.id()).unwrap();
                    let shape_with_text = component_shapes.remove(&node.id()).unwrap();

                    Component::new(node, shape_with_text, position)
                })
                .collect();

            // Map node IDs to their component indices
            let component_indices: HashMap<_, _> = components
                .iter()
                .enumerate()
                .map(|(idx, component)| (component.node_id(), idx))
                .collect();

            // Build the list of relations between components
            let relations: Vec<LayoutRelation> = graph
                .scope_relations(containment_scope)
                .filter_map(|relation| {
                    // Only include relations between visible components
                    // (not including relations within inner blocks)
                    if let (Some(&source_index), Some(&target_index)) = (
                        component_indices.get(&relation.source()),
                        component_indices.get(&relation.target()),
                    ) {
                        Some(LayoutRelation::from_ast(
                            relation,
                            source_index,
                            target_index,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            let positioned_content = PositionedContent::new(Layout::new(components, relations));

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
        graph: &'a ComponentGraph<'a, '_>,
        containment_scope: &ContainmentScope,
        positioned_content_sizes: &HashMap<Id, Size>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> HashMap<Id, draw::ShapeWithText<'a>> {
        let mut component_shapes: HashMap<Id, draw::ShapeWithText<'a>> = HashMap::new();

        // TODO: move it to the best place.
        for node in graph.scope_nodes(containment_scope) {
            let mut shape = draw::Shape::new(
                node.type_definition()
                    .shape_definition()
                    .expect("Node must have a shape definition for component layout")
                    .clone_box(),
            );
            shape.set_padding(self.padding);
            let text = draw::Text::new(
                Cow::Borrowed(
                    node.type_definition()
                        .shape_definition()
                        .expect("Node type must be a shape")
                        .text(),
                ),
                node.display_text().to_string(),
            );
            let mut shape_with_text = draw::ShapeWithText::new(shape, Some(text));

            match node.block() {
                ast::Block::Diagram(_) => {
                    // Since we process in post-order (innermost to outermost),
                    // embedded diagram layouts should already be calculated and available
                    let layout = embedded_layouts
                        .get(&node.id())
                        .expect("Embedded layout not found");

                    let content_size = layout.calculate_size();
                    shape_with_text
                        .set_inner_content_size(content_size)
                        .expect("Diagram blocks should always support content sizing");
                }
                ast::Block::Scope(_) => {
                    let content_size = *positioned_content_sizes
                        .get(&node.id())
                        .expect("Scope size not found");
                    shape_with_text
                        .set_inner_content_size(content_size)
                        .expect("Scope blocks should always support content sizing");
                }
                ast::Block::None => {
                    // No content to size, so don't call set_inner_content_size
                }
            };
            component_shapes.insert(node.id(), shape_with_text);
        }

        component_shapes
    }

    /// Calculate positions for components in a containment scope
    fn positions<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        containment_scope: &ContainmentScope,
        component_shapes: &HashMap<Id, draw::ShapeWithText<'a>>,
    ) -> HashMap<Id, Point> {
        // Step 1: Assign layers for the top-level nodes
        let layers = Self::assign_layers_for_containment_scope_graph(graph, containment_scope);
        // Step 2: Calculate layer metrics (widths and spacings)
        let (layer_widths, layer_spacings) =
            self.calculate_layer_metrics(graph, containment_scope, &layers, component_shapes);
        // Step 3: Calculate X positions for each layer
        let layer_x_positions = self.calculate_layer_x_positions(&layer_widths, &layer_spacings);
        // Step 4: Position nodes within their layers
        self.position_nodes_in_layers(&layers, &layer_x_positions, component_shapes)
    }

    /// Calculate metrics for each layer: widths and spacings between layers
    fn calculate_layer_metrics<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        containment_scope: &ContainmentScope,
        layers: &[Vec<Id>],
        component_shapes: &HashMap<Id, draw::ShapeWithText<'a>>,
    ) -> (Vec<f32>, Vec<f32>) {
        // Calculate max width for each layer
        let layer_widths: Vec<f32> = layers
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|&node_idx| component_shapes.get(&node_idx).unwrap().size().width())
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less))
                    .unwrap_or_default()
            })
            .collect();

        // Initialize spacings with default padding
        let mut layer_spacings =
            vec![self.padding.horizontal_sum() / 2.0; layers.len().saturating_sub(1)];

        // Adjust spacings based on relation labels
        for relation in graph.scope_relations(containment_scope) {
            if let Some(text) = relation.text() {
                let label_width = text.calculate_size().width();

                // Find layers for source and target nodes
                let (source_layer, target_layer) = self.find_node_layers(graph, relation, layers);

                if let (Some(src), Some(tgt)) = (source_layer, target_layer)
                    && src != tgt
                {
                    // Only adjust spacing for relations between different layers
                    let min_layer = src.min(tgt);
                    let needed_spacing = label_width + 30.0; // Add some padding

                    // Update spacing if label requires more space
                    if min_layer < layer_spacings.len() {
                        layer_spacings[min_layer] = layer_spacings[min_layer].max(needed_spacing);
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
        graph: &ComponentGraph,
        relation: &ast::Relation,
        layers: &[Vec<Id>],
    ) -> (Option<usize>, Option<usize>) {
        let mut source_layer = None;
        let mut target_layer = None;

        for (layer_idx, layer_nodes) in layers.iter().enumerate() {
            for node_id in layer_nodes {
                let node = graph.node_by_id(*node_id).expect("Node not found");
                if node.id() == relation.source() {
                    source_layer = Some(layer_idx);
                }
                if node.id() == relation.target() {
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
                self.padding.horizontal_sum() / 2.0
            };
            x_pos += width + spacing;
        }

        layer_x_positions
    }

    /// Position nodes within their layers
    fn position_nodes_in_layers<'a>(
        &self,
        layers: &[Vec<Id>],
        layer_x_positions: &[f32],
        component_shapes: &HashMap<Id, draw::ShapeWithText<'a>>,
    ) -> HashMap<Id, Point> {
        let mut positions = HashMap::new();

        for (layer_idx, layer_nodes) in layers.iter().enumerate() {
            let x = layer_x_positions[layer_idx];

            // Calculate heights for vertical positioning
            let mut y_pos = 0.0;
            for (j, &node_idx) in layer_nodes.iter().enumerate() {
                let node_height = component_shapes.get(&node_idx).unwrap().size().height();

                if j > 0 {
                    y_pos += self.padding.vertical_sum() / 2.0; // Space between components
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
        graph: &ComponentGraph,
        containment_scope: &ContainmentScope,
    ) -> Vec<Vec<Id>> {
        let mut layers = Vec::new();
        let mut visited = HashSet::new();

        // Find root nodes
        let root_nodes: Vec<_> = graph.scope_roots(containment_scope).collect();

        let start_nodes = if root_nodes.is_empty() {
            graph.scope_nodes(containment_scope).take(1).collect()
        } else {
            root_nodes
        };

        // Perform BFS to assign layers
        let mut queue = VecDeque::new();
        for node in start_nodes {
            queue.push_back((node, 0));
        }

        while let Some((node, layer)) = queue.pop_front() {
            if !visited.insert(node.id()) {
                continue;
            }
            while layers.len() <= layer {
                layers.push(Vec::new());
            }

            layers[layer].push(node.id());

            for neighbor in graph
                .scope_outgoing_neighbors(containment_scope, node.id())
                .filter(|node| !visited.contains(&node.id()))
            {
                queue.push_back((neighbor, layer + 1));
            }
        }

        // Handle disconnected components by processing any remaining unvisited nodes
        while visited.len() < containment_scope.nodes_count() {
            // Find an unvisited node to start a new component
            // PERF: This can be an outer loop for a single iteration.
            let unvisited_node_id = containment_scope
                .node_ids()
                .find(|id| !visited.contains(id))
                .expect("Should have unvisited nodes");

            let unvisited_node = graph.node_by_id(unvisited_node_id).expect("Node not found");

            queue.push_back((unvisited_node, 0));

            // Process this disconnected component using the same BFS logic
            // TODO: this is a duplicated code.
            while let Some((node, layer)) = queue.pop_front() {
                if !visited.insert(node.id()) {
                    continue;
                }
                while layers.len() <= layer {
                    layers.push(Vec::new());
                }

                layers[layer].push(node.id());

                for neighbor in graph
                    .scope_outgoing_neighbors(containment_scope, node.id())
                    .filter(|node| !visited.contains(&node.id()))
                {
                    queue.push_back((neighbor, layer + 1));
                }
            }
        }

        layers
    }
}

impl ComponentEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a ComponentGraph<'a, '_>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<Layout<'a>> {
        self.calculate_layout(graph, embedded_layouts)
    }
}
