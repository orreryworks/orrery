//! SVG rendering for layout layers.

use log::debug;
use svg::{self, node::element as svg_element};

use filament_core::{draw::LayeredOutput, geometry::Bounds};

use super::Svg;
use crate::layout::{
    component,
    layer::{ContentStack, Layer, LayeredLayout, LayoutContent},
    positioning::LayoutBounds,
    sequence,
};

impl Svg {
    /// Renders the complete layered layout to an SVG document.
    pub fn render_layered_layout(&mut self, layout: &LayeredLayout) -> svg::Document {
        // Calculate content bounds
        let content_bounds = self.calculate_layered_layout_bounds(layout);
        let content_size = content_bounds.to_size();

        // Calculate final SVG dimensions with margins
        let svg_size = self.calculate_svg_dimensions(&content_size);

        // Create the SVG document with calculated dimensions
        let doc = svg::Document::new()
            .set(
                "viewBox",
                format!("0 0 {} {}", svg_size.width(), svg_size.height()),
            )
            .set("width", svg_size.width())
            .set("height", svg_size.height());

        // Add background
        let mut doc = self.add_background(doc, svg_size);

        // Add clip paths for all layers that need clipping
        // Each clip path gets a unique ID based on the layer's z-index
        for layer in layout.iter_from_bottom() {
            if let Some(bounds) = layer.clip_bounds() {
                let clip_id = format!("clip-layer-{}", layer.z_index());
                let clip_path = self.create_clip_path(&clip_id, bounds);
                doc = doc.add(clip_path);
            }
        }

        // Calculate margins for centering
        let margin_x = (svg_size.width() - content_size.width()) / 2.0;
        let margin_y = (svg_size.height() - content_size.height()) / 2.0;

        // Create a main group with translation to center content and adjust for min bounds
        let mut main_group = svg_element::Group::new().set(
            "transform",
            format!(
                "translate({}, {})",
                margin_x - content_bounds.min_x(),
                margin_y - content_bounds.min_y()
            ),
        );

        // Add each layer in order
        for layer in layout.iter_from_bottom() {
            main_group = main_group.add(self.render_layer(layer));
        }

        // Add marker definitions for all layers
        let arrow_markers_defs = self.arrow_with_text_drawer.draw_marker_definitions();
        doc = doc.add(arrow_markers_defs);

        // Add the main group to the document
        doc.add(main_group)
    }

    /// Creates an SVG clip path for a layer.
    ///
    /// This generates an SVG Definitions element containing a ClipPath with the specified ID.
    /// The clip path contains a rectangle that matches the provided bounds.
    ///
    /// # Arguments
    ///
    /// * `clip_id` - Unique identifier for the clip path.
    /// * `bounds` - The bounds to use for clipping.
    fn create_clip_path(&self, clip_id: &str, bounds: Bounds) -> svg_element::Definitions {
        let defs = svg_element::Definitions::new();

        // Create a clip path with a rectangle matching the bounds
        let clip_rect = svg_element::Rectangle::new()
            .set("x", bounds.min_x())
            .set("y", bounds.min_y())
            .set("width", bounds.width())
            .set("height", bounds.height());

        let clip_path = svg_element::ClipPath::new()
            .set("id", clip_id)
            .add(clip_rect);

        defs.add(clip_path)
    }

    /// Calculates the combined bounds of all layers, considering their offsets.
    ///
    /// Computes the total bounding box that contains all layers in the layout,
    /// accounting for layer offsets. This is used to determine the overall size needed
    /// for the SVG document.
    ///
    /// # Returns
    ///
    /// A `Bounds` object that encompasses all content in all layers.
    fn calculate_layered_layout_bounds(&self, layout: &LayeredLayout) -> Bounds {
        let mut layout_iter = layout.iter_from_bottom();
        // Start with the bounds of the first (bottom) layer.
        // Note: `iter_from_bottom().next()` returning `None` is equivalent to `is_empty()`..
        let Some(first_layer) = layout_iter.next() else {
            return Bounds::default();
        };
        let mut combined_bounds = self.calculate_layer_bounds(first_layer);

        // Merge with bounds of additional layers, adjusting for layer offset
        for layer in layout_iter {
            let layer_bounds = self.calculate_layer_bounds(layer);

            // Adjust bounds for layer offset by creating a translated copy
            let offset_bounds = layer_bounds.translate(layer.offset());

            // Merge with the combined bounds to include this layer
            combined_bounds = combined_bounds.merge(&offset_bounds);
        }

        combined_bounds
    }

    /// Calculates bounds for a single layer.
    fn calculate_layer_bounds(&self, layer: &Layer) -> Bounds {
        match layer.content() {
            LayoutContent::Component(comp_layout) => {
                self.calculate_component_diagram_bounds(comp_layout)
            }
            LayoutContent::Sequence(seq_layout) => {
                self.calculate_sequence_diagram_bounds(seq_layout)
            }
        }
    }

    /// Renders a single layer to SVG.
    ///
    /// Creates an SVG group for the layer, applies transformations and clipping,
    /// and renders the layer's content (either component or sequence diagram).
    ///
    /// # Arguments
    ///
    /// * `layer` - The layer to render.
    ///
    /// # Returns
    ///
    /// An SVG `Group` element containing the rendered layer.
    fn render_layer(&mut self, layer: &Layer) -> svg_element::Group {
        // Create a group for this layer
        let mut layer_group = svg_element::Group::new();

        let offset = layer.offset();
        // Apply offset transformation if not at origin
        if !offset.is_zero() {
            layer_group = layer_group.set(
                "transform",
                format!("translate({}, {})", offset.x(), offset.y()),
            );
        }

        // Apply clipping if specified for this layer
        if let Some(_bounds) = layer.clip_bounds() {
            // Create a unique clip ID for this layer based on its z-index
            let clip_id = format!("clip-layer-{}", layer.z_index());
            // Apply the clip-path property referencing the previously defined clip path
            layer_group = layer_group.set("clip-path", format!("url(#{clip_id})"));
        }

        // Render the layer content based on its type
        self.render_layer_content(layer.content())
            .into_iter()
            .fold(layer_group, |group, content_group| group.add(content_group))
    }

    /// Renders layer content by dispatching to the appropriate content-specific renderer.
    fn render_layer_content(&mut self, content: &LayoutContent) -> Vec<Box<dyn svg::Node>> {
        match content {
            LayoutContent::Component(layout) => self
                .render_content_stack(layout, |svg, content| svg.render_component_content(content)),
            LayoutContent::Sequence(layout) => self
                .render_content_stack(layout, |svg, content| svg.render_sequence_content(content)),
        }
    }

    /// Renders a [`ContentStack`] with positioned content.
    fn render_content_stack<T: LayoutBounds>(
        &mut self,
        content_stack: &ContentStack<T>,
        render_fn: impl Fn(&mut Self, &T) -> Vec<Box<dyn svg::Node>>,
    ) -> Vec<Box<dyn svg::Node>> {
        let mut groups = Vec::with_capacity(content_stack.len());
        // Render all positioned content in the stack (reverse order for proper layering)
        for positioned_content in content_stack.iter().rev() {
            let offset = positioned_content.offset();
            let content = positioned_content.content();
            debug!(offset:?; "Rendering positioned content");

            // Create a group for this positioned content with its offset applied
            let mut positioned_group = svg_element::Group::new();

            // Apply the positioned content's offset as a transform
            if !positioned_content.offset().is_zero() {
                positioned_group = positioned_group.set(
                    "transform",
                    format!("translate({}, {})", offset.x(), offset.y()),
                );
            }

            // Use the provided render function to render the content
            positioned_group = render_fn(self, content)
                .into_iter()
                .fold(positioned_group, |group, content_group| {
                    group.add(content_group)
                });

            // Add the positioned group to the layer
            groups.push(positioned_group.into());
        }
        groups
    }

    /// Renders component-specific content.
    fn render_component_content(&mut self, content: &component::Layout) -> Vec<Box<dyn svg::Node>> {
        let mut output = LayeredOutput::new();

        // Render all components within this positioned content
        for component in content.components() {
            let component_output = self.render_component(component);
            output.merge(component_output);
        }

        // Render all relations within this positioned content
        for relation in content.relations() {
            let relation_output = self.render_relation(
                content.source(relation),
                content.target(relation),
                relation.arrow_with_text(),
            );
            output.merge(relation_output);
        }

        output.render()
    }

    /// Renders sequence-specific content.
    fn render_sequence_content(&mut self, content: &sequence::Layout) -> Vec<Box<dyn svg::Node>> {
        let mut output = LayeredOutput::new();

        // Render all participants within this positioned content
        for participant in content.participants().values() {
            let participant_output = self.render_participant(participant);
            output.merge(participant_output);
        }

        // Render all fragments within this positioned content
        for fragment in content.fragments() {
            let fragment_output = self.render_fragment(fragment);
            output.merge(fragment_output);
        }

        // Render all activation boxes within this positioned content
        // Sort by nesting level to ensure proper z-order (lower levels render first, higher levels on top)
        let mut sorted_activations: Vec<_> = content.activations().iter().collect();
        sorted_activations.sort_by_key(|activation_box| activation_box.drawable().nesting_level());

        for activation_box in sorted_activations {
            let activation_output = self.render_activation_box(activation_box, content);
            output.merge(activation_output);
        }

        // Render all notes within this positioned content
        for note in content.notes() {
            let note_output = self.render_note(note);
            output.merge(note_output);
        }

        // Render all messages within this positioned content
        for message in content.messages() {
            let message_output = self.render_message(message, content);
            output.merge(message_output);
        }

        output.render()
    }
}
