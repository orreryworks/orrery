//! Orrery Core Types and Definitions
//!
//! This crate provides the foundational types and definitions for the Orrery
//! diagram language. It includes:
//!
//! - **Identifiers**: Efficient string-interned identifiers ([`identifier::Id`])
//! - **Colors**: Color handling with CSS color support ([`color::Color`])
//! - **Geometry**: Basic geometric types ([`geometry`] module)
//! - **Draw**: Visual definitions for diagram elements ([`draw`] module)
//! - **Semantic**: Semantic model types for diagrams ([`semantic`] module)

pub mod color;
pub mod draw;
pub mod geometry;
pub mod identifier;
pub mod semantic;
