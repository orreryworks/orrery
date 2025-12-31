//! Basic layout engines
//!
//! This module contains layout engines that use simple, deterministic algorithms
//! to position components. These layouts produce consistent results and are the
//! default choice for most diagrams.

mod component;
mod sequence;

pub use component::Engine as Component;
pub use sequence::Engine as Sequence;
