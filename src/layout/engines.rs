//! Layout engine factory module
//!
//! This module provides a system for selecting and using different layout engines
//! based on the layout_engine attribute in the diagram. It supports both component
//! and sequence diagram types, with different algorithm options for each.
//!
//! The module uses a builder pattern for creating and configuring layout engines.
//!
//! The output format is LayeredLayout: A flattened structure with layers for easier rendering.

// Layout engine modules with different implementations
mod basic;
mod force;
mod sugiyama;

use crate::{
    ast::{DiagramKind, LayoutEngine, TypeId},
    draw,
    geometry::{self, Insets},
    graph::{Collection, Graph},
    layout::{
        component,
        layer::{LayeredLayout, LayoutContent},
        sequence,
    },
};
use log::trace;
use std::collections::HashMap;

use super::layer::ContentStack;

/// Enum to store different layout results based on diagram type
/// Contains the direct layout information without any embedded diagram data
#[derive(Debug, Clone)]
pub enum LayoutResult<'a> {
    // TODO: Do I need this?
    Component(ContentStack<component::Layout<'a>>),
    Sequence(ContentStack<sequence::Layout<'a>>),
}

impl<'a> LayoutResult<'a> {
    /// Calculate the size of this layout, using the appropriate sizing implementation
    fn calculate_size(&self) -> geometry::Size {
        match self {
            LayoutResult::Component(layout) => layout.layout_size(),
            LayoutResult::Sequence(layout) => layout.layout_size(),
        }
    }
}

/// Map type containing pre-calculated layout information for embedded diagrams,
/// indexed by the TypeId of the node containing the embedded diagram
pub type EmbeddedLayouts<'a> = HashMap<TypeId, LayoutResult<'a>>;

// Trait defining the interface for component diagram layout engines
pub trait ComponentEngine {
    /// Calculate layout for a component diagram
    ///
    /// - `graph`: The graph representing the diagram to layout
    /// - `embedded_layouts`: Pre-calculated layouts for any embedded diagrams,
    ///   indexed by their TypeId. When a node contains an embedded diagram,
    ///   its size should be determined by looking up its layout here rather than
    ///   calculating it again.
    fn calculate<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<component::Layout<'a>>;
}

/// Trait defining the interface for sequence diagram layout engines
pub trait SequenceEngine {
    /// Calculate layout for a sequence diagram
    ///
    /// - `graph`: The graph representing the diagram to layout
    /// - `embedded_layouts`: Pre-calculated layouts for any embedded diagrams,
    ///   indexed by their TypeId. When a node contains an embedded diagram,
    ///   its size should be determined by looking up its layout here rather than
    ///   calculating it again.
    fn calculate<'a>(
        &self,
        graph: &'a Graph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> ContentStack<sequence::Layout<'a>>;
}

/// Builder for creating and configuring layout engines.
/// Builder is not reuseable after build() is called.
#[derive(Default)]
pub struct EngineBuilder {
    // Cache for reusing engines with the same configuration
    component_engines: HashMap<LayoutEngine, Box<dyn ComponentEngine>>,
    sequence_engines: HashMap<LayoutEngine, Box<dyn SequenceEngine>>,

    // Configuration options
    component_padding: Insets,
    min_component_spacing: f32,
    message_spacing: f32,
    force_simulation_iterations: usize,
}

impl EngineBuilder {
    /// Create a new engine builder with default engine cache and configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the padding around components
    pub fn with_component_padding(mut self, padding: Insets) -> Self {
        self.component_padding = padding;
        self
    }

    /// Set the minimum spacing between components
    pub fn with_component_spacing(mut self, spacing: f32) -> Self {
        self.min_component_spacing = spacing;
        self
    }

    /// Set the spacing between sequence diagram messages
    pub fn with_message_spacing(mut self, spacing: f32) -> Self {
        self.message_spacing = spacing;
        self
    }

    /// Set the number of iterations for force-directed layout simulation
    pub fn with_force_iterations(mut self, iterations: usize) -> Self {
        self.force_simulation_iterations = iterations;
        self
    }

    /// Get a component engine of the specified type with configured options
    pub fn component_engine(&mut self, engine_type: LayoutEngine) -> &dyn ComponentEngine {
        let engine = self
            .component_engines
            .entry(engine_type)
            .or_insert_with(|| {
                let engine: Box<dyn ComponentEngine> = match engine_type {
                    LayoutEngine::Basic => {
                        let mut e = basic::Component::new();
                        // Configure the engine with our settings
                        e.set_padding(self.component_padding);
                        e.set_min_spacing(self.min_component_spacing);
                        Box::new(e)
                    }
                    LayoutEngine::Force => {
                        let mut e = force::Component::new();
                        // Configure the force-directed engine
                        e.set_padding(self.component_padding)
                            .set_text_padding(self.message_spacing)
                            .set_min_distance(self.min_component_spacing)
                            .set_iterations(self.force_simulation_iterations);
                        Box::new(e)
                    }
                    LayoutEngine::Sugiyama => {
                        let mut e = sugiyama::Component::new();
                        // Configure the hierarchical engine
                        e.set_horizontal_spacing(self.min_component_spacing);
                        e.set_vertical_spacing(self.min_component_spacing);
                        e.set_container_padding(self.component_padding);
                        Box::new(e)
                    }
                };
                engine
            });
        // Dereference to avoid returning reference to temporary
        &**engine
    }

    /// Get a sequence engine of the specified type with configured options
    pub fn sequence_engine(&mut self, engine_type: LayoutEngine) -> &dyn SequenceEngine {
        let engine = self.sequence_engines.entry(engine_type).or_insert_with(|| {
            // Currently only Basic is supported for sequence diagrams
            let mut engine = basic::Sequence::new();
            // Configure the engine with our settings
            engine.set_message_spacing(self.message_spacing);
            engine.set_min_spacing(self.min_component_spacing);
            Box::new(engine)
        });
        // Dereference to avoid returning reference to temporary
        &**engine
    }

    /// Build a layered layout structure for rendering
    ///
    /// Flattens the diagram hierarchy into layers that can be rendered in sequence.
    /// This is a two-phase process:
    /// 1. Calculate layouts for all diagrams in post-order (innermost to outermost)
    /// 2. Adjust positions of embedded diagrams relative to their containers
    pub fn build<'a>(mut self, collection: &'a Collection<'a>) -> LayeredLayout<'a> {
        let mut layered_layout = LayeredLayout::new();

        let mut layout_info: HashMap<TypeId, LayoutResult<'a>> = HashMap::new();

        // Map from container ID to its layer index in the layered_layout
        let mut container_element_to_layer: HashMap<TypeId, usize> = HashMap::new();

        // Track container-embedded diagram relationships for position adjustment in the second phase
        // Format: (container_layer_idx, container_position, container_shape, embedded_layer_idx)
        let mut embedded_diagrams: Vec<(
            usize,
            draw::PositionedDrawable<draw::ShapeWithText>,
            usize,
        )> = Vec::new();

        // First phase: calculate all layouts
        for (type_id, graph) in collection.diagram_tree_in_post_order() {
            // Calculate the layout for this diagram using the appropriate engine
            let diagram = graph.diagram();
            let layout_result = match diagram.kind {
                DiagramKind::Component => {
                    let engine = self.component_engine(diagram.layout_engine);

                    let layout = engine.calculate(graph, &layout_info);
                    LayoutResult::Component(layout)
                }
                DiagramKind::Sequence => {
                    let engine = self.sequence_engine(diagram.layout_engine);

                    let layout = engine.calculate(graph, &layout_info);
                    LayoutResult::Sequence(layout)
                }
            };

            // Create and add the layer with the calculated layout
            // PERFORMANCE: Get rid of clone() if possible.
            let layer_content = match &layout_result {
                LayoutResult::Component(layout) => LayoutContent::Component(layout.clone()),
                LayoutResult::Sequence(layout) => LayoutContent::Sequence(layout.clone()),
            };

            // Add the layer to the layered layout and get its assigned index
            let layer_idx = layered_layout.add_layer(layer_content);

            // Record the mapping from container ID to its layer index
            if let Some(id) = type_id {
                container_element_to_layer.insert(id.clone(), layer_idx);
            }

            if !container_element_to_layer.is_empty() {
                match &layout_result {
                    LayoutResult::Component(layout) => {
                        // Check for embedded diagrams in each positioned content
                        for positioned_content in layout.iter() {
                            // Check for embedded diagrams in each positioned content
                            for component in &positioned_content.content().components {
                                if let Some(embedded_idx) =
                                    container_element_to_layer.get(component.node_id())
                                {
                                    // Store information needed to position the embedded diagram within its container:
                                    // (container layer index, container position, container shape, embedded diagram layer index)
                                    embedded_diagrams.push((
                                        layer_idx,
                                        component.drawable().clone(),
                                        *embedded_idx,
                                    ));
                                }
                            }
                        }
                    }
                    LayoutResult::Sequence(layout) => {
                        // Check for embedded diagrams in sequence layout
                        for positioned_content in layout.iter() {
                            for participant in &positioned_content.content().participants {
                                if let Some(embedded_idx) =
                                    container_element_to_layer.get(participant.component.node_id())
                                {
                                    // Store information needed to position the embedded diagram within a sequence participant:
                                    // (container layer index, participant position, participant shape, embedded diagram layer index)
                                    embedded_diagrams.push((
                                        layer_idx,
                                        participant.component.drawable().clone(),
                                        *embedded_idx,
                                    ));
                                }
                            }
                        }
                    }
                }
            };

            // Store the layout for embedded diagram references
            if let Some(id) = type_id {
                layout_info.insert(id.clone(), layout_result);
            }
        }

        // Second phase: Apply position adjustments and set up clipping bounds for embedded diagrams
        for (container_idx, positioned_shape, embedded_idx) in embedded_diagrams.into_iter().rev() {
            layered_layout.adjust_relative_position(
                container_idx,
                &positioned_shape,
                embedded_idx,
                self.component_padding,
            );
        }

        trace!(layered_layout:?; "Built layered layout");

        layered_layout
    }
}
