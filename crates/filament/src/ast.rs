/// AST module for the Filament language
///
/// This module contains the parsing, validation, and elaboration infrastructure for
/// transforming Filament source code into semantic diagram models.
///
/// ## Module Responsibilities
///
/// The `ast` module handles:
/// - **Lexing**: Tokenizing source code
/// - **Parsing**: Building syntactic AST with span information
/// - **Validation**: Checking diagram semantics at syntax level
/// - **Elaboration**: Transforming parser AST into semantic model (see [`crate::semantic`])
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
/// ## Output
///
/// The `build_ast()` function produces a [`crate::semantic::Diagram`] - the fully elaborated
/// semantic model ready for structure analysis and layout.
///
/// ## Internal Modules
/// - `span`: Provides location tracking for AST elements (public)
/// - `parser_types`: Spanned parser AST types (internal)
/// - `desugar`: AST normalization (internal)
/// - `elaborate`: Elaboration phase (internal)
/// - `elaborate_utils`: Type definitions and attribute extractors (internal)
mod builtin_types;
mod desugar;
mod elaborate;
mod elaborate_utils;
mod lexer;
mod parser;
#[cfg(test)]
mod parser_tests;
mod parser_types;
pub mod span;
mod tokens;
mod validate;

pub use parser_types::DiagramKind;

use crate::{config::AppConfig, error::FilamentError, semantic};

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
pub fn build_ast(cfg: &AppConfig, source: &str) -> Result<semantic::Diagram, FilamentError> {
    // Step 1: Tokenize the source code
    let tokens =
        lexer::tokenize(source).map_err(|err| FilamentError::new_lexer_error(err, source))?;

    // Step 2: Parse the tokens into AST
    let parsed_ast = parser::build_diagram(&tokens)
        .map_err(|err| FilamentError::new_parse_error(err, source))?;

    // Step 3: Apply desugaring transformations
    let desugared_ast = desugar::desugar(parsed_ast);

    // Step 4: Validate diagram semantics at syntax level before elaboration
    if let parser_types::Element::Diagram(diagram) = desugared_ast.inner() {
        validate::validate_diagram(diagram)
            .map_err(|err| FilamentError::new_validation_error(err, source))?;
    }

    // Step 5: Elaborate the AST with rich error handling
    let elaborate_builder = elaborate::Builder::new(cfg, source);
    elaborate_builder
        .build(&desugared_ast)
        .map_err(|err| FilamentError::new_elaboration_error(err, source))
}
