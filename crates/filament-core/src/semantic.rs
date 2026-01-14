//! Semantic diagram model types.
//!
//! This module contains the semantic representation of diagrams after parsing and elaboration.
//! These types represent the fully resolved, type-checked diagram structure before it is
//! transformed into a hierarchy and laid out for rendering.
//!
//! # Pipeline Position
//!
//! ```text
//! Source Text
//!     ↓ lexer
//! Tokens
//!     ↓ parser
//! Parser AST (parser_types) - syntactic structure with spans
//!     ↓ desugar + validate + elaborate
//! Semantic Model (these types) - resolved types, validated references
//!     ↓ structure
//! Hierarchy Graph (DiagramHierarchy)
//!     ↓ layout
//! Positioned Elements (LayeredLayout)
//!     ↓ export
//! SVG
//! ```
//!
//! # Organization
//!
//! - [`diagram`] - Core diagram structures: [`Diagram`], [`Scope`], [`Block`], [`LayoutEngine`]
//! - [`element`] - Diagram elements: [`Node`], [`Relation`], [`Fragment`], [`Note`], etc.

pub mod diagram;
pub mod element;

pub use diagram::*;
pub use element::*;
