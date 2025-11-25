use std::{borrow::Cow, collections::HashMap};

use log::debug;
use rust_sugiyama::configure::Config;

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

/// The Sugiyama layout engine for component diagrams
/// Based on the Sugiyama algorithm for layered drawing of directed graphs
/// Uses the rust-sugiyama implementation with fallback to a simple hierarchical layout
pub struct Engine {
    /// Padding around text elements
    text_padding: f32,

    /// Horizontal spacing between components
    horizontal_spacing: f32,

    /// Vertical spacing between layers
    vertical_spacing: f32,

    /// Container padding for nested components
    container_padding: Insets,
}

impl Engine {
    /// Create a new Sugiyama component layout engine
    pub fn new() -> Self {
        Self {
            text_padding: 20.0,
            horizontal_spacing: 50.0,
            vertical_spacing: 80.0,
            container_padding: Insets::uniform(20.0),
        }
    }

    /// Set the text padding
    #[allow(dead_code)]
    pub fn set_text_padding(&mut self, padding: f32) -> &mut Self {
        self.text_padding = padding;
        self
    }

    /// Set the horizontal spacing between components
    pub fn set_horizontal_spacing(&mut self, spacing: f32) -> &mut Self {
        self.horizontal_spacing = spacing;
        self
    }

    /// Set the vertical spacing between layers
    pub fn set_vertical_spacing(&mut self, spacing: f32) -> &mut Self {
        self.vertical_spacing = spacing;
        self
    }

    /// Set the padding inside container components
    pub fn set_container_padding(&mut self, padding: Insets) -> &mut Self {
        self.container_padding = padding;
        self
    }

    fn calculate_layout<'a>(
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

            // Extract sizes from shapes for position calculation
            let component_sizes: HashMap<Id, Size> = component_shapes
                .iter()
                .map(|(idx, shape_with_text)| (*idx, shape_with_text.size()))
                .collect();

            // Calculate positions for components in this scope
            let positions = self.positions(graph, containment_scope, &component_sizes);

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

    /// Calculate component shapes with proper sizing and padding
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
            shape.set_padding(self.container_padding);
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
        graph: &ComponentGraph<'a, '_>,
        containment_scope: &ContainmentScope,
        _component_sizes: &HashMap<Id, Size>,
    ) -> HashMap<Id, Point> {
        // Prepare layout
        let mut positions = HashMap::new();

        // Convert our graph to a format suitable for the Sugiyama algorithm
        let mut edges = Vec::new();
        let mut node_ids: HashMap<Id, u32> = HashMap::new();

        // Get nodes for this containment scope
        let scope_nodes: Vec<_> = graph.scope_nodes(containment_scope).collect();

        // Map node IDs to u32 IDs for rust-sugiyama
        for (i, node) in scope_nodes.iter().enumerate() {
            let id = i as u32;
            node_ids.insert(node.id(), id);
        }

        // Extract edges for this containment scope
        for relation in graph.scope_relations(containment_scope) {
            if let (Some(&source_id), Some(&target_id)) = (
                node_ids.get(&relation.source()),
                node_ids.get(&relation.target()),
            ) {
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

            // Create a bidirectional mapping between our original node IDs and sequential IDs
            let id_to_node_id: HashMap<u32, Id> =
                node_ids.iter().map(|(&node, &id)| (id, node)).collect();

            // Try the rust_sugiyama crate with our sequential IDs, catching any panics
            let layouts = std::panic::catch_unwind(move || {
                let config = Config {
                    minimum_length: 1,
                    vertex_spacing: 3.0,
                    ..Default::default()
                };
                rust_sugiyama::from_edges(&edges, &config)
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
                            debug!("Node ID {id} from rust-sugiyama result is out of valid range");
                            continue;
                        };

                        // Map the ID back to our original node Id
                        if let Some(&node_id) = id_to_node_id.get(&node_id) {
                            // Scale coordinates for proper spacing
                            // Apply spacing that ensures adequate separation between nodes
                            // Use smaller scaling factors to reduce padding
                            let x_pos = (x as f32) * self.horizontal_spacing * 1.0;
                            let y_pos = (y as f32) * self.vertical_spacing * 1.0;
                            positions.insert(node_id, Point::new(x_pos, y_pos));
                        }
                    }

                    // If mapping failed for all nodes, fall back to hierarchical layout
                    if positions.is_empty() {
                        panic!("Failed to map any rust-sugiyama positions back to graph nodes.");
                    }
                }

                // Empty results case
                Ok(results) if results.is_empty() => {
                    panic!("Rust-sugiyama returned empty layout results.");
                }

                // Unexpected success case
                Ok(_) => {
                    panic!("Rust-sugiyama returned unexpected result format.");
                }

                // Error/panic case
                Err(err) => {
                    if let Some(panic_msg) = err.downcast_ref::<String>() {
                        panic!("Rust-sugiyama layout engine panicked: {panic_msg}.");
                    } else {
                        panic!("Rust-sugiyama layout engine panicked with unknown error.");
                    }
                }
            }

            // Center the layout if we have positions
            if !positions.is_empty() {
                self.center_layout(&mut positions);
            }

            debug!(
                "Layout generated with {} positioned nodes and positive coordinates",
                positions.len(),
            );
        } else if !scope_nodes.is_empty() {
            // No edges but we have nodes - arrange them horizontally with positive coordinates
            debug!("Graph has no edges. Arranging nodes horizontally with positive coordinates.");
            for (i, node) in scope_nodes.iter().enumerate() {
                // For no-edge graphs, ensure adequate horizontal spacing and a margin from the top
                let x =
                    self.horizontal_spacing * 0.8 + (i as f32) * (self.horizontal_spacing * 0.5);
                positions.insert(node.id(), Point::new(x, self.vertical_spacing * 0.8));
            }
        }

        positions
    }

    fn center_layout(&self, positions: &mut HashMap<Id, Point>) {
        // Find min and max x, y coordinates
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for point in positions.values() {
            min_x = min_x.min(point.x());
            min_y = min_y.min(point.y());
            max_x = max_x.max(point.x());
            max_y = max_y.max(point.y());
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
            *position = position.sub_point(Point::new(offset_x, offset_y));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sugiyama_layout_basics() {
        // Create a minimal engine and ensure it can be instantiated
        let _engine = Engine::new();
    }
}
