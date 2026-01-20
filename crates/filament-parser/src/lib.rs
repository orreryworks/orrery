//! # Filament Parser
//!
//! Parser for the Filament diagram language. This crate provides the
//! parsing pipeline from source text to semantic diagram representation.
//!
//! ## Usage
//!
//! ```
//! # use filament_parser::{parse, ElaborateConfig, error::ParseError};
//!
//! fn main() -> Result<(), ParseError> {
//!     let source = r#"
//!         diagram component;
//!         user: Rectangle;
//!         server: Rectangle;
//!         user -> server: "Request";
//!     "#;
//!
//!     let diagram = parse(source, ElaborateConfig::default())?;
//!     Ok(())
//! }
//! ```

pub mod error;

mod builtin_types;
mod desugar;
mod elaborate;
mod elaborate_utils;
mod lexer;
mod parser;
#[cfg(test)]
mod parser_tests;
mod parser_types;
mod span;
mod tokens;
mod validate;

pub use elaborate::ElaborateConfig;
pub use span::Span;

use filament_core::semantic::Diagram;

use elaborate::Builder;
use error::ParseError;
use parser_types::Element;

/// Parse source text into a semantic diagram.
///
/// This is the main entry point for parsing Filament diagram source code.
/// It orchestrates the complete parsing pipeline:
///
/// 1. **Tokenize** - Convert source text to tokens
/// 2. **Parse** - Build AST from tokens
/// 3. **Desugar** - Normalize syntax sugar
/// 4. **Validate** - Check semantic validity
/// 5. **Elaborate** - Transform to semantic model
///
/// # Arguments
///
/// * `source` - The Filament diagram source code to parse
/// * `config` - Configuration for the elaboration phase (layout engine defaults)
///
/// # Returns
///
/// Returns the parsed [`filament_core::semantic::Diagram`] on success,
/// or a [`ParseError`] with location information on failure.
///
/// # Example
///
/// ```
/// # use filament_parser::{parse, ElaborateConfig, error::ParseError};
///
/// fn main() -> Result<(), ParseError> {
///     let source = "diagram component; box: Rectangle;";
///     let diagram = parse(source, ElaborateConfig::default())?;
///     Ok(())
/// }
/// ```
pub fn parse(source: &str, config: ElaborateConfig) -> Result<Diagram, ParseError> {
    // Step 1: Tokenize
    let tokens = lexer::tokenize(source).map_err(ParseError::from)?;

    // Step 2: Parse
    let ast = parser::build_diagram(&tokens).map_err(ParseError::from)?;

    // Step 3: Desugar
    let desugared = desugar::desugar(ast);

    // Step 4: Validate
    if let Element::Diagram(diagram) = desugared.inner() {
        validate::validate_diagram(diagram)?;
    }

    // Step 5: Elaborate
    let builder = Builder::new(config, source);
    builder.build(&desugared).map_err(ParseError::from)
}
