/// AST module for the Filament language
///
/// This module contains the types and functionality for parsing, elaborating,
/// and working with Filament AST (Abstract Syntax Tree).
///
/// ## Parser Architecture
///
/// Filament uses a modern two-stage parser architecture with full language support:
///
/// ### Two-Stage Parser Architecture
/// - `lexer`: Tokenizes source code into tokens
/// - `parser`: Parses tokens into AST nodes
/// - Used by `build_ast()` for parsing with complete language support
/// - Supports all Filament language features including relations and nested components
///
/// ### Span Architecture
/// - **Leaf types** (strings, literals, identifiers) are wrapped in `Spanned<T>` for precise location tracking
/// - **Composite types** use unwrapped collections and derive spans from inner elements via `span()` methods
/// - **Collection parsers** return `Vec<T>` directly instead of `Vec<Spanned<T>>` to avoid wrap-then-unwrap inefficiency
/// - **Error reporting** uses `from_span()` with extracted spans for rich diagnostics
///
/// ## Other Modules
/// - `span`: Provides location tracking for AST elements
/// - `parser_types`: Contains spanned versions of parser types with source location tracking
/// - `desugar`: Performs AST normalization between parsing and elaboration
/// - `elaborate`: Handles AST elaboration with rich error diagnostics
mod builtin_types;
mod desugar;
mod elaborate;
mod elaborate_types;
mod lexer;
mod parser;
#[cfg(test)]
mod parser_tests;
mod parser_types;
pub mod span;
mod tokens;
mod validate;

use crate::{
    config::AppConfig,
    error::{FilamentError, diagnostic::DiagnosticError},
};
pub use elaborate_types::*;
use span::Span;

/// Builds a fully elaborated AST from source code using the two-stage parser.
///
/// This function centralizes the process of building a Filament diagram AST by:
/// 1. Tokenizing the source code
/// 2. Parsing the tokens into an AST
/// 3. Applying desugaring transformations to normalize the AST
/// 4. Elaborating the AST to resolve references and validate the structure
/// 5. Handling error wrapping and source code association for diagnostics
///
/// ## Parser Features
///
/// The two-stage parser supports the complete Filament language specification:
/// - Component definitions, attributes, display names
/// - Built-in types (Rectangle, Oval, etc.)
/// - String literals with full escape sequence support
/// - Relations (arrows like `->`, `<-`, `<->`) with attributes and labels
/// - Nested components with `{}` syntax
/// - Type definitions and specifications
///
/// # Arguments
///
/// * `cfg` - Application configuration
/// * `source` - The source code to parse and elaborate
///
/// # Returns
///
/// The elaborated diagram AST or a `FilamentError`
///
/// # Examples
///
/// ```rust
/// use filament::{ast, config::AppConfig};
///
/// let source = r#"
///     diagram component;
///     app: Rectangle [color="blue"];
/// "#;
///
/// let config = AppConfig::default();
/// let result = ast::build_ast(&config, source);
/// ```
pub fn build_ast(cfg: &AppConfig, source: &str) -> Result<elaborate_types::Diagram, FilamentError> {
    // Step 1: Tokenize the source code
    let tokens = lexer::tokenize(source).map_err(|err| {
        let diag_err = DiagnosticError::from_span(
            format!("Lexer failed to parse input: {err}"),
            Span::default(), // TODO: Extract span from lexer error
            "lexer error",
            Some("Check for invalid characters or malformed tokens".to_string()),
        );
        FilamentError::new_parse_error(diag_err, source)
    })?;

    // Step 2: Parse the tokens into AST
    let parsed_ast =
        parser::build_diagram(&tokens).map_err(|e| FilamentError::new_parse_error(e, source))?;

    // Step 3: Apply desugaring transformations
    let desugared_ast = desugar::desugar(parsed_ast);

    // Step 4: Validate diagram semantics at syntax level before elaboration
    if let parser_types::Element::Diagram(diagram) = desugared_ast.inner() {
        validate::validate_diagram(diagram)
            .map_err(|e| FilamentError::new_validation_error(e, source))?;
    }

    // Step 5: Elaborate the AST with rich error handling
    let elaborate_builder = elaborate::Builder::new(cfg, source);
    elaborate_builder
        .build(&desugared_ast)
        .map_err(|e| FilamentError::new_elaboration_error(e, source))
}
