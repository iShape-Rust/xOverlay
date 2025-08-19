use crate::core::direction::ContourDirection;

/// Configuration options for polygon Boolean operations using [`Overlay`].
///
/// These options control precision, simplification, and contour filtering
/// during the Boolean operation process. You can use this to adjust output
/// direction, eliminate small artifacts, or retain collinear points.
#[derive(Debug, Clone, Copy)]
pub struct IntOverlayOptions {
    /// Preserve collinear points in the input before Boolean operations.
    pub preserve_input_collinear: bool,

    /// Desired direction for output contours (default outer: CCW / hole: CW).
    pub output_direction: ContourDirection,

    /// Preserve collinear points in the output after Boolean operations.
    pub preserve_output_collinear: bool,

    /// Minimum area threshold to include a contour in the result.
    pub min_output_area: u64,
}

impl Default for IntOverlayOptions {
    fn default() -> Self {
        Self {
            preserve_input_collinear: false,
            output_direction: ContourDirection::CounterClockwise,
            preserve_output_collinear: false,
            min_output_area: 0,
        }
    }
}

impl IntOverlayOptions {
    pub fn keep_all_points() -> Self {
        Self {
            preserve_input_collinear: true,
            output_direction: ContourDirection::CounterClockwise,
            preserve_output_collinear: true,
            min_output_area: 0,
        }
    }
    pub fn keep_output_points() -> Self {
        Self {
            preserve_input_collinear: false,
            output_direction: ContourDirection::CounterClockwise,
            preserve_output_collinear: true,
            min_output_area: 0,
        }
    }
}