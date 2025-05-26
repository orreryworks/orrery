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
        for layer in &layout.layers {
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
        for layer in &layout.layers {
            main_group = main_group.add(self.render_layer(layer));
        }

        // Add the main group to the document
        doc.add(main_group)
    }

    /// Creates SVG clip path for a layer
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

    /// Calculate the combined bounds of all layers
    fn calculate_layered_layout_bounds(&self, layout: &LayeredLayout) -> Bounds {
        if layout.is_empty() {
            return Bounds::default();
        }

        // Start with the bounds of the first layer
        let mut combined_bounds = match &layout.layers[0].content {
            LayerContent::Component(comp_layout) => {
                self.calculate_component_diagram_bounds(comp_layout)
            }
            LayerContent::Sequence(seq_layout) => {
                self.calculate_sequence_diagram_bounds(seq_layout)
            }
        };

        // Merge with bounds of additional layers, adjusting for layer offset
        for layer in &layout.layers[1..] {
            let layer_bounds = match &layer.content {
                LayerContent::Component(comp_layout) => {
                    self.calculate_component_diagram_bounds(comp_layout)
                }
                LayerContent::Sequence(seq_layout) => {
                    self.calculate_sequence_diagram_bounds(seq_layout)
                }
            };

            // Adjust bounds for layer offset
            let offset_bounds = Bounds {
                min_x: layer_bounds.min_x + layer.offset.x,
                min_y: layer_bounds.min_y + layer.offset.y,
                max_x: layer_bounds.max_x + layer.offset.x,
                max_y: layer_bounds.max_y + layer.offset.y,
            };

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

        for layer in &layout.layers {
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

        // Apply clipping if specified
        if let Some(_bounds) = &layer.clip_bounds {
            // Create a unique clip ID for this layer
            let clip_id = format!("clip-layer-{}", layer.z_index);
            layer_group = layer_group.set("clip-path", format!("url(#{})", clip_id));
        }

        // Render the layer content
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
