use crate::core::cpu_count::CPUCount;
use crate::core::fill_rule::FillRule;
use crate::core::options::IntOverlayOptions;
use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column_map::{Column, ColumnMap};
use alloc::vec::Vec;
use i_shape::int::shape::{IntContour, IntShapes};

/// Input geometry prepared for an orthogonal Boolean operation.
pub struct Overlay {
    pub options: IntOverlayOptions,
    pub cpu_count: CPUCount,
    pub(crate) columns: Vec<Column>,
}

impl Overlay {
    #[inline]
    pub fn with_contours(subj: &[IntContour<i32>], clip: &[IntContour<i32>]) -> Self {
        Self::with_contours_custom(subj, clip, Default::default(), CPUCount::Auto)
    }

    #[inline]
    pub fn with_contours_custom(
        subj: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        options: IntOverlayOptions,
        cpu_count: CPUCount,
    ) -> Self {
        let map = ColumnMap::with_subj_and_clip(subj, clip, cpu_count, options.columns_config);

        Self {
            options,
            cpu_count,
            columns: map.columns,
        }
    }

    #[inline]
    pub fn overlay(self, fill_rule: FillRule, overlay_rule: OverlayRule) -> IntShapes<i32> {
        self.process_overlay(fill_rule, overlay_rule)
            .extract_shapes(overlay_rule)
    }
}
