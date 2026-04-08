//! Orrery - A diagram language for creating component and sequence diagrams.
//!
//! Parsing, layout, and rendering for the Orrery diagram language. Both component
//! diagrams and sequence diagrams are supported through a text-based DSL.

pub mod config;

mod error;
mod export;
mod layout;
mod structure;

pub use orrery_core::{color, draw, identifier, semantic};
pub use orrery_parser::error::ParseError;

pub use error::RenderError;

use std::{fs, path::Path};

pub use orrery_parser::{InMemorySourceProvider, SourceProvider};

use bumpalo::Bump;
use log::{debug, info, trace};

use orrery_core::geometry::Insets;
use orrery_parser::ElaborateConfig;

use config::AppConfig;
use export::Exporter;

/// Builder for parsing and rendering Orrery diagrams.
///
/// This provides an API for processing Orrery diagrams through parsing,
/// layout, and rendering stages.
///
/// # Examples
///
/// ```rust,no_run
/// # use std::path::Path;
/// # use bumpalo::Bump;
/// # use orrery::{DiagramBuilder, SourceProvider, InMemorySourceProvider, config::AppConfig};
/// let arena = Bump::new();
/// let mut provider = InMemorySourceProvider::new();
/// provider.add_file("app.orr", "diagram component; app: Rectangle;");
///
/// // With custom config
/// let config = AppConfig::default();
/// let builder = DiagramBuilder::new(config, &provider);
///
/// // Parse file to semantic model
/// let diagram = builder.parse(&arena, Path::new("app.orr"))
///     .expect("Failed to parse");
///
/// // Render semantic model to SVG
/// let svg = builder.render_svg(&diagram)
///     .expect("Failed to render");
/// ```
pub struct DiagramBuilder<'a, P: SourceProvider> {
    config: AppConfig,
    provider: &'a P,
}

impl<'a, P: SourceProvider> DiagramBuilder<'a, P> {
    /// Create a new diagram builder with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration including layout and style settings
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use orrery::{DiagramBuilder, InMemorySourceProvider, config::AppConfig};
    /// let provider = InMemorySourceProvider::new();
    /// let config = AppConfig::default();
    /// let builder = DiagramBuilder::new(config, &provider);
    /// ```
    pub fn new(config: AppConfig, provider: &'a P) -> Self {
        Self { config, provider }
    }

    /// Parse an Orrery file into a semantic diagram.
    ///
    /// Uses the [`SourceProvider`] held by this builder to resolve the root file
    /// and all its imports, then runs the full pipeline: resolve (tokenize →
    /// parse per file) → desugar → validate → elaborate.
    ///
    /// # Arguments
    ///
    /// * `root_path` — Path to the root/entry Orrery file
    ///
    /// # Errors
    ///
    /// Returns `RenderError` for syntax errors, validation errors, or
    /// elaboration errors.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::path::Path;
    /// # use bumpalo::Bump;
    /// # use orrery::{DiagramBuilder, InMemorySourceProvider, config::AppConfig};
    /// let arena = Bump::new();
    /// let mut provider = InMemorySourceProvider::new();
    /// provider.add_file("app.orr", "diagram component; app: Rectangle;");
    ///
    /// let builder = DiagramBuilder::new(AppConfig::default(), &provider);
    /// let diagram = builder.parse(&arena, Path::new("app.orr"))
    ///     .expect("Failed to parse diagram");
    /// ```
    pub fn parse<'b>(
        &self,
        arena: &'b Bump,
        root_path: &Path,
    ) -> Result<semantic::Diagram, ParseError<'b>> {
        info!("Parsing diagram");
        let elaborate_config = ElaborateConfig::new(
            self.config.layout().component(),
            self.config.layout().sequence(),
        );

        let diagram = orrery_parser::parse(arena, root_path, self.provider, elaborate_config)?;

        debug!("Diagram parsed successfully");
        trace!(diagram:?; "Parsed diagram");

        Ok(diagram)
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
    /// Returns `RenderError` for layout or rendering errors.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use std::path::Path;
    /// # use bumpalo::Bump;
    /// # use orrery::{DiagramBuilder, InMemorySourceProvider, config::AppConfig};
    /// let arena = Bump::new();
    /// let mut provider = InMemorySourceProvider::new();
    /// provider.add_file("app.orr", "diagram component; app: Rectangle;");
    ///
    /// let builder = DiagramBuilder::new(AppConfig::default(), &provider);
    ///
    /// let diagram = builder.parse(&arena, Path::new("app.orr"))
    ///     .expect("Failed to parse");
    ///
    /// let svg = builder.render_svg(&diagram)
    ///     .expect("Failed to render diagram");
    ///
    /// println!("{}", svg);
    /// ```
    pub fn render_svg(&self, diagram: &semantic::Diagram) -> Result<String, RenderError> {
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
            .with_event_padding(15.0);

        // Calculate layout
        info!("Processing diagrams in hierarchy");
        let layered_layout = engine_builder.build(&diagram_hierarchy)?;
        info!(layers_count = layered_layout.len(); "Layout calculated");

        // Render to SVG using a temporary file
        // TODO: In the future, modify SvgBuilder to support in-memory rendering
        let temp_file =
            tempfile::NamedTempFile::new().map_err(|err| RenderError::Export(Box::new(err)))?;
        let temp_path = temp_file.path().to_string_lossy().to_string();

        let mut svg_exporter = export::svg::SvgBuilder::new(&temp_path)
            .with_style(self.config.style())
            .with_diagram(diagram)
            .build()?;

        svg_exporter.export_layered_layout(&layered_layout)?;

        // Read the SVG content back from the temp file
        let svg_string = fs::read_to_string(&temp_path).map_err(RenderError::Io)?;

        info!("SVG rendered successfully");
        Ok(svg_string)
    }
}
