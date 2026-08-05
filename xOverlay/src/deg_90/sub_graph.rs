use crate::core::fill_rule::FillRule;
use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column;
use crate::deg_90::column_map::Column;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_shape::int::shape::{IntContour, IntShapes};

pub(super) struct SubGraph {
    pub(super) range: LineRange,
    pub(super) contours: Vec<IntContour<i32>>,
}

impl SubGraph {
    pub(super) fn with_column(
        column: Column,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> Self {
        let range = column.range;
        let contours = column::extract_contours(&column, fill_rule, overlay_rule);

        Self { range, contours }
    }

    pub(super) fn into_shapes(self) -> IntShapes<i32> {
        column::rebuild_shapes(self.contours)
    }
}
