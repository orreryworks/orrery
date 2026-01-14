//! Filament - A diagram language for creating component and sequence diagrams
//!
//! This library provides parsing, layout, and rendering capabilities for the Filament
//! diagram language. It supports component diagrams and sequence diagrams with a
//! text-based DSL.

pub mod ast;
pub mod config;

mod error;
mod export;
mod layout;
mod structure;

pub use filament_core::{color, draw, identifier, semantic};

pub use error::FilamentError;

use std::fs;

use log::{debug, info, trace};

use filament_core::geometry::Insets;

use config::AppConfig;
use export::Exporter;

/// Builder for parsing and rendering Filament diagrams.
///
/// This provides an API for processing Filament diagrams through parsing,
/// layout, and rendering stages.
///
/// # Examples
///
/// ```rust,no_run
/// use filament::{DiagramBuilder, config::AppConfig};
///
/// let source = "diagram component; app: Rectangle;";
///
/// // With custom config
/// let config = AppConfig::default();
/// let builder = DiagramBuilder::new(config);
///
/// // Parse source to semantic model
/// let diagram = builder.parse(source)
///     .expect("Failed to parse");
///
/// // Render semantic model to SVG
/// let svg = builder.render_svg(&diagram)
///     .expect("Failed to render");
///
/// // Or use default config
/// let builder = DiagramBuilder::default();
/// ```
#[derive(Default)]
pub struct DiagramBuilder {
    config: AppConfig,
}

impl DiagramBuilder {
    /// Create a new diagram builder with the given configuration.
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
    /// let builder = DiagramBuilder::new(config);
    /// ```
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Parse source code into a semantic diagram.
    ///
    /// This performs lexing, parsing, desugaring, validation, and elaboration
    /// to produce a fully resolved semantic diagram model.
    ///
    /// # Arguments
    ///
    /// * `source` - Filament source code as a string
    ///
    /// # Errors
    ///
    /// Returns `FilamentError` for syntax errors, validation errors, or
    /// elaboration errors.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use filament::{DiagramBuilder, config::AppConfig};
    ///
    /// let source = "diagram component; app: Rectangle;";
    /// let builder = DiagramBuilder::new(AppConfig::default());
    /// let diagram = builder.parse(source)
    ///     .expect("Failed to parse diagram");
    /// ```
    pub fn parse(&self, source: &str) -> Result<semantic::Diagram, FilamentError> {
        info!("Building diagram AST");
        let elaborated_ast = ast::build_ast(&self.config, source)?;
        debug!("AST built successfully");
        trace!(elaborated_ast:?; "Elaborated AST");
        Ok(elaborated_ast)
    }

    /// Render a semantic diagram to SVG string.
    ///
    /// This transforms a semantic diagram through the layout and rendering
    /// pipeline to produce an SVG string.
    ///
    /// # Arguments
    ///
    /// * `diagram` - A semantic diagram to render
    ///
    /// # Errors
    ///
    /// Returns `FilamentError` for layout or rendering errors.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use filament::{DiagramBuilder, config::AppConfig};
    ///
    /// let source = "diagram component; app: Rectangle;";
    /// let builder = DiagramBuilder::new(AppConfig::default());
    ///
    /// let diagram = builder.parse(source)
    ///     .expect("Failed to parse");
    ///
    /// let svg = builder.render_svg(&diagram)
    ///     .expect("Failed to render diagram");
    ///
    /// println!("{}", svg);
    /// ```
    pub fn render_svg(&self, diagram: &semantic::Diagram) -> Result<String, FilamentError> {
        // Build the diagram structure/graph
        info!(diagram_kind:? = diagram.kind(); "Building diagram structure");
        let diagram_hierarchy = structure::DiagramHierarchy::from_diagram(diagram)?;
        debug!("Structure built successfully");

        // Create layout engine
        let engine_builder = layout::EngineBuilder::new()
            .with_padding(Insets::uniform(35.0))
            .with_min_spacing(50.0)
            .with_horizontal_spacing(50.0)
            .with_vertical_spacing(50.0)
            .with_message_spacing(60.0);

        // Calculate layout
        info!("Processing diagrams in hierarchy");
        let layered_layout = engine_builder.build(&diagram_hierarchy)?;
        info!(layers_count = layered_layout.len(); "Layout calculated");

        // Render to SVG using a temporary file
        // TODO: In the future, modify SvgBuilder to support in-memory rendering
        let temp_file =
            tempfile::NamedTempFile::new().map_err(|err| FilamentError::Export(Box::new(err)))?;
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let mut svg_exporter = export::svg::SvgBuilder::new(&temp_path)
            .with_style(self.config.style())
            .with_diagram(diagram)
            .build()?;

        svg_exporter.export_layered_layout(&layered_layout)?;

        // Read the SVG content back from the temp file
        let svg_string = fs::read_to_string(&temp_path).map_err(FilamentError::Io)?;

        info!("SVG rendered successfully");
        Ok(svg_string)
    }
}
