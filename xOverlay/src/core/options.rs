use crate::core::direction::ContourDirection;
pub use crate::deg_90::config::ColumnConfig90;

/// Configuration options for polygon Boolean operations using [`Overlay`].
///
/// These options control precision, simplification, and contour filtering
/// during the Boolean operation process. You can use this to adjust output
/// direction, eliminate small artifacts, or retain collinear points.
#[derive(Debug, Clone, Copy)]
pub struct IntOverlayOptions {
    /// Desired direction for output contours (default outer: CCW / hole: CW).
    pub output_direction: ContourDirection,

    /// Preserve collinear points in the output after Boolean operations.
    pub preserve_output_collinear: bool,

    pub columns_config: ColumnConfig90,
}

impl Default for IntOverlayOptions {
    fn default() -> Self {
        Self {
            output_direction: ContourDirection::CounterClockwise,
            preserve_output_collinear: false,
            columns_config: Default::default(),
        }
    }
}

impl IntOverlayOptions {
    pub fn keep_all_points() -> Self {
        Self {
            preserve_output_collinear: true,
            ..Self::default()
        }
    }
    pub fn keep_output_points() -> Self {
        Self {
            preserve_output_collinear: true,
            ..Self::default()
        }
    }
}
