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
mod sugiyama;

use std::collections::HashMap;

use log::trace;

use super::layer::ContentStack;
use crate::{
    error::FilamentError,
    geometry,
    identifier::Id,
    layout::{
        component,
        layer::{LayeredLayout, LayoutContent},
        positioning::LayoutBounds,
        sequence,
    },
    semantic::LayoutEngine,
    structure,
};

/// Enum to store different layout results based on diagram type
/// Contains the direct layout information without any embedded diagram data
#[derive(Debug, Clone)]
pub enum LayoutResult<'a> {
    // TODO: Do I need this?
    Component(ContentStack<component::Layout<'a>>),
    Sequence(ContentStack<sequence::Layout<'a>>),
}

impl<'a> LayoutResult<'a> {
    /// Calculate the coordinate offset needed to normalize this layout to start at origin.
    ///
    /// Embedded layouts may naturally have non-zero minimum points based on how their
    /// positioning engines calculate positions. This method returns the offset that should
    /// be applied to normalize the layout's coordinate system so its bounds start at origin (0, 0).
    ///
    /// Returns a Point that, when added to component positions, will shift the layout to
    /// start at the origin.
    pub fn normalize_offset(&self) -> geometry::Point {
        let bounds = self.layout_bounds();
        geometry::Point::new(-bounds.min_x(), -bounds.min_y())
    }

    /// Calculate the size of this layout, using the appropriate sizing implementation
    fn calculate_size(&self) -> geometry::Size {
        match self {
            LayoutResult::Component(layout) => layout.layout_size(),
            LayoutResult::Sequence(layout) => layout.layout_size(),
        }
    }

    /// Calculate the bounds of this layout
    fn layout_bounds(&self) -> geometry::Bounds {
        match self {
            LayoutResult::Component(layout) => layout
                .iter()
                .last()
                .map(|content| content.content().layout_bounds())
                .unwrap_or_default(),
            LayoutResult::Sequence(layout) => layout
                .iter()
                .last()
                .map(|content| content.content().layout_bounds())
                .unwrap_or_default(),
        }
    }
}

/// Map type containing pre-calculated layout information for embedded diagrams,
/// indexed by the Id of the node containing the embedded diagram
pub type EmbeddedLayouts<'a> = HashMap<Id, LayoutResult<'a>>;

// Trait defining the interface for component diagram layout engines
pub trait ComponentEngine {
    /// Calculate layout for a component diagram
    ///
    /// - `graph`: The graph representing the diagram to layout
    /// - `embedded_layouts`: Pre-calculated layouts for any embedded diagrams,
    ///   indexed by their Id. When a node contains an embedded diagram,
    ///   its size should be determined by looking up its layout here rather than
    ///   calculating it again.
    ///
    /// # Errors
    /// Returns `FilamentError::Layout` if the layout engine fails to calculate positions.
    fn calculate<'a>(
        &self,
        graph: &'a structure::ComponentGraph<'a, '_>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<ContentStack<component::Layout<'a>>, FilamentError>;
}

/// Trait defining the interface for sequence diagram layout engines
pub trait SequenceEngine {
    /// Calculate layout for a sequence diagram
    ///
    /// - `graph`: The graph representing the diagram to layout
    /// - `embedded_layouts`: Pre-calculated layouts for any embedded diagrams,
    ///   indexed by their Id. When a node contains an embedded diagram,
    ///   its size should be determined by looking up its layout here rather than
    ///   calculating it again.
    ///
    /// # Errors
    /// Returns `FilamentError::Layout` if the layout engine fails to calculate positions.
    fn calculate<'a>(
        &self,
        graph: &'a structure::SequenceGraph<'a>,
        embedded_layouts: &EmbeddedLayouts<'a>,
    ) -> Result<ContentStack<sequence::Layout<'a>>, FilamentError>;
}

/// Builder for creating and configuring layout engines.
/// Builder is not reuseable after build() is called.
#[derive(Default)]
pub struct EngineBuilder {
    // Cache for reusing engines with the same configuration
    component_engines: HashMap<LayoutEngine, Box<dyn ComponentEngine>>,
    sequence_engines: HashMap<LayoutEngine, Box<dyn SequenceEngine>>,

    // Configuration options
    padding: geometry::Insets,
    min_spacing: f32,
    horizontal_spacing: f32,
    vertical_spacing: f32,
    message_spacing: f32,
}

impl EngineBuilder {
    /// Create a new engine builder with default engine cache and configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the padding inside all shapes (components, participants, containers)
    pub fn with_padding(mut self, padding: geometry::Insets) -> Self {
        self.padding = padding;
        self
    }

    /// Set the minimum spacing between elements
    pub fn with_min_spacing(mut self, spacing: f32) -> Self {
        self.min_spacing = spacing;
        self
    }

    /// Set the horizontal spacing between elements
    pub fn with_horizontal_spacing(mut self, spacing: f32) -> Self {
        self.horizontal_spacing = spacing;
        self
    }

    /// Set the vertical spacing between elements
    pub fn with_vertical_spacing(mut self, spacing: f32) -> Self {
        self.vertical_spacing = spacing;
        self
    }

    /// Set the spacing between sequence diagram messages
    pub fn with_message_spacing(mut self, spacing: f32) -> Self {
        self.message_spacing = spacing;
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
                        e.set_padding(self.padding);
                        e.set_min_spacing(self.min_spacing);
                        Box::new(e)
                    }
                    LayoutEngine::Sugiyama => {
                        let mut e = sugiyama::Component::new();
                        // Configure the hierarchical engine
                        e.set_horizontal_spacing(self.horizontal_spacing);
                        e.set_vertical_spacing(self.vertical_spacing);
                        e.set_container_padding(self.padding);
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
            engine.set_text_padding(self.padding);
            engine.set_message_spacing(self.message_spacing);
            engine.set_min_spacing(self.min_spacing);
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
    ///
    /// # Errors
    /// Returns `FilamentError::Layout` if any layout engine fails to calculate positions.
    pub fn build<'a>(
        mut self,
        collection: &'a structure::DiagramHierarchy<'a, '_>,
    ) -> Result<LayeredLayout<'a>, FilamentError> {
        let mut layered_layout = LayeredLayout::new();

        let mut layout_info: HashMap<Id, LayoutResult<'a>> = HashMap::new();

        // Map from container ID to its layer index in the layered_layout
        let mut container_element_to_layer: HashMap<Id, usize> = HashMap::new();

        // Track the root diagram (which has no container_id)
        let mut root_layout: Option<(usize, LayoutResult<'a>)> = None;

        // Track container-embedded diagram relationships for position adjustment in the second phase
        // Format: (container_layer_idx, reference to container drawable, embedded_layer_idx)
        // Note: Using type inference for the reference lifetime - it borrows from layout_info
        let mut embedded_diagrams = Vec::new();

        // First phase: calculate all layouts
        for (container_id, graphed_diagram) in collection.iter_post_order() {
            // Calculate the layout for this diagram using the appropriate engine
            let diagram = graphed_diagram.ast_diagram();
            let layout_result = match graphed_diagram.graph_kind() {
                structure::GraphKind::ComponentGraph(graph) => {
                    let engine = self.component_engine(diagram.layout_engine());

                    let layout = engine.calculate(graph, &layout_info)?;
                    LayoutResult::Component(layout)
                }
                structure::GraphKind::SequenceGraph(graph) => {
                    let engine = self.sequence_engine(diagram.layout_engine());

                    let layout = engine.calculate(graph, &layout_info)?;
                    LayoutResult::Sequence(layout)
                }
            };

            // Create and add the layer with the calculated layout
            // PERF: Get rid of clone() if possible.
            let layer_content = match &layout_result {
                LayoutResult::Component(layout) => LayoutContent::Component(layout.clone()),
                LayoutResult::Sequence(layout) => LayoutContent::Sequence(layout.clone()),
            };

            // Add the layer to the layered layout and get its assigned index
            let layer_idx = layered_layout.add_layer(layer_content);

            // Record the mapping from container ID to its layer index
            if let Some(id) = container_id {
                container_element_to_layer.insert(id, layer_idx);
                layout_info.insert(id, layout_result);
            } else {
                root_layout = Some((layer_idx, layout_result));
            }
        }

        // Second phase: populate embedded_diagrams by checking all layouts for embedded content
        for (layer_idx, layout_result) in root_layout
            .iter()
            .map(|(idx, result)| (*idx, result))
            .chain(
                layout_info
                    .iter()
                    .map(|(&id, result)| (container_element_to_layer[&id], result)),
            )
        {
            match layout_result {
                LayoutResult::Component(layout) => {
                    for positioned_content in layout.iter() {
                        for component in positioned_content.content().components() {
                            if let Some(&embedded_idx) =
                                container_element_to_layer.get(&component.node_id())
                            {
                                // Store reference to drawable
                                embedded_diagrams.push((
                                    layer_idx,
                                    component.drawable(),
                                    embedded_idx,
                                ));
                            }
                        }
                    }
                }
                LayoutResult::Sequence(layout) => {
                    for positioned_content in layout.iter() {
                        for participant in positioned_content.content().participants().values() {
                            if let Some(&embedded_idx) =
                                container_element_to_layer.get(&participant.component().node_id())
                            {
                                // Store reference to drawable
                                embedded_diagrams.push((
                                    layer_idx,
                                    participant.component().drawable(),
                                    embedded_idx,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Third phase: Apply position adjustments and set up clipping bounds for embedded diagrams
        for (container_idx, positioned_shape, embedded_idx) in embedded_diagrams.into_iter() {
            layered_layout.adjust_relative_position(
                container_idx,
                positioned_shape,
                embedded_idx,
            )?;
        }

        trace!(layered_layout:?; "Built layered layout");

        Ok(layered_layout)
    }
}
