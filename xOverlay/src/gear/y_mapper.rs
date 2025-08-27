use crate::gear::split_buffer::{SplitDn, SplitDp, SplitHz};
use crate::gear::y_layout::YLayout;
use crate::geom::range::LineRange;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;

pub(super) struct YMapper {
    pub(super) parts_layout: YLayout,
    pub(super) hz_parts_count: Vec<usize>,
    pub(super) dp_parts_count: Vec<usize>,
    pub(super) dn_parts_count: Vec<usize>,
    pub(super) hz_parts_start: Vec<usize>,
    pub(super) dp_parts_start: Vec<usize>,
    pub(super) dn_parts_start: Vec<usize>,
}

impl YMapper {
    #[inline]
    pub(super) fn new(layout: YLayout) -> Self {
        let n = layout.count();
        Self {
            parts_layout: layout,
            hz_parts_count: vec![0; n],
            dp_parts_count: vec![0; n],
            dn_parts_count: vec![0; n],

            hz_parts_start: vec![0; n],
            dp_parts_start: vec![0; n],
            dn_parts_start: vec![0; n],
        }
    }

    pub(super) fn map_hz(&mut self, slice: &[SplitHz]) {
        self.hz_parts_count.fill(0);

        // count space
        for hz in slice {
            let index = self.parts_layout.index(hz.y);
            unsafe { *self.hz_parts_count.get_unchecked_mut(index) += 1 };
        }

        Self::prepare_count_and_start(&mut self.hz_parts_count, &mut self.hz_parts_start);
    }

    pub(super) fn map_dp(&mut self, slice: &[SplitDp]) {
        self.dp_parts_count.fill(0);

        // count space
        for dp in slice {
            let index = self.parts_layout.index(dp.y_range.min);
            unsafe { *self.dp_parts_count.get_unchecked_mut(index) += 1 };
        }

        Self::prepare_count_and_start(&mut self.dp_parts_count, &mut self.dp_parts_start);
    }

    pub(super) fn map_dn_edges(&mut self, slice: &[SplitDn]) {
        self.dn_parts_count.fill(0);

        // count space
        for dn in slice {
            let index = self.parts_layout.index(dn.y_range.min);
            unsafe { *self.dn_parts_count.get_unchecked_mut(index) += 1 };
        }

        Self::prepare_count_and_start(&mut self.dn_parts_count, &mut self.dn_parts_start);
    }

    #[inline(always)]
    pub(super) fn next_hz_index(&mut self, y: i32) -> usize {
        let part = self.parts_layout.index(y);
        let (start, count) = unsafe {
            (*self.hz_parts_start.get_unchecked(part), self.hz_parts_count.get_unchecked_mut(part))
        };
        let result = start + *count;
        *count += 1;
        result
    }

    #[inline(always)]
    pub(super) fn next_dp_index(&mut self, y: i32) -> usize {
        let part = self.parts_layout.index(y);
        let (start, count) = unsafe {
            (*self.dp_parts_start.get_unchecked(part), self.dp_parts_count.get_unchecked_mut(part))
        };
        let result = start + *count;
        *count += 1;
        result
    }

    #[inline(always)]
    pub(super) fn next_dn_index(&mut self, y: i32) -> usize {
        let part = self.parts_layout.index(y);
        let (start, count) = unsafe {
            (*self.dn_parts_start.get_unchecked(part), self.dn_parts_count.get_unchecked_mut(part))
        };
        let result = start + *count;
        *count += 1;
        result
    }

    #[inline]
    fn prepare_count_and_start(count_buffer: &mut [usize], start_buffer: &mut [usize]) {
        let mut offset = 0;
        for (count, start) in count_buffer.iter_mut().zip(start_buffer.iter_mut()) {
            *start = offset;
            offset += *count;
            *count = 0;
        }
    }

    #[inline(always)]
    fn range_hz_for_indices(&self, parts: Range<usize>) -> Range<usize> {
        Self::range_for_indices(parts, &self.hz_parts_start, &self.hz_parts_count)
    }

    #[inline(always)]
    fn range_dp_for_indices(&self, parts: Range<usize>) -> Range<usize> {
        Self::range_for_indices(parts, &self.dp_parts_start, &self.dp_parts_count)
    }

    #[inline(always)]
    fn range_dn_for_indices(&self, parts: Range<usize>) -> Range<usize> {
        Self::range_for_indices(parts, &self.dn_parts_start, &self.dn_parts_count)
    }

    #[inline(always)]
    fn range_for_indices(parts: Range<usize>, start: &[usize], count: &[usize]) -> Range<usize> {
        unsafe {
            let &start_0 = start.get_unchecked(parts.start);
            let &start_1 = start.get_unchecked(parts.end);
            let &count_1 = count.get_unchecked(parts.end);

            start_0..start_1 + count_1
        }
    }

    #[inline(always)]
    pub(super) fn indices_by_range_hz(&self, y_range: LineRange) -> Range<usize> {
        let parts = self.parts_layout.indices_by_range(y_range);
        self.range_hz_for_indices(parts)
    }

    // dp

    #[inline(always)]
    pub(super) fn indices_by_range_bottom_offset_dp(&self, y_range: LineRange) -> Range<usize> {
        let parts = self.parts_layout.indices_by_range(y_range);
        self.range_dp_for_indices(parts)
    }

    #[inline(always)]
    pub(super) fn indices_bottom_offset_dp(&self, y: i32) -> Range<usize> {
        let parts = self.parts_layout.indices_bottom_offset(y);
        self.range_dp_for_indices(parts)
    }

    // dn

    #[inline(always)]
    pub(super) fn indices_by_range_bottom_offset_dn(&self, y_range: LineRange) -> Range<usize> {
        let parts = self.parts_layout.indices_by_range(y_range);
        self.range_dn_for_indices(parts)
    }

    #[inline(always)]
    pub(super) fn indices_bottom_offset_dn(&self, y: i32) -> Range<usize> {
        let parts = self.parts_layout.indices_bottom_offset(y);
        self.range_dn_for_indices(parts)
    }
}

#[cfg(test)]
mod tests {
    use crate::gear::x_layout::XLayout;
    use crate::gear::x_mapper::XMapper;
    use i_float::int::point::IntPoint;

    #[test]
    fn test_0() {
        let subj = [[
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]
        .to_vec()];

        let mut mapper = XMapper::new(XLayout::with_subj_and_clip(&subj, &[], 2).unwrap());

        mapper.add_contours(&subj);

        assert_eq!(mapper.hz_parts[0], 2);
        assert_eq!(mapper.vr_parts[0], 2);
    }
}
