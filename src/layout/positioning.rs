//! Layout positioning algorithms
//!
//! This module provides algorithms for calculating element positions in diagrams.
//! It contains reusable positioning logic that can be used by different layout engines.

use crate::{
    ast,
    layout::{geometry::Size, text},
};
use std::iter::IntoIterator;

/// Calculate additional spacing needed based on text labels
///
/// This function examines a collection of optional text labels and determines
/// the minimum spacing required to accommodate the widest label plus padding.
///
/// # Arguments
/// * `labels` - Collection of optional text label references
/// * `padding` - Additional padding to add around the calculated text width
///
/// # Returns
/// The width needed for the widest label plus padding, or 0 if no labels
pub fn calculate_label_spacing<'a, I>(labels: I, padding: f32) -> f32
where
    I: IntoIterator<Item = Option<&'a String>>,
{
    labels
        .into_iter()
        .flatten()
        .map(|label| text::calculate_text_size(label, 14).width() + padding)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

/// Distribute elements horizontally with appropriate spacing
///
/// Places elements in a horizontal row with specified minimum spacing between them,
/// with the option to provide custom spacing between specific pairs of elements.
///
/// # Arguments
/// * `sizes` - Sizes of the elements to distribute
/// * `min_spacing` - Minimum spacing between elements
/// * `extra_spacings` - Optional vector of custom spacings between elements
/// * `start_position` - Starting X position for the first element
///
/// # Returns
/// A vector of X positions for the centers of each element
pub fn distribute_horizontally(
    sizes: &[Size],
    min_spacing: f32,
    extra_spacings: Option<&[f32]>,
) -> Vec<f32> {
    let mut positions = Vec::with_capacity(sizes.len());
    let mut x_position: f32 = 0.0;

    for (i, size) in sizes.iter().enumerate() {
        // For the first element, we start at the given position
        if i == 0 {
            x_position += size.width() / 2.0;
        } else {
            // For subsequent elements, we position based on previous element and spacing
            let prev_width = sizes[i - 1].width();

            // Get any extra spacing from the provided array, or use 0.0
            let additional_spacing = extra_spacings
                .and_then(|spacings| spacings.get(i - 1).copied())
                .unwrap_or(0.0);

            // Use the larger of min_spacing or additional_spacing
            let effective_spacing = min_spacing.max(additional_spacing);

            // Move position by half of previous width + spacing + half of current width
            x_position += (prev_width / 2.0) + effective_spacing + (size.width() / 2.0);
        }

        positions.push(x_position);
    }

    positions
}

/// Calculate the size of a component or participant based on its text content
///
/// # Arguments
/// * `node` - The node whose size needs to be calculated
/// * `min_width` - Minimum width constraint for the element
/// * `min_height` - Minimum height constraint for the element
/// * `padding` - Padding to add around the text content
///
/// # Returns
/// The calculated size with applied constraints
/// TODO why do I need this anymore? Can I use shape instead?
pub fn calculate_bounded_text_size(
    node: &ast::Node,
    min_width: f32,
    min_height: f32,
    padding: f32,
) -> Size {
    // Calculate text size based on the node's display text and font size
    let size = text::calculate_text_size(node.display_text(), node.type_definition.font_size);

    // Add padding around the text and ensure minimum size
    size.add_padding(padding)
        .max(Size::new(min_width, min_height))
}
