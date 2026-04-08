//! SVG export backend for Orrery diagrams.
//!
//! This module provides `SvgBuilder` for configuring and `Svg` for rendering
//! laid-out diagrams to SVG files. It delegates to submodules for
//! diagram-kind-specific rendering.

mod component;
mod layer;
mod sequence;

use std::{fs::File, io::Write};

use log::{debug, error, info};
use svg::{Document, node::element::Rectangle};

use orrery_core::{
    color::Color,
    draw::ArrowWithTextDrawer,
    geometry::{Insets, Size},
    semantic,
};

use crate::{config::StyleConfig, error::RenderError, export, layout::layer::LayeredLayout};

/// SVG exporter builder to configure and build the SVG exporter.
pub struct SvgBuilder<'a> {
    file_name: String,
    style: Option<&'a StyleConfig>,
    diagram: Option<&'a semantic::Diagram>,
}

/// Base SVG exporter structure with common properties and methods.
pub struct Svg {
    file_name: String,
    background_color: Option<Color>,
    arrow_with_text_drawer: ArrowWithTextDrawer, // NOTE: Does it need to be in this level or should it be in the SvgBuilder level?
}

impl<'a> SvgBuilder<'a> {
    /// Creates a new builder with the given output file name.
    ///
    /// # Arguments
    ///
    /// * `file_name` - The destination file path for the SVG output.
    pub fn new(file_name: &str) -> Self {
        Self {
            file_name: file_name.to_string(),
            style: None,
            diagram: None,
        }
    }

    /// Sets the style configuration.
    ///
    /// # Arguments
    ///
    /// * `style` - The style configuration to apply.
    pub fn with_style(mut self, style: &'a StyleConfig) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the diagram to extract styles from.
    ///
    /// # Arguments
    ///
    /// * `diagram` - The semantic diagram.
    pub fn with_diagram(mut self, diagram: &'a semantic::Diagram) -> Self {
        self.diagram = Some(diagram);
        self
    }

    /// Builds the SVG exporter with the configured options.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if style configuration parsing fails.
    pub fn build(self) -> Result<Svg, RenderError> {
        let mut background_color = None;

        if let Some(diagram) = self.diagram {
            if let Some(color) = diagram.background_color() {
                background_color = Some(color);
            }
        } else if let Some(style) = self.style {
            background_color = style.background_color().map_err(RenderError::Layout)?;
        }

        let arrow_with_text_drawer = ArrowWithTextDrawer::new();

        Ok(Svg {
            file_name: self.file_name,
            background_color,
            arrow_with_text_drawer,
        })
    }
}

impl Svg {
    /// Calculates the optimal size for the SVG based on content dimensions.
    ///
    /// Adds a small margin around the content.
    ///
    /// # Arguments
    ///
    /// * `content_size` - The bounding size of the rendered diagram content.
    ///
    /// # Returns
    ///
    /// A [`Size`] that includes uniform margin padding around the content.
    pub fn calculate_svg_dimensions(&self, content_size: Size) -> Size {
        // Add some margin to the content size
        let margin: f32 = 50.0;
        let svg_size = content_size.add_padding(Insets::uniform(margin));

        debug!(
            "Final SVG dimensions: {}x{}",
            svg_size.width(),
            svg_size.height()
        );

        svg_size
    }

    /// Adds a background color rectangle to an SVG document if one is configured.
    ///
    /// # Arguments
    ///
    /// * `doc` - The SVG document to add the background to.
    /// * `size` - The dimensions of the background rectangle.
    ///
    /// # Returns
    ///
    /// The document, with a background rectangle prepended when a color is set.
    pub fn add_background(&self, mut doc: Document, size: Size) -> Document {
        // Add background if specified in the SVG exporter
        if let Some(bg_color) = &self.background_color {
            let bg = Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", size.width())
                .set("height", size.height())
                .set("fill", bg_color.to_string())
                .set("fill-opacity", bg_color.alpha());
            doc = doc.add(bg);
        }

        doc
    }

    /// Writes an SVG document to the configured output file.
    ///
    /// # Arguments
    ///
    /// * `doc` - The completed SVG document to persist.
    ///
    /// # Errors
    ///
    /// Returns [`export::Error::Io`] if file creation or writing fails.
    pub fn write_document(&self, doc: Document) -> Result<(), export::Error> {
        info!(file_name = self.file_name; "Creating SVG file");
        // Create the output file
        let f = match File::create(&self.file_name) {
            Ok(file) => file,
            Err(err) => {
                error!(file_name=self.file_name, err:err; "Failed to create SVG file");
                return Err(export::Error::Io(err));
            }
        };

        // Write the SVG content to the file
        if let Err(err) = write!(&f, "{doc}") {
            error!(file_name=self.file_name, err:err; "Failed to write SVG content");
            return Err(export::Error::Io(err));
        }

        Ok(())
    }
}

// Implementation of Exporter trait for SVG
impl export::Exporter for Svg {
    fn export_layered_layout(&mut self, layout: &LayeredLayout) -> Result<(), export::Error> {
        let doc = self.render_layered_layout(layout);
        debug!("SVG document rendered for layered layout");

        self.write_document(doc)
    }
}
