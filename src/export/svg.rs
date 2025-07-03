mod component;
mod layer;
mod sequence;

use crate::{
    ast,
    color::Color,
    config::StyleConfig,
    draw::ArrowWithTextDrawer,
    error::FilamentError,
    export,
    geometry::{Insets, Size},
    layout::layer::LayeredLayout,
};
use log::{debug, error, info};
use std::{fs::File, io::Write};
use svg::{Document, node::element::Rectangle};

/// SVG exporter builder to configure and build the SVG exporter
pub struct SvgBuilder<'a> {
    file_name: String,
    style: Option<&'a StyleConfig>,
    diagram: Option<&'a ast::Diagram>,
}

/// Base SVG exporter structure with common properties and methods
pub struct Svg {
    file_name: String,
    background_color: Option<Color>,
    arrow_with_text_drawer: ArrowWithTextDrawer, // NOTE: Does it need to be in this level or should it be in the SvgBuilder level?
}

impl<'a> SvgBuilder<'a> {
    pub fn new(file_name: &str) -> Self {
        Self {
            file_name: file_name.to_string(),
            style: None,
            diagram: None,
        }
    }

    /// Set style configuration
    pub fn with_style(mut self, style: &'a StyleConfig) -> Self {
        self.style = Some(style);
        self
    }

    /// Set diagram to extract styles from
    pub fn with_diagram(mut self, diagram: &'a ast::Diagram) -> Self {
        self.diagram = Some(diagram);
        self
    }

    /// Build the SVG exporter with the configured options
    pub fn build(self) -> Result<Svg, FilamentError> {
        let mut background_color = None;

        if let Some(diagram) = self.diagram {
            if let Some(color) = diagram.background_color {
                background_color = Some(color);
            }
        } else if let Some(style) = self.style {
            background_color = style.background_color()?;
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
    /// Calculate the optimal size for the SVG based on content dimensions
    /// Adds a small margin around the content
    pub fn calculate_svg_dimensions(&self, content_size: &Size) -> Size {
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

    /// Add background color to an SVG document if specified
    pub fn add_background(&self, mut doc: Document, size: Size) -> Document {
        // Add background if specified in the SVG exporter
        if let Some(bg_color) = &self.background_color {
            let bg = Rectangle::new()
                .set("x", 0)
                .set("y", 0)
                .set("width", size.width())
                .set("height", size.height())
                .set("fill", bg_color.to_string());
            doc = doc.add(bg);
        }

        doc
    }

    /// Writes an SVG document to the specified file
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
