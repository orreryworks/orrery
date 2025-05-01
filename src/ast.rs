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
pub mod parser;
mod parser_types;
pub mod span;
