//! # Orrery Parser
//!
//! Parser for the Orrery diagram language. This crate provides the
//! parsing pipeline from source text to semantic diagram representation.
//!
//! ## Usage
//!
//! ```
//! # use std::path::Path;
//! # use bumpalo::Bump;
//! # use orrery_parser::{parse, ElaborateConfig, InMemorySourceProvider};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let arena = Bump::new();
//! let source = r#"
//!     diagram component;
//!     user: Rectangle;
//!     server: Rectangle;
//!     user -> server: "Request";
//! "#;
//!
//! let mut provider = InMemorySourceProvider::new();
//! provider.add_file("main.orr", source);
//!
//! let diagram = parse(&arena, Path::new("main.orr"), provider, ElaborateConfig::default())
//!     .map_err(|e| e.to_string())?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod source_map;
pub mod source_provider;

mod builtin_types;
mod desugar;
mod elaborate;
mod elaborate_utils;
mod file_id;
mod lexer;
mod parser;
#[cfg(test)]
mod parser_tests;
mod parser_types;
mod resolver;
mod span;
mod tokens;
mod validate;

pub use elaborate::ElaborateConfig;
pub use source_provider::{InMemorySourceProvider, SourceProvider};
pub use span::Span;

use std::path::Path;

use bumpalo::Bump;

use orrery_core::semantic::Diagram;

use elaborate::Builder;
use error::ParseError;
use resolver::Resolver;

/// Parse an Orrery file into a semantic diagram.
///
/// This is the main entry point for parsing Orrery diagram source code.
/// It orchestrates the complete parsing pipeline:
///
/// 1. **Resolve** — Recursively load the root file and all its imports via
///    the [`SourceProvider`], building a virtual address space in the
///    `SourceMap` and populating the import tree. For each file:
///    - **Tokenize** — Convert source text to tokens
///    - **Parse** — Build an AST from tokens
/// 2. **Desugar** — Normalize syntax sugar and flatten imported types
/// 3. **Validate** — Check semantic validity
/// 4. **Elaborate** — Transform to semantic model
///
/// # Arguments
///
/// * `arena` — A [`Bump`] arena that owns all source text. The arena must
///   outlive the returned error (if any), since [`ParseError`] borrows
///   source data from it.
/// * `root_path` — Path to the root/entry Orrery file.
/// * `provider` — A [`SourceProvider`] implementation that resolves import
///   paths and reads source text.
/// * `config` — Configuration for the elaboration phase.
///
/// # Returns
///
/// Returns the parsed [`orrery_core::semantic::Diagram`] on success,
/// or a [`ParseError`] with location information on failure.
///
/// # Example
///
/// ```
/// # use std::path::Path;
/// # use bumpalo::Bump;
/// # use orrery_parser::{parse, ElaborateConfig, InMemorySourceProvider};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let arena = Bump::new();
/// let mut provider = InMemorySourceProvider::new();
/// provider.add_file("main.orr", "diagram component; box: Rectangle;");
///
/// let diagram = parse(&arena, Path::new("main.orr"), provider, ElaborateConfig::default())
///     .map_err(|e| e.to_string())?;
/// # Ok(())
/// # }
/// ```
pub fn parse<'a, P: SourceProvider>(
    arena: &'a Bump,
    root_path: &Path,
    provider: P,
    config: ElaborateConfig,
) -> Result<Diagram, ParseError<'a>> {
    // Step 1: Resolve — load all files recursively via the provider
    let resolver = Resolver::new(arena, provider);
    let resolved = resolver.resolve(root_path)?;
    let (file_ast, source_map) = resolved.into_parts();

    // Step 2: Desugar — normalize syntax sugar, flatten imported types
    let desugared = desugar::desugar(file_ast);

    // Step 3: Validate — check semantic validity
    if let Err(diags) = validate::validate(&desugared) {
        return Err(ParseError::new(diags, source_map));
    }

    // Step 4: Elaborate — transform to semantic model
    let builder = Builder::new(config);
    builder
        .build(&desugared)
        .map_err(|diag| ParseError::from_diagnostic(diag, source_map))
}
