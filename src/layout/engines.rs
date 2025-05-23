//! Layout engine factory module
//!
//! This module provides a system for selecting and using different layout engines
//! based on the layout_engine attribute in the diagram. It supports both component
//! and sequence diagram types, with different algorithm options for each.
//!
//! The module uses a builder pattern for creating and configuring layout engines.

// Layout engine modules with different implementations
mod basic;
mod force;
mod sugiyama;

use crate::{
    ast::{DiagramKind, LayoutEngine, TypeId},
    graph::{Collection, Graph},
    layout::{component, sequence},
};
use std::collections::HashMap;

/// Map type containing pre-calculated layout information for embedded diagrams,
/// indexed by the TypeId of the node containing the embedded diagram
pub type EmbeddedLayouts<'a> = HashMap<TypeId, LayoutResult<'a>>;

/// Enum to store different layout results based on diagram type
/// Used both for returning the main diagram layout and for storing
/// layouts of embedded diagrams that will be referenced by parent diagrams
#[derive(Debug)]
pub enum LayoutResult<'a> {
    Component(component::Layout<'a>),
    Sequence(sequence::Layout<'a>),
}

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
    ) -> component::Layout<'a>;
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
    ) -> sequence::Layout<'a>;
}

/// Builder for creating and configuring layout engines.
/// Builder is not reuseable after build() is called.
pub struct EngineBuilder {
    // Cache for reusing engines with the same configuration
    component_engines: HashMap<LayoutEngine, Box<dyn ComponentEngine>>,
    sequence_engines: HashMap<LayoutEngine, Box<dyn SequenceEngine>>,

    // Configuration options
    component_padding: f32,
    min_component_spacing: f32,
    message_spacing: f32,
    force_simulation_iterations: usize,
    //
}

impl EngineBuilder {
    /// Create a new engine builder with default engine cache and configuration
    pub fn new() -> Self {
        Self {
            component_engines: HashMap::new(),
            sequence_engines: HashMap::new(),
            component_padding: 40.0,
            min_component_spacing: 40.0,
            message_spacing: 50.0,
            force_simulation_iterations: 300,
        }
    }

    /// Set the padding around components
    pub fn with_component_padding(mut self, padding: f32) -> Self {
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
                        e.set_padding(self.component_padding);
                        e.set_min_distance(self.min_component_spacing);
                        e.set_iterations(self.force_simulation_iterations);
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

    /// Process all graphs in the collection in post-order
    ///
    /// 1. First processes all embedded (child) diagrams
    /// 2. Then processes parent diagrams, using the layouts of embedded diagrams
    ///
    /// Each diagram can specify its own layout engine, which will be respected during processing.
    pub fn build<'a>(
        mut self,
        collection: &'a Collection<'a>,
    ) -> (LayoutResult<'a>, EmbeddedLayouts<'a>) {
        let mut embedded_layouts = HashMap::new();

        // We'll get (type_id, graph) pairs, where type_id is None for the root
        let mut main_layout = None;

        // Process in post-order to ensure inner diagrams are processed before outer ones
        // This means child diagrams are always processed before their parents
        for (type_id, graph) in collection.hierarchy_in_post_order() {
            // Access the diagram directly from the graph
            let diagram = graph.diagram();
            let layout_result = match diagram.kind {
                DiagramKind::Component => {
                    // Get the appropriate component engine for this diagram
                    let engine = self.component_engine(diagram.layout_engine);

                    let layout = engine.calculate(graph, &embedded_layouts);
                    LayoutResult::Component(layout)
                }
                DiagramKind::Sequence => {
                    // Get the appropriate sequence engine for this diagram
                    let engine = self.sequence_engine(diagram.layout_engine);

                    let layout = engine.calculate(graph, &embedded_layouts);
                    LayoutResult::Sequence(layout)
                }
            };

            // If this is the root graph (type_id is None), set it as the main layout
            // Otherwise, add it to embedded_layouts for reference by parent diagrams
            if let Some(id) = type_id {
                embedded_layouts.insert(id.clone(), layout_result);
            } else {
                main_layout = Some(layout_result);
            }
        }

        // The root layout should always exist
        (main_layout.unwrap(), embedded_layouts)
    }
}
