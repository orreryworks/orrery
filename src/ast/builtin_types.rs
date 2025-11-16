//! Built-in base type names
//!
//! This module defines string constants for the built-in base types used in
//! the Filament type system. These are primarily used during desugaring to
//! inject default type names for syntactic sugar patterns.

/// Built-in base type for relations (arrows)
pub const ARROW: &str = "Arrow";

/// Built-in base type for notes
pub const NOTE: &str = "Note";

/// Built-in base type for fragments
pub const FRAGMENT: &str = "Fragment";

/// Built-in base type for activations
pub const ACTIVATE: &str = "Activate";
