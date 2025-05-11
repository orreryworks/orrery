//! Layout engine factory module
//!
//! This module provides a system for selecting and using different layout engines
//! based on the layout_engine attribute in the diagram. It supports both component
//! and sequence diagram types, with different algorithm options for each.

// Layout engine modules with different implementations
mod basic;
mod force;
mod sugiyama;

use crate::{
    ast::LayoutEngine,
    error::FilamentError,
    graph::Graph,
    layout::{component, sequence},
};

/// Trait defining the interface for component diagram layout engines
pub trait ComponentEngine {
    /// Calculate layout for a component diagram
    fn calculate<'a>(&self, graph: &'a Graph) -> component::Layout<'a>;
}

/// Trait defining the interface for sequence diagram layout engines
pub trait SequenceEngine {
    /// Calculate layout for a sequence diagram
    fn calculate<'a>(&self, graph: &'a Graph) -> sequence::Layout<'a>;
}

/// Factory function to create the appropriate component layout engine
pub fn create_component_engine(
    engine_type: LayoutEngine,
) -> Result<Box<dyn ComponentEngine>, FilamentError> {
    match engine_type {
        LayoutEngine::Basic => Ok(Box::new(basic::Component::new())),
        LayoutEngine::Force => Ok(Box::new(force::Component::new())),
        LayoutEngine::Sugiyama => Ok(Box::new(sugiyama::Component::new())),
        // Future layout engines would be added here
    }
}

/// Factory function to create the appropriate sequence layout engine
pub fn create_sequence_engine(
    engine_type: LayoutEngine,
) -> Result<Box<dyn SequenceEngine>, FilamentError> {
    match engine_type {
        LayoutEngine::Basic => Ok(Box::new(basic::Sequence::new())),
        // Future layout engines would be added here
        _ => Err(FilamentError::Layout(format!(
            "Engine `{engine_type}` is not available for sequence diagram"
        ))),
    }
}
