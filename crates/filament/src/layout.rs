//! Layout engine for positioning diagram elements.
//!
//! This module transforms semantic diagrams into positioned elements ready for
//! rendering. It handles node positioning, edge routing, and layer organization
//! for z-ordering across both component and sequence diagram types.
//!
//! # Pipeline Position
//!
//! ```text
//! Semantic Model (Diagram)
//!     ↓ structure
//! DiagramHierarchy
//!     ↓ layout (this module)
//! LayeredLayout
//!     ↓ export
//! Output
//! ```
//!
//! # Submodules
//!
//! - [`component`] - Positioned diagram elements and their relationships (used across
//!   all diagram kinds)
//! - [`layer`] - Layer organization and z-ordering for rendering
//! - [`positioning`] - Reusable positioning algorithms for layout engines
//! - [`sequence`] - Sequence diagram layout (participants, messages, activations)
//!
//! # Re-exports
//!
//! - [`EngineBuilder`] - Builder for creating and configuring layout engines

pub mod component;
mod engines;
pub mod layer;
pub mod positioning;
pub mod sequence;

// Public re-export of the engine builder for easier access
pub use engines::EngineBuilder;
