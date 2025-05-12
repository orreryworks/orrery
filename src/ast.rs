/// AST module for the Filament language
///
/// This module contains the types and functionality for parsing, elaborating,
/// and working with Filament AST (Abstract Syntax Tree).
///
/// - `parser`: Contains the core parsing logic using nom
/// - `span`: Provides location tracking for AST elements
/// - `parser_types`: Contains spanned versions of parser types with source location tracking
/// - `elaborate`: Handles AST elaboration with rich error diagnostics
pub mod elaborate;
mod elaborate_types;
pub mod parser;
mod parser_types;
pub mod span;

use crate::{config::AppConfig, error::FilamentError};
pub use elaborate_types::*;

/// Builds a fully elaborated AST from source code.
///
/// This function centralizes the process of building a Filament diagram AST by:
/// 1. Parsing the source code into an initial AST
/// 2. Elaborating the AST to resolve references and validate the structure
/// 3. Handling error wrapping and source code association for diagnostics
///
/// # Arguments
///
/// * `source` - The source code to parse and elaborate
///
/// # Returns
///
/// The elaborated diagram AST or a `FilamentError`
pub fn build_ast(cfg: &AppConfig, source: &str) -> Result<elaborate_types::Diagram, FilamentError> {
    // Step 1: Parse the diagram
    let parsed_ast = parser::build_diagram(source)?;

    // Step 2: Elaborate the AST with rich error handling
    let elaborate_builder = elaborate::Builder::new(cfg, source);
    elaborate_builder
        .build(&parsed_ast)
        .map_err(|e| FilamentError::new_elaboration_error(e, source))
}
