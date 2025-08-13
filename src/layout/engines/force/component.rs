//! Force-directed component layout engine
//!
//! This module implements a force-directed graph layout algorithm
//! for component diagrams.

use crate::{
    ast,
    draw::{self, Drawable},
    geometry::{Insets, Point, Size},
    graph::{ContainmentScope, Graph},
    layout::{
        component::{Component, Layout, LayoutRelation, adjust_positioned_contents_offset},
        engines::{ComponentEngine, EmbeddedLayouts},
        layer::{ContentStack, PositionedContent},
    },
};
use log::debug;
use petgraph::graph::NodeIndex;
use std::{collections::HashMap, rc::Rc};

/// Force layout engine for component diagrams
///
/// This engine implements a simple force-directed layout algorithm
/// for component diagrams. It uses a physics simulation to position
/// components based on a system of attractive and repulsive forces.
pub struct Engine {
    // Simulation parameters
    iterations: usize,
    spring_constant: f32,
    repulsion_constant: f32,
    damping_factor: f32,
    // Used for maintaining distance between components
    text_padding: f32,
    min_distance: f32,
    // Component padding
    padding: Insets,
}

impl Engine {
    /// Create a new force component layout engine
    pub fn new() -> Self {
        Self {
            iterations: 100,
            spring_constant: 0.1,
            repulsion_constant: 1000.0,
            damping_factor: 0.85,
            text_padding: 10.0,
            min_distance: 80.0,
            padding: Insets::uniform(10.0),
        }
    }

    /// Set the number of iterations for the force simulation
    pub fn set_iterations(&mut self, iterations: usize) -> &mut Self {
        self.iterations = iterations;
        self
    }

    /// Set the spring constant for edge forces
    #[allow(dead_code)]
    pub fn set_spring_constant(&mut self, constant: f32) -> &mut Self {
        self.spring_constant = constant;
        self
    }

    /// Set the repulsion constant for node forces
    #[allow(dead_code)]
    pub fn set_repulsion_constant(&mut self, constant: f32) -> &mut Self {
        self.repulsion_constant = constant;
        self
    }

    /// Set the damping factor for the simulation
    #[allow(dead_code)]
    pub fn set_damping_factor(&mut self, factor: f32) -> &mut Self {
        self.damping_factor = factor;
        self
    }

    /// Set the text padding
    #[allow(dead_code)]
    pub fn set_text_padding(&mut self, padding: f32) -> &mut Self {
        // TODO: Do I need this padding?
        self.text_padding = padding;
        self
    }

    /// Set the minimum distance between components
    pub fn set_min_distance(&mut self, distance: f32) -> &mut Self {
        self.min_distance = distance;
        self
    }

    /// Set the padding around components
    pub fn set_padding(&mut self, padding: Insets) -> &mut Self {
        self.padding = padding;
        self
    }

    /// Calculate component shapes with proper sizing and padding
    fn calculate_component_shapes<'a>(
        &self,
        graph: &Graph<'a>,
        containment_scope: &ContainmentScope,
        positioned_content_sizes: &HashMap<NodeIndex, Size>,
        embedded_layouts: &EmbeddedLayouts,
    ) -> HashMap<NodeIndex, draw::ShapeWithText> {
        let mut component_shapes: HashMap<NodeIndex, draw::ShapeWithText> = HashMap::new();

        // TODO: move it to the best place.
        for (node_idx, node) in graph.containment_scope_nodes_with_indices(containment_scope) {
            let mut shape = draw::Shape::new(Rc::clone(
                node.type_definition
                    .shape_definition()
                    .expect("Node must have a shape definition for component layout"),
            ));
            shape.set_padding(self.padding);
            let text = draw::Text::new(
                Rc::clone(&node.type_definition.text_definition),
                node.display_text().to_string(),
            );
            let mut shape_with_text = draw::ShapeWithText::new(shape, Some(text));

            match node.block {
                ast::Block::Diagram(_) => {
                    // Since we process in post-order (innermost to outermost),
                    // embedded diagram layouts should already be calculated and available
                    let layout = embedded_layouts
                        .get(&node.id)
                        .expect("Embedded layout not found");

                    let content_size = layout.calculate_size();
                    shape_with_text
                        .set_inner_content_size(content_size)
                        .expect("Diagram blocks should always support content sizing");
                }
                ast::Block::Scope(_) => {
                    let content_size = *positioned_content_sizes
                        .get(&node_idx)
                        .expect("Scope size not found");
                    shape_with_text
                        .set_inner_content_size(content_size)
                        .expect("Scope blocks should always support content sizing");
                }
                ast::Block::None => {
                    // No content to size, so don't call set_inner_content_size
                }
            };
            component_shapes.insert(node_idx, shape_with_text);
        }

        component_shapes
    }

    /// Initialize random positions for components
    fn initialize_positions<'a>(
        &self,
        graph: &Graph<'a>,
        containment_scope: &ContainmentScope,
        _component_sizes: &HashMap<NodeIndex, Size>,
    ) -> HashMap<NodeIndex, Point> {
        use rand::Rng;
        let mut rng = rand::rng();

        // Calculate approximate grid dimensions
        let node_count = graph
            .containment_scope_nodes_with_indices(containment_scope)
            .count();
        let grid_size = (node_count as f32).sqrt().ceil() as usize;
        let cell_size = self.min_distance * 1.5;

        // Place nodes in a grid pattern with some randomness
        graph
            .containment_scope_nodes_with_indices(containment_scope)
            .enumerate()
            .map(|(i, (node_idx, _node))| {
                let row = i / grid_size;
                let col = i % grid_size;

                // Calculate base position with spacing based on component sizes
                let base = Point::new(col as f32 * cell_size, row as f32 * cell_size);

                // Add some randomness to avoid perfect grid alignment
                let jitter =
                    Point::new(rng.random_range(-20.0..20.0), rng.random_range(-20.0..20.0));

                (node_idx, base.add_point(jitter))
            })
            .collect()
    }

    /// Run force-directed layout algorithm
    fn run_force_simulation<'a>(
        &self,
        graph: &Graph<'a>,
        containment_scope: &ContainmentScope,
        component_sizes: &HashMap<NodeIndex, Size>,
    ) -> HashMap<NodeIndex, Point> {
        // Initialize positions in a grid pattern
        let mut positions = self.initialize_positions(graph, containment_scope, component_sizes);
        let mut velocities: HashMap<NodeIndex, (f32, f32)> = HashMap::new();

        // Initialize velocities
        for &node_idx in positions.keys() {
            velocities.insert(node_idx, (0.0, 0.0));
        }

        // Run simulation for fixed number of iterations
        for _ in 0..self.iterations {
            // Calculate forces between all components
            let mut forces: HashMap<NodeIndex, (f32, f32)> = HashMap::new();

            // Initialize forces
            for &node_idx in positions.keys() {
                forces.insert(node_idx, (0.0, 0.0));
            }

            // Get all nodes for iteration
            let nodes: Vec<NodeIndex> = positions.keys().copied().collect();

            // Add repulsive forces between all components
            for &node_i in &nodes {
                for &node_j in &nodes {
                    if node_i == node_j {
                        continue;
                    }

                    let pos_i = positions[&node_i];
                    let pos_j = positions[&node_j];

                    let trans = pos_i.sub_point(pos_j);

                    // Get component sizes to calculate appropriate distances
                    let size_i = *component_sizes
                        .get(&node_i)
                        .expect("Component size not found");
                    let size_j = *component_sizes
                        .get(&node_j)
                        .expect("Component size not found");

                    // Calculate minimum distance based on component sizes plus padding
                    let min_dist =
                        (size_i.width() + size_j.width() + size_i.height() + size_j.height()) / 4.0
                            + self.min_distance;

                    // Avoid division by zero
                    let distance = trans.hypot().max(1.0);

                    // Stronger repulsion when components are too close
                    let force_factor = if distance < min_dist {
                        self.repulsion_constant * (min_dist / distance).powf(2.0)
                    } else {
                        self.repulsion_constant / distance
                    };

                    // Normalize direction vector
                    let force_x = force_factor * trans.x() / distance;
                    let force_y = force_factor * trans.y() / distance;

                    // Add force to node_i
                    let (fx, fy) = forces[&node_i];
                    forces.insert(node_i, (fx + force_x, fy + force_y));
                }
            }

            // Add attractive forces (spring forces) between connected components
            for relation in graph.containment_scope_relations(containment_scope) {
                // Get node indices for source and target
                let source_node_idx = graph
                    .containment_scope_nodes_with_indices(containment_scope)
                    .find(|(_, node)| node.id == relation.source)
                    .map(|(idx, _)| idx);
                let target_node_idx = graph
                    .containment_scope_nodes_with_indices(containment_scope)
                    .find(|(_, node)| node.id == relation.target)
                    .map(|(idx, _)| idx);

                if let (Some(source), Some(target)) = (source_node_idx, target_node_idx)
                    && let (Some(&pos_source), Some(&pos_target)) =
                        (positions.get(&source), positions.get(&target))
                {
                    let dist = pos_source.sub_point(pos_target);

                    // Avoid division by zero
                    let distance = dist.hypot().max(1.0);

                    // Spring force (proportional to distance)
                    let force = self.spring_constant * distance;

                    // Normalize direction vector
                    let force_x = force * dist.x() / distance;
                    let force_y = force * dist.y() / distance;

                    // Subtract force from source (pull towards target)
                    let (fx_source, fy_source) = forces[&source];
                    forces.insert(source, (fx_source - force_x, fy_source - force_y));

                    // Add force to target (pull towards source)
                    let (fx_target, fy_target) = forces[&target];
                    forces.insert(target, (fx_target + force_x, fy_target + force_y));
                }
            }

            // Update velocities and positions
            for &node_idx in &nodes {
                let (force_x, force_y) = forces[&node_idx];
                let (vel_x, vel_y) = velocities[&node_idx];

                // Apply forces to update velocity (with damping)
                let new_vel_x = (vel_x + force_x) * self.damping_factor;
                let new_vel_y = (vel_y + force_y) * self.damping_factor;
                velocities.insert(node_idx, (new_vel_x, new_vel_y));

                // Update position based on velocity
                let pos = positions[&node_idx];
                positions.insert(
                    node_idx,
                    Point::new(pos.x() + new_vel_x, pos.y() + new_vel_y),
                );
            }
        }

        // Center the layout
        self.center_layout(&mut positions);

        positions
    }

    /// Center the layout around the origin
    fn center_layout(&self, positions: &mut HashMap<NodeIndex, Point>) {
        if positions.is_empty() {
            return;
        }

        // Find bounding box
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for pos in positions.values() {
            min_x = min_x.min(pos.x());
            min_y = min_y.min(pos.y());
            max_x = max_x.max(pos.x());
            max_y = max_y.max(pos.y());
        }

        // Calculate center offset
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Center everything
        for pos in positions.values_mut() {
            *pos = pos.sub_point(Point::new(center_x, center_y));
        }

        // Scale the layout if it's too large
        let width = max_x - min_x;
        let height = max_y - min_y;
        let max_dimension = 1200.0; // Maximum desired layout dimension

        if width > max_dimension || height > max_dimension {
            let scale_factor = max_dimension / width.max(height);
            for pos in positions.values_mut() {
                *pos = pos.scale(scale_factor);
            }
        }
    }
}

impl ComponentEngine for Engine {
    fn calculate<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts,
    ) -> ContentStack<Layout> {
        let mut content_stack = ContentStack::<Layout>::new();
        let mut positioned_content_sizes = HashMap::<NodeIndex, Size>::new();

        for containment_scope in graph.containment_scopes() {
            debug!(
                scope_node_count = graph.containment_scope_nodes_with_indices(containment_scope).count();
                "Processing containment scope with force layout"
            );

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
                .map(|(idx, shape_with_text)| (*idx, shape_with_text.size()))
                .collect();

            // Run force-directed layout to get positions
            let positions = self.run_force_simulation(graph, containment_scope, &component_sizes);

            // Build the final component list using the pre-configured shapes
            let components: Vec<Component> = graph
                .containment_scope_nodes_with_indices(containment_scope)
                .map(|(node_idx, node)| {
                    let position = *positions.get(&node_idx).unwrap();
                    let shape_with_text = component_shapes.remove(&node_idx).unwrap();

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
                .containment_scope_relations(containment_scope)
                .filter_map(|relation| {
                    // Only include relations between visible components
                    // (not including relations within inner blocks)
                    if let (Some(&source_index), Some(&target_index)) = (
                        component_indices.get(&relation.source),
                        component_indices.get(&relation.target),
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

            let positioned_content = PositionedContent::new(Layout {
                components,
                relations,
            });

            if let Some(container) = containment_scope.container() {
                // If this layer is a container, we need to adjust its size based on its contents
                let size = positioned_content.layout_size();
                debug!(
                    container_id:? = graph.node_from_idx(container).id,
                    size:? = size;
                    "Recording container size for force layout"
                );
                positioned_content_sizes.insert(container, size);
            }
            content_stack.push(positioned_content);
        }

        adjust_positioned_contents_offset(&mut content_stack, graph);
        content_stack
    }
}
