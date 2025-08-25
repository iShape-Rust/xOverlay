use crate::gear::segment::Segment;
use alloc::vec;
use alloc::vec::Vec;
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

    pub(super) fn add_hz_list(&mut self, list: &[Segment]) {
        self.hz_parts_count.clear();
        self.hz_parts_count.resize(self.layout.count(), 0);

        for hz in list.iter() {
            let index = self.layout.bottom_index(hz.min);
            unsafe {
                *self.hz_parts_count.get_unchecked_mut(index) += 1;
            }
        }
        let mut offset = 0;
        for (count, start) in self.hz_parts_count.iter_mut().zip(self.hz_parts_start.iter_mut()) {
            *start = offset;
            offset += *count;
            *count = 0;
        }
    }

    pub(super) fn add_dg_pos_list(&mut self, list: &[Segment]) {
        self.dg_pos_parts_count.resize(self.layout.count(), 0);
        for dg in list.iter() {
            let index = self.layout.bottom_index(dg.min);
            unsafe {
                *self.dg_pos_parts_count.get_unchecked_mut(index) += 1;
            }
        }

        let mut offset = 0;
        for (count, start) in self.dg_pos_parts_count.iter_mut().zip(self.dg_pos_parts_start.iter_mut()) {
            *start = offset;
            offset += *count;
            *count = 0;
        }
    }

    pub(super) fn add_dg_neg_list(&mut self, list: &[Segment]) {
        self.dg_neg_parts_count.resize(self.layout.count(), 0);
        for dg in list.iter() {
            let index = self.layout.bottom_index(dg.min);
            unsafe {
                *self.dg_neg_parts_count.get_unchecked_mut(index) += 1;
            }
        }

        let mut offset = 0;
        for (count, start) in self.dg_neg_parts_count.iter_mut().zip(self.dg_neg_parts_start.iter_mut()) {
            *start = offset;
            offset += *count;
            *count = 0;
        }
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
