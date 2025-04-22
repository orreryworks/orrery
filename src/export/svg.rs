use crate::{
    export,
    layout::common::{Point, Size},
};
use log::{debug, error, info};
use std::{fs::File, io::Write};
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
        info!(file_name = self.file_name; "Creating SVG file");
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
        let doc = self.render_component_diagram(layout);
        debug!("SVG document rendered");

        self.write_document(doc)
    }

    fn export_sequence_layout(
        &self,
        layout: &crate::layout::sequence::Layout,
    ) -> Result<(), export::Error> {
        let doc = self.render_sequence_diagram(layout);
        debug!("SVG document rendered");

        self.write_document(doc)
    }
}
