use crate::layout::common::{Point, Size};
use crate::{color::Color, export};
use log::{debug, error};
use std::fs::File;
use std::io::Write;
use svg::Document;

/// Base SVG exporter structure with common properties and methods
pub struct Svg {
    pub file_name: String,
}

impl Svg {
    pub fn new(file_name: &str) -> Self {
        Self {
            file_name: file_name.to_string(),
        }
    }

    /// Create a path data string from two points
    pub fn create_path_data_from_points(&self, start: &Point, end: &Point) -> String {
        format!("M {} {} L {} {}", start.x, start.y, end.x, end.y)
    }

    /// Calculate the optimal size for the SVG based on content dimensions
    /// Adds a small margin around the content
    pub fn calculate_svg_dimensions(&self, content_size: &Size) -> Size {
        // Add some margin to the content size
        let margin = 50.0;
        let width = content_size.width + margin * 2.0;
        let height = content_size.height + margin * 2.0;

        debug!("Final SVG dimensions: {}x{}", width, height);

        Size { width, height }
    }

    /// Writes an SVG document to the specified file
    pub fn write_document(&self, doc: Document) -> Result<(), export::Error> {
        // Create the output file
        let f = match File::create(&self.file_name) {
            Ok(file) => file,
            Err(err) => {
                error!("Failed to create SVG file {}: {}", self.file_name, err);
                return Err(export::Error::Io(err));
            }
        };

        // Write the SVG content to the file
        if let Err(err) = write!(&f, "{}", doc) {
            error!("Failed to write SVG content to {}: {}", self.file_name, err);
            return Err(export::Error::Io(err));
        }

        Ok(())
    }
}

mod component;
mod renderer;
mod sequence;

// Single implementation of Exporter trait that delegates to specialized methods
impl export::Exporter for Svg {
    fn export_component_layout(
        &self,
        layout: &crate::layout::component::Layout,
    ) -> Result<(), export::Error> {
        self.export_component_layout(layout)
    }

    fn export_sequence_layout(
        &self,
        layout: &crate::layout::sequence::Layout,
    ) -> Result<(), export::Error> {
        self.export_sequence_layout(layout)
    }
}
