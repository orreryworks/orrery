//! Force-directed component layout engine
//!
//! This module implements a force-directed graph layout algorithm
//! for component diagrams.

use crate::{
    graph::Graph,
    layout::{
        common::{Component, Point, Size},
        component::{Layout, LayoutRelation},
        engines::ComponentEngine,
        positioning::calculate_element_size,
    },
};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

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
    min_component_width: f32,
    min_component_height: f32,
    text_padding: f32,
    min_distance: f32,
}

impl Engine {
    /// Create a new force component layout engine
    pub fn new() -> Self {
        Self {
            iterations: 300,
            spring_constant: 0.005,
            repulsion_constant: 10000.0,
            damping_factor: 0.95,
            min_component_width: 100.0,
            min_component_height: 60.0,
            text_padding: 20.0,
            min_distance: 150.0,
        }
    }

    /// Calculate sizes for all components
    fn calculate_component_sizes(&self, graph: &Graph) -> HashMap<NodeIndex, Size> {
        graph
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
            .collect()
    }

    /// Initialize random positions for components
    fn initialize_positions(
        &self,
        graph: &Graph,
        _component_sizes: &HashMap<NodeIndex, Size>,
    ) -> HashMap<NodeIndex, Point> {
        use rand::Rng;
        let mut rng = rand::rng();

        // Calculate approximate grid dimensions
        let node_count = graph.node_indices().count();
        let grid_size = (node_count as f32).sqrt().ceil() as usize;
        let cell_size = self.min_distance * 1.5;

        // Place nodes in a grid pattern with some randomness
        graph
            .node_indices()
            .enumerate()
            .map(|(i, node_idx)| {
                let row = i / grid_size;
                let col = i % grid_size;

                // Calculate base position with spacing based on component sizes
                let base_x = col as f32 * cell_size;
                let base_y = row as f32 * cell_size;

                // Add some randomness to avoid perfect grid alignment
                let jitter_x = rng.random_range(-20.0..20.0);
                let jitter_y = rng.random_range(-20.0..20.0);

                (
                    node_idx,
                    Point {
                        x: base_x + jitter_x,
                        y: base_y + jitter_y,
                    },
                )
            })
            .collect()
    }

    /// Run force-directed layout algorithm
    fn run_force_simulation(
        &self,
        graph: &Graph,
        component_sizes: &HashMap<NodeIndex, Size>,
    ) -> HashMap<NodeIndex, Point> {
        // Initialize positions in a grid pattern
        let mut positions = self.initialize_positions(graph, component_sizes);
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

                    let dx = pos_i.x - pos_j.x;
                    let dy = pos_i.y - pos_j.y;

                    // Get component sizes to calculate appropriate distances
                    let default_size = Size {
                        width: self.min_component_width,
                        height: self.min_component_height,
                    };
                    let size_i = component_sizes.get(&node_i).unwrap_or(&default_size);
                    let size_j = component_sizes.get(&node_j).unwrap_or(&default_size);

                    // Calculate minimum distance based on component sizes plus padding
                    let min_dist = (size_i.width + size_j.width + size_i.height + size_j.height)
                        / 4.0
                        + self.min_distance;

                    // Avoid division by zero
                    let distance = (dx * dx + dy * dy).sqrt().max(1.0);

                    // Stronger repulsion when components are too close
                    let force_factor = if distance < min_dist {
                        self.repulsion_constant * (min_dist / distance).powf(2.0)
                    } else {
                        self.repulsion_constant / distance
                    };

                    // Normalize direction vector
                    let force_x = force_factor * dx / distance;
                    let force_y = force_factor * dy / distance;

                    // Add force to node_i
                    let (fx, fy) = forces[&node_i];
                    forces.insert(node_i, (fx + force_x, fy + force_y));
                }
            }

            // Add attractive forces (spring forces) between connected components
            for edge_idx in graph.edge_indices() {
                let (source, target) = graph.edge_endpoints(edge_idx).unwrap();

                if let (Some(&pos_source), Some(&pos_target)) =
                    (positions.get(&source), positions.get(&target))
                {
                    let dx = pos_source.x - pos_target.x;
                    let dy = pos_source.y - pos_target.y;

                    // Avoid division by zero
                    let distance = (dx * dx + dy * dy).sqrt().max(1.0);

                    // Spring force (proportional to distance)
                    let force = self.spring_constant * distance;

                    // Normalize direction vector
                    let force_x = force * dx / distance;
                    let force_y = force * dy / distance;

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
                    Point {
                        x: pos.x + new_vel_x,
                        y: pos.y + new_vel_y,
                    },
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
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_x = max_x.max(pos.x);
            max_y = max_y.max(pos.y);
        }

        // Calculate center offset
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        // Center everything
        for pos in positions.values_mut() {
            pos.x -= center_x;
            pos.y -= center_y;
        }

        // Scale the layout if it's too large
        let width = max_x - min_x;
        let height = max_y - min_y;
        let max_dimension = 1200.0; // Maximum desired layout dimension

        if width > max_dimension || height > max_dimension {
            let scale_factor = max_dimension / width.max(height);
            for pos in positions.values_mut() {
                pos.x *= scale_factor;
                pos.y *= scale_factor;
            }
        }
    }
}

impl ComponentEngine for Engine {
    fn calculate<'a>(&self, graph: &'a Graph) -> Layout<'a> {
        // Calculate sizes for all components
        let component_sizes = self.calculate_component_sizes(graph);

        // Run force-directed layout to get positions
        let positions = self.run_force_simulation(graph, &component_sizes);

        // Build components with positions and sizes
        let components: Vec<Component<'a>> = graph
            .node_indices()
            .filter_map(|node_idx| {
                let node = graph.node_weight(node_idx)?;
                let position = positions.get(&node_idx)?;
                let size = component_sizes.get(&node_idx)?;

                Some(Component {
                    node,
                    position: *position,
                    size: size.clone(),
                })
            })
            .collect();

        // Map node indices to component indices
        let node_to_index: HashMap<_, _> = graph
            .node_indices()
            .enumerate()
            .map(|(idx, node_idx)| (node_idx, idx))
            .collect();

        // Build relations between components
        let relations: Vec<LayoutRelation<'a>> = graph
            .edge_indices()
            .filter_map(|edge_idx| {
                let (source, target) = graph.edge_endpoints(edge_idx)?;
                let relation = graph.edge_weight(edge_idx)?;

                let source_index = *node_to_index.get(&source)?;
                let target_index = *node_to_index.get(&target)?;

                Some(LayoutRelation::new(relation, source_index, target_index))
            })
            .collect();

        Layout {
            components,
            relations,
        }
    }
}
