use alloc::vec::Vec;
use i_float::int::rect::IntRect;
use crate::gear::s_mapper::SPart;
use crate::gear::x_layout::XLayout;
use crate::gear::source::GeometrySource;

#[derive(Clone)]
pub(crate) struct Section {
    pub(super) source: GeometrySource,
    pub(super) layout: XLayout,
}

impl Section {
    #[inline(always)]
    pub(super) fn new(
        rect: IntRect,
        part: SPart,
        avg_count_per_column: usize,
        max_parts_count: usize,
    ) -> Self {
        let items_count = part.count_vr + part.count_hz + part.count_dp + part.count_dn;
        let source = GeometrySource {
            vr_list: Vec::with_capacity(part.count_vr),
            hz_list: Vec::with_capacity(part.count_hz),
            dp_list: Vec::with_capacity(part.count_dp),
            dn_list: Vec::with_capacity(part.count_dn),
        };

        Self {
            source,
            layout: XLayout::with_rect(rect, items_count, avg_count_per_column, max_parts_count),
        }
    }
}
