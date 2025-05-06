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

    /// Create a path data string for the given arrow style
    pub fn create_path_data_for_style(&self, start: &Point, end: &Point, style: &crate::ast::ArrowStyle) -> String {
        match style {
            crate::ast::ArrowStyle::Straight => self.create_path_data_from_points(start, end),
            crate::ast::ArrowStyle::Curved => self.create_curved_path_data_from_points(start, end),
            crate::ast::ArrowStyle::Orthogonal => self.create_orthogonal_path_data_from_points(start, end),
        }
    }

    /// Create a path data string from two points
    pub fn create_path_data_from_points(&self, start: &Point, end: &Point) -> String {
        format!("M {} {} L {} {}", start.x, start.y, end.x, end.y)
    }

    /// Create a curved path data string from two points
    /// Creates a cubic bezier curve with control points positioned to create a nice arc
    pub fn create_curved_path_data_from_points(&self, start: &Point, end: &Point) -> String {
        // For the control points, we'll use points positioned to create a smooth arc
        // between the start and end points
        let ctrl1_x = start.x + (end.x - start.x) / 4.0;
        let ctrl1_y = start.y - (end.y - start.y) / 2.0;
        
        let ctrl2_x = end.x - (end.x - start.x) / 4.0;
        let ctrl2_y = end.y + (start.y - end.y) / 2.0;
        
        format!(
            "M {} {} C {} {}, {} {}, {} {}",
            start.x, start.y,
            ctrl1_x, ctrl1_y,
            ctrl2_x, ctrl2_y,
            end.x, end.y
        )
    }
    
    /// Create an orthogonal path data string from two points
    /// Creates a path with only horizontal and vertical line segments
    pub fn create_orthogonal_path_data_from_points(&self, start: &Point, end: &Point) -> String {
        // Determine whether to go horizontal first then vertical, or vertical first then horizontal
        // This decision is based on the relative positions of the start and end points
        
        // Calculate absolute differences in x and y directions
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        
        // If we're more horizontal than vertical, go horizontal first
        if dx > dy {
            // Go horizontal first (50% of the way), then vertical, then horizontal again
            let mid_x1 = start.x + (end.x - start.x) * 0.5;
            
            format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start.x, start.y,
                mid_x1, start.y,
                mid_x1, end.y,
                end.x, end.y
            )
        } else {
            // Go vertical first (50% of the way), then horizontal, then vertical again
            let mid_y1 = start.y + (end.y - start.y) * 0.5;
            
            format!(
                "M {} {} L {} {} L {} {} L {} {}",
                start.x, start.y,
                start.x, mid_y1,
                end.x, mid_y1,
                end.x, end.y
            )
        }
    }

    /// Calculate the optimal size for the SVG based on content dimensions
    /// Adds a small margin around the content
    pub fn calculate_svg_dimensions(&self, content_size: &Size) -> Size {
        // Add some margin to the content size
        let margin: f32 = 50.0;
        let width = margin.mul_add(2.0, content_size.width); // content_size.width + margin * 2.0;
        let height = margin.mul_add(2.0, content_size.height); // content_size.height + margin * 2.0;

        debug!("Final SVG dimensions: {width}x{height}");

        Size { width, height }
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
