//! Force-directed layout engines
//!
//! This module contains layout engines that use force-directed algorithms to position
//! components. These layouts create natural-looking arrangements that highlight
//! structural relationships in the diagram.

mod component;

pub use component::Engine as Component;
