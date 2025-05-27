use crate::layout::{
    common::Bounds,
    layer::{Layer, LayerContent, LayeredLayout},
};
use svg::{
    Document,
    node::element::{ClipPath, Definitions, Group, Rectangle},
};

use super::Svg;

impl Svg {
    /// Render the complete layered layout to an SVG document
    pub fn render_layered_layout(&self, layout: &LayeredLayout) -> Document {
        // Calculate content bounds
        let content_bounds = self.calculate_layered_layout_bounds(layout);
        let content_size = content_bounds.to_size();

        // Calculate final SVG dimensions with margins
        let svg_size = self.calculate_svg_dimensions(&content_size);

        // Create the SVG document with calculated dimensions
        let doc = Document::new()
            .set(
                "viewBox",
                format!("0 0 {} {}", svg_size.width, svg_size.height),
            )
            .set("width", svg_size.width)
            .set("height", svg_size.height);

        // Add background
        let mut doc = self.add_background(doc, svg_size.width, svg_size.height);

        // Add marker definitions for all layers
        let defs = self.create_marker_definitions_for_all_layers(layout);
        doc = doc.add(defs);

        // Add clip paths for all layers that need clipping
        // Each clip path gets a unique ID based on the layer's z-index
        for layer in layout.iter_from_bottom() {
            if let Some(bounds) = &layer.clip_bounds {
                let clip_id = format!("clip-layer-{}", layer.z_index);
                let clip_path = self.create_clip_path(&clip_id, bounds);
                doc = doc.add(clip_path);
            }
        }

        // Calculate margins for centering
        let margin_x = (svg_size.width - content_size.width) / 2.0;
        let margin_y = (svg_size.height - content_size.height) / 2.0;

        // Create a main group with translation to center content and adjust for min bounds
        let mut main_group = Group::new().set(
            "transform",
            format!(
                "translate({}, {})",
                margin_x - content_bounds.min_x,
                margin_y - content_bounds.min_y
            ),
        );

        // Add each layer in order
        for layer in layout.iter_from_bottom() {
            main_group = main_group.add(self.render_layer(layer));
        }

        // Add the main group to the document
        doc.add(main_group)
    }

    /// Creates SVG clip path for a layer
    ///
    /// This generates an SVG Definitions element containing a ClipPath with the specified ID.
    /// The clip path contains a rectangle that matches the provided bounds.
    ///
    /// # Parameters
    /// * `clip_id` - Unique identifier for the clip path
    /// * `bounds` - The bounds to use for clipping
    fn create_clip_path(&self, clip_id: &str, bounds: &Bounds) -> Definitions {
        let defs = Definitions::new();

        // Create a clip path with a rectangle matching the bounds
        let clip_rect = Rectangle::new()
            .set("x", bounds.min_x)
            .set("y", bounds.min_y)
            .set("width", bounds.width())
            .set("height", bounds.height());

        let clip_path = ClipPath::new().set("id", clip_id).add(clip_rect);

        defs.add(clip_path)
    }

    /// Calculate the combined bounds of all layers, considering their offsets
    ///
    /// This method computes the total bounding box that contains all layers in the layout,
    /// accounting for layer offsets. This is used to determine the overall size needed
    /// for the SVG document.
    ///
    /// # Returns
    /// A Bounds object that encompasses all content in all layers
    fn calculate_layered_layout_bounds(&self, layout: &LayeredLayout) -> Bounds {
        if layout.is_empty() {
            return Bounds::default();
        }

        let mut layout_iter = layout.iter_from_bottom();
        // Start with the bounds of the first (bottom) layer
        let mut combined_bounds = match &layout_iter
            .next()
            .expect("Bottom layer should always exist").content // FIXME: Convert to Result.
        {
            LayerContent::Component(comp_layout) => {
                self.calculate_component_diagram_bounds(comp_layout)
            }
            LayerContent::Sequence(seq_layout) => {
                self.calculate_sequence_diagram_bounds(seq_layout)
            }
        };

        // Merge with bounds of additional layers, adjusting for layer offset
        for layer in layout_iter {
            let layer_bounds = match &layer.content {
                LayerContent::Component(comp_layout) => {
                    self.calculate_component_diagram_bounds(comp_layout)
                }
                LayerContent::Sequence(seq_layout) => {
                    self.calculate_sequence_diagram_bounds(seq_layout)
                }
            };

            // Adjust bounds for layer offset by creating a translated copy
            let offset_bounds = layer_bounds.translate(layer.offset);

            // Merge with the combined bounds to include this layer
            combined_bounds = combined_bounds.merge(&offset_bounds);
        }

        combined_bounds
    }

    /// Create marker definitions for all layers
    fn create_marker_definitions_for_all_layers(
        &self,
        layout: &LayeredLayout,
    ) -> svg::node::element::Definitions {
        // Collect all unique colors used across all layers
        let mut all_colors = Vec::new();

        for layer in layout.iter_from_bottom() {
            match &layer.content {
                LayerContent::Component(comp_layout) => {
                    for relation in &comp_layout.relations {
                        all_colors.push(&relation.relation.color);
                    }
                }
                LayerContent::Sequence(seq_layout) => {
                    for message in &seq_layout.messages {
                        all_colors.push(&message.relation.color);
                    }
                }
            }
        }

        // Create marker definitions for all collected colors
        super::arrows::create_marker_definitions(all_colors.into_iter())
    }

    /// Render a single layer to SVG
    ///
    /// This method creates an SVG group for the layer, applies transformations and clipping,
    /// and renders the layer's content (either component or sequence diagram).
    ///
    /// # Parameters
    /// * `layer` - The layer to render
    ///
    /// # Returns
    /// An SVG Group element containing the rendered layer
    fn render_layer(&self, layer: &Layer) -> Group {
        // Create a group for this layer
        let mut layer_group = Group::new();

        // Apply offset transformation if not at origin
        if layer.offset.x != 0.0 || layer.offset.y != 0.0 {
            layer_group = layer_group.set(
                "transform",
                format!("translate({}, {})", layer.offset.x, layer.offset.y),
            );
        }

        // Apply clipping if specified for this layer
        if let Some(_bounds) = &layer.clip_bounds {
            // Create a unique clip ID for this layer based on its z-index
            let clip_id = format!("clip-layer-{}", layer.z_index);
            // Apply the clip-path property referencing the previously defined clip path
            layer_group = layer_group.set("clip-path", format!("url(#{})", clip_id));
        }

        // Render the layer content based on its type
        match &layer.content {
            LayerContent::Component(layout) => {
                // Render all components
                for component in &layout.components {
                    layer_group = layer_group.add(self.render_component(component));
                }

                // Render all relations
                for relation in &layout.relations {
                    layer_group = layer_group.add(self.render_relation(
                        layout.source(relation),
                        layout.target(relation),
                        relation.relation,
                    ));
                }
            }
            LayerContent::Sequence(layout) => {
                // Render all participants
                for participant in &layout.participants {
                    layer_group = layer_group.add(self.render_participant(participant));
                }

                // Render all messages
                for message in &layout.messages {
                    layer_group = layer_group.add(self.render_message(message, layout));
                }
            }
        }

        layer_group
    }
}
