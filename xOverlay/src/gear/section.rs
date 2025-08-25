use crate::gear::x_mapper::XPart;
use alloc::vec::Vec;
use i_float::int::rect::IntRect;
use crate::gear::x_layout::XLayout;
use crate::gear::source::GeometrySource;

pub(crate) struct Section {
    pub(crate) source: GeometrySource,
    pub(crate) border_points: Vec<i32>,
    pub(crate) layout: XLayout,
}

impl Section {
    #[inline(always)]
    pub(crate) fn new(
        rect: IntRect,
        part: XPart,
        avg_count_per_column: usize,
        max_parts_count: usize,
    ) -> Self {
        let items_count = part.vr + part.hz + part.dg_pos + part.dg_neg;
        let source = GeometrySource {
            vr_list: Vec::with_capacity(part.vr),
            hz_list: Vec::with_capacity(part.hz),
            dg_pos_list: Vec::with_capacity(part.dg_pos),
            dg_neg_list: Vec::with_capacity(part.dg_neg),
        };

        Self {
            source,
            border_points: Vec::with_capacity(part.border),
            layout: XLayout::with_rect(rect, items_count, avg_count_per_column, max_parts_count),
        }
    }
}
