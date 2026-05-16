//! Reusable algorithms for calculating element positions in diagrams.
//!
//! This module provides algorithms for calculating element positions in diagrams.
//! It contains reusable positioning logic that can be used by different layout engines.

use orrery_core::geometry::{Bounds, Size};

/// A trait for types that can calculate their own size and bounds
pub trait LayoutBounds {
    /// Calculate the size of this layout, possibly adding padding
    fn layout_size(&self) -> Size {
        self.layout_bounds().to_size()
    }

    /// Calculate the bounds of this layout's content
    /// Returns the bounding box that contains all content, which may have
    /// a non-zero minimum point if content doesn't start at the origin.
    fn layout_bounds(&self) -> Bounds;
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
