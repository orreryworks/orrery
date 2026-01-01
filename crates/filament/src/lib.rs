//! Filament - A diagram language for creating component and sequence diagrams
//!
//! This library provides parsing, layout, and rendering capabilities for the Filament
//! diagram language. It supports component diagrams and sequence diagrams with a
//! text-based DSL.

pub mod ast;
pub mod color;
pub mod config;
pub mod draw;
pub mod geometry;
pub mod identifier;

mod error;
mod export;
mod layout;
mod structure;

pub use error::FilamentError;

use std::fs;

use log::{debug, info, trace};

use config::AppConfig;
use export::Exporter;

/// Builder for parsing and rendering Filament diagrams.
///
/// This provides a fluent API for processing Filament source code through
/// the full pipeline: parsing → layout → rendering.
///
/// # Examples
///
/// ```rust,no_run
/// use filament::DiagramBuilder;
///
/// let source = "diagram component; app: Rectangle;";
///
/// // Parse only
/// let diagram = DiagramBuilder::new(source)
///     .parse()
///     .expect("Failed to parse");
///
/// // Full rendering
/// let svg = DiagramBuilder::new(source)
///     .render_svg()
///     .expect("Failed to render");
/// ```
pub struct DiagramBuilder<'a> {
    source: &'a str,
    config: Option<AppConfig>,
}

impl<'a> DiagramBuilder<'a> {
    /// Create a new diagram builder from source code.
    ///
    /// # Arguments
    ///
    /// * `source` - Filament source code as a string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use filament::DiagramBuilder;
    ///
    /// let builder = DiagramBuilder::new("diagram component;");
    /// ```
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            config: None,
        }
    }

    /// Provide custom configuration for the diagram.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration including layout and style settings
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use filament::{DiagramBuilder, config::AppConfig};
    ///
    /// let config = AppConfig::default();
    /// let builder = DiagramBuilder::new("diagram component;")
    ///     .with_config(config);
    /// ```
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Parse the source code into an AST.
    ///
    /// This performs lexing, parsing, desugaring, validation, and elaboration
    /// to produce a fully resolved diagram AST.
    ///
    /// # Errors
    ///
    /// Returns `FilamentError` for syntax errors, validation errors, or
    /// elaboration errors.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use filament::DiagramBuilder;
    ///
    /// let diagram = DiagramBuilder::new("diagram component; app: Rectangle;")
    ///     .parse()
    ///     .expect("Failed to parse diagram");
    /// ```
    pub fn parse(self) -> Result<ast::Diagram, FilamentError> {
        let config = self.config.unwrap_or_default();
        info!("Building diagram AST");
        let elaborated_ast = ast::build_ast(&config, self.source)?;
        debug!("AST built successfully");
        trace!(elaborated_ast:?; "Elaborated AST");
        Ok(elaborated_ast)
    }

    /// Parse, layout, and render the diagram to SVG string.
    ///
    /// This is the main entry point for most use cases. It processes the
    /// source code through the complete pipeline and returns an SVG string.
    ///
    /// # Errors
    ///
    /// Returns `FilamentError` for parsing, layout, or rendering errors.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use filament::DiagramBuilder;
    ///
    /// let svg = DiagramBuilder::new("diagram component; app: Rectangle;")
    ///     .render_svg()
    ///     .expect("Failed to render diagram");
    ///
    /// println!("{}", svg);
    /// ```
    pub fn render_svg(self) -> Result<String, FilamentError> {
        let config = self.config.unwrap_or_default();

        // Parse the diagram
        info!("Building diagram AST");
        let elaborated_ast = ast::build_ast(&config, self.source)?;
        debug!("AST built successfully");
        trace!(elaborated_ast:?; "Elaborated AST");

        // Build the diagram structure/graph
        info!(diagram_kind:? = elaborated_ast.kind(); "Building diagram structure");
        let diagram_hierarchy = structure::DiagramHierarchy::from_diagram(&elaborated_ast)?;
        debug!("Structure built successfully");

        // Create layout engine
        let engine_builder = layout::EngineBuilder::new()
            .with_padding(geometry::Insets::uniform(35.0))
            .with_min_spacing(50.0)
            .with_horizontal_spacing(50.0)
            .with_vertical_spacing(50.0)
            .with_message_spacing(60.0);

        // Calculate layout
        info!("Processing diagrams in hierarchy");
        let layered_layout = engine_builder.build(&diagram_hierarchy);
        info!(layers_count = layered_layout.len(); "Layout calculated");

        // Render to SVG using a temporary file
        // TODO: In the future, modify SvgBuilder to support in-memory rendering
        let temp_file =
            tempfile::NamedTempFile::new().map_err(|e| FilamentError::Export(Box::new(e)))?;
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let mut svg_exporter = export::svg::SvgBuilder::new(&temp_path)
            .with_style(config.style())
            .with_diagram(&elaborated_ast)
            .build()?;

        svg_exporter.export_layered_layout(&layered_layout)?;

        // Read the SVG content back from the temp file
        let svg_string = fs::read_to_string(&temp_path).map_err(|e| FilamentError::Io(e))?;

        info!("SVG rendered successfully");
        Ok(svg_string)
    }
}
