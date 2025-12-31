//! Sugiyama (hierarchical) layout engines
//!
//! This module contains layout engines that use the Sugiyama algorithm (also known as
//! hierarchical or layered layout) to position components. This layout is particularly
//! effective for visualizing hierarchical structures and directed graphs with minimal
//! edge crossings.

mod component;

pub use component::Engine as Component;
