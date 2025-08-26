use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;
use crate::gear::split::IndexEdge;
use crate::gear::y_layout::YLayout;

pub(super) struct YPart {
    pub(super) hz: usize,
    pub(super) dg_pos: usize,
    pub(super) dg_neg: usize,
}

pub(super) struct YMapper {
    pub(super) layout: YLayout,
    pub(super) hz_parts_count: Vec<usize>,
    pub(super) dg_pos_parts_count: Vec<usize>,
    pub(super) dg_neg_parts_count: Vec<usize>,
    pub(super) hz_parts_start: Vec<usize>,
    pub(super) dg_pos_parts_start: Vec<usize>,
    pub(super) dg_neg_parts_start: Vec<usize>,
}

impl YMapper {
    #[inline]
    pub(super) fn new(layout: YLayout) -> Self {
        let n = layout.count();
        Self {
            layout,
            hz_parts_count: vec![0; n],
            dg_pos_parts_count: vec![0; n],
            dg_neg_parts_count: vec![0; n],
            hz_parts_start: vec![0; n],
            dg_pos_parts_start: vec![0; n],
            dg_neg_parts_start: vec![0; n],
        }
    }

    pub(super) fn map_hz_edges(&mut self, edges: &[IndexEdge]) {
        self.hz_parts_count.fill(0);
        Self::count_edges_by_pos(&self.layout, &mut self.hz_parts_count, edges);
        Self::prepare_count_and_start(&mut self.hz_parts_count, &mut self.hz_parts_start);
    }

    pub(super) fn map_dp_edges(&mut self, edges: &[IndexEdge]) {
        self.dg_pos_parts_count.fill(0);
        Self::count_edges_by_min(&self.layout, &mut self.dg_pos_parts_count, edges);
        Self::prepare_count_and_start(&mut self.dg_pos_parts_count, &mut self.dg_pos_parts_start);
    }

    pub(super) fn map_dn_edges(&mut self, edges: &[IndexEdge]) {
        self.dg_neg_parts_count.fill(0);
        Self::count_edges_by_min(&self.layout, &mut self.dg_neg_parts_count, edges);
        Self::prepare_count_and_start(&mut self.dg_neg_parts_count, &mut self.dg_neg_parts_start);
    }

    #[inline(always)]
    pub(super) fn next_hz_index(&mut self, y: i32) -> usize {
        let index = self.layout.bottom_index(y);
        let count = unsafe {
            self.hz_parts_count.get_unchecked_mut(index)
        };
        let result = *count;
        *count = result + 1;
        result
    }

    #[inline(always)]
    pub(super) fn next_dg_pos_index(&mut self, y: i32) -> usize {
        let index = self.layout.bottom_index(y);
        let count = unsafe {
            self.dg_pos_parts_count.get_unchecked_mut(index)
        };
        let result = *count;
        *count = result + 1;
        result
    }

    #[inline(always)]
    pub(super) fn next_dg_neg_index(&mut self, y: i32) -> usize {
        let index = self.layout.bottom_index(y);
        let count = unsafe {
            self.dg_neg_parts_count.get_unchecked_mut(index)
        };
        let result = *count;
        *count = result + 1;
        result
    }


    #[inline]
    fn count_edges_by_min(layout: &YLayout, count_buffer: &mut [usize], edges: &[IndexEdge]) {
        count_buffer.fill(0);
        for e in edges.iter() {
            let index = layout.bottom_index(e.range.min);
            unsafe {
                *count_buffer.get_unchecked_mut(index) += 1;
            }
        }
    }

    #[inline]
    fn count_edges_by_pos(layout: &YLayout, count_buffer: &mut [usize], edges: &[IndexEdge]) {
        count_buffer.fill(0);
        for e in edges.iter() {
            let index = layout.bottom_index(e.pos);
            unsafe {
                *count_buffer.get_unchecked_mut(index) += 1;
            }
        }
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
    pub(super) fn range_hz_for_indices(&self, i0: usize, i1: usize) -> Range<usize> {
        Self::range_for_indices(i0, i1, &self.hz_parts_start, &self.hz_parts_start)
    }

    #[inline(always)]
    pub(super) fn range_dp_for_indices(&self, i0: usize, i1: usize) -> Range<usize> {
        Self::range_for_indices(i0, i1, &self.dg_pos_parts_start, &self.dg_pos_parts_start)
    }

    #[inline(always)]
    pub(super) fn range_dn_for_indices(&self, i0: usize, i1: usize) -> Range<usize> {
        Self::range_for_indices(i0, i1, &self.dg_neg_parts_start, &self.dg_neg_parts_start)
    }

    #[inline(always)]
    fn range_for_indices(i0: usize, i1: usize, start: &[usize], count: &[usize]) -> Range<usize> {
        unsafe {
            let &start_0 = start.get_unchecked(i0);
            let &start_1 = start.get_unchecked(i1);
            let &count_1 = count.get_unchecked(i1);

            start_0..start_1 + count_1
        }
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
