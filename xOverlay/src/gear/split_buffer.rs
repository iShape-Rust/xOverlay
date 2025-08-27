use crate::gear::split::IndexEdge;
use crate::gear::y_layout::YLayout;
use crate::gear::y_mapper::YMapper;
use crate::geom::diagonal::{Diagonal, NegativeDiagonal, PositiveDiagonal};
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::bin_key::index::{BinKey, BinLayout};
use i_key_sort::sort::key_sort::KeyBinSort;
use crate::gear::segment::Segment;

#[derive(Debug, Clone, Default)]
pub(super) struct SplitHz {
    index: u32,
    pub(super) y: i32,
    pub(super) x_range: LineRange,
    parent_x_range: LineRange,
}

#[derive(Debug, Clone, Default)]
pub(super) struct SplitDp {
    index: u32,
    pub(super) x_range: LineRange,
    pub(super) y_range: LineRange,
    parent_x_range: LineRange,
}

#[derive(Debug, Clone, Default)]
pub(super) struct SplitDn {
    index: u32,
    pub(super) x_range: LineRange,
    pub(super) y_range: LineRange,
    parent_x_range: LineRange,
}

#[derive(Clone, Copy)]
pub(super) struct XMark {
    pub(super) index: u32,
    pub(super) x: i32,
}

#[derive(Clone, Copy)]
pub(super) struct YMark {
    pub(super) index: u32,
    pub(super) y: i32,
}

pub(super) struct MarkResult {
    pub(super) vr_marks: Vec<YMark>,
    pub(super) hz_marks: Vec<XMark>,
    pub(super) dp_marks: Vec<XMark>,
    pub(super) dn_marks: Vec<XMark>,
}

pub(super) struct SplitBuffer {
    mapper: YMapper,
    hz_edges: Vec<SplitHz>,
    dp_edges: Vec<SplitDp>,
    dn_edges: Vec<SplitDn>,

    vr_marks: Vec<YMark>,
    hz_marks: Vec<XMark>,
    dp_marks: Vec<XMark>,
    dn_marks: Vec<XMark>,
}

impl SplitBuffer {
    pub(super) fn new(y_range: LineRange, log_height: u32) -> Self {
        let layout = YLayout::new(y_range, log_height);
        let mapper = YMapper::new(layout);

        Self {
            mapper,
            hz_edges: Vec::new(),
            dp_edges: Vec::new(),
            dn_edges: Vec::new(),
            vr_marks: Vec::with_capacity(32),
            hz_marks: Vec::with_capacity(32),
            dp_marks: Vec::with_capacity(32),
            dn_marks: Vec::with_capacity(32),
        }
    }

    #[inline(always)]
    pub(super) fn is_not_empty_hz(&self) -> bool {
        !self.hz_edges.is_empty()
    }

    #[inline(always)]
    pub(super) fn is_not_empty_dp(&self) -> bool {
        !self.dp_edges.is_empty()
    }

    #[inline(always)]
    pub(super) fn is_not_empty_dn(&self) -> bool {
        !self.dn_edges.is_empty()
    }

    pub(super) fn add_hz_edges(&mut self, max_x: i32, slice: &[SplitHz]) {
        self.mapper.map_hz(slice);
        self.hz_edges.resize(slice.len(), SplitHz::default());
        for hz in slice {
            let map_index = self.mapper.next_hz_index(hz.y);
            let left = hz.left_part(max_x);
            unsafe {
                *self.hz_edges.get_unchecked_mut(map_index) = left;
            }
        }
    }

    pub(super) fn add_dp_edges(&mut self, max_x: i32, slice: &[SplitDp]) {
        self.mapper.map_dp(slice);
        self.dp_edges.resize(slice.len(), SplitDp::default());
        for dp in slice {
            let map_index = self.mapper.next_dp_index(dp.y_range.min);
            let left = dp.left_part(max_x);
            unsafe {
                *self.dp_edges.get_unchecked_mut(map_index) = left;
            }
        }
    }

    pub(super) fn add_dn_edges(&mut self, max_x: i32, slice: &[SplitDn]) {
        self.mapper.map_dn_edges(slice);
        self.dn_edges.resize(slice.len(), SplitDn::default());
        for dn in slice {
            let map_index = self.mapper.next_dn_index(dn.y_range.min);
            let left = dn.left_part(max_x);
            unsafe {
                *self.dn_edges.get_unchecked_mut(map_index) = left;
            }
        }
    }


    #[inline]
    pub(super) fn intersect(&mut self) {
        if self.is_not_empty_hz() {
            if self.is_not_empty_dp() {
                self.intersect_hz_and_dp();
            }
            if self.is_not_empty_dn() {
                self.intersect_hz_and_dn();
            }
        }
        if self.is_not_empty_dp() && self.is_not_empty_dn() {
            self.intersect_dgs();
        }
    }

    fn intersect_hz_and_dp(&mut self) {
        for hz in self.hz_edges.iter() {
            let y = hz.y;
            let dp_range = self.mapper.indices_bottom_offset_dp(y);
            for dp in self.dp_edges[dp_range].iter() {
                if dp.y_range.not_contains(y) {
                    continue;
                }

                let x = dp.find_x(y);
                if hz.x_range.not_contains(x) {
                    continue;
                }

                if dp.y_range.strict_contains(y) {
                    self.dp_marks.push(XMark { index: dp.index, x });
                }

                if hz.x_range.strict_contains(x) {
                    self.hz_marks.push(XMark { index: hz.index, x });
                }
            }
        }
    }

    fn intersect_hz_and_dn(&mut self) {
        for hz in self.hz_edges.iter() {
            let y = hz.y;
            let dn_range = self.mapper.indices_bottom_offset_dn(y);
            for dn in self.dn_edges[dn_range].iter() {
                if dn.y_range.not_contains(y) {
                    continue;
                }

                let x = dn.find_x(y);
                if hz.x_range.not_contains(x) {
                    continue;
                }

                if dn.y_range.strict_contains(y) {
                    self.dn_marks.push(XMark { index: dn.index, x });
                }

                if hz.x_range.strict_contains(x) {
                    self.hz_marks.push(XMark { index: hz.index, x });
                }
            }
        }
    }

    #[inline(always)]
    pub(super) fn intersect_vr_and_hz(&mut self, vr: IndexEdge) {
        let hz_range = self.mapper.indices_by_range_hz(vr.range);
        let x = vr.pos;
        for hz in self.hz_edges[hz_range].iter() {
            let y = hz.y;
            if hz.x_range.not_contains(x) || vr.range.not_contains(y) {
                continue;
            }

            if hz.parent_x_range.strict_contains(x) {
                self.hz_marks.push(XMark { index: hz.index, x });
            }

            if vr.range.strict_contains(y) {
                self.vr_marks.push(YMark { index: vr.index, y });
            }
        }
    }

    #[inline(always)]
    pub(super) fn intersect_vr_and_dp(&mut self, vr: IndexEdge) {
        let dp_range = self.mapper.indices_by_range_bottom_offset_dp(vr.range);
        let x = vr.pos;
        for dp in self.dp_edges[dp_range].iter() {
            if dp.x_range.not_contains(x) {
                continue;
            }

            let y = dp.find_y(x);
            if vr.range.not_contains(y) {
                continue;
            }

            if dp.parent_x_range.strict_contains(x) {
                self.dp_marks.push(XMark { index: dp.index, x });
            }

            if vr.range.strict_contains(y) {
                self.vr_marks.push(YMark { index: vr.index, y });
            }
        }
    }

    #[inline(always)]
    pub(super) fn intersect_vr_and_dn(&mut self, vr: IndexEdge) {
        let dn_range = self.mapper.indices_by_range_bottom_offset_dn(vr.range);
        let x = vr.pos;
        for dn in self.dn_edges[dn_range].iter() {
            if dn.x_range.not_contains(x) {
                continue;
            }

            let y = dn.find_y(x);
            if vr.range.not_contains(y) {
                continue;
            }

            if dn.parent_x_range.strict_contains(x) {
                self.dn_marks.push(XMark { index: dn.index, x });
            }

            if vr.range.strict_contains(y) {
                self.vr_marks.push(YMark { index: vr.index, y });
            }
        }
    }

    #[inline(always)]
    fn intersect_dgs(&mut self) {
        for dp in self.dp_edges.iter() {
            let dn_range = self.mapper.indices_by_range_bottom_offset_dn(dp.y_range);
            for dn in self.dn_edges[dn_range].iter() {
                let p = Self::cross_dgs(dp, dn);

                if dp.parent_x_range.strict_contains(p.x) {
                    self.dp_marks.push(XMark {
                        index: dp.index,
                        x: p.x,
                    });
                }

                if dn.parent_x_range.strict_contains(p.x) {
                    self.dn_marks.push(XMark {
                        index: dn.index,
                        x: p.x,
                    });
                }
            }
        }
    }

    #[inline(always)]
    fn cross_dgs(dp: &SplitDp, dn: &SplitDn) -> IntPoint {
        let dy = dp.y_range.min.wrapping_add(dn.y_range.min);
        let dx = dp.x_range.min.wrapping_add(dn.x_range.max);

        let y = dy.wrapping_add(dx) >> 1;
        let x = dp.find_x(y);
        IntPoint::new(x, y)
    }

    pub(super) fn into_marks(mut self) -> MarkResult {
        self.hz_marks
            .sort_with_bins(|m0, m1| m0.index.cmp(&m1.index).then(m0.x.cmp(&m1.x)));
        self.vr_marks
            .sort_with_bins(|m0, m1| m0.index.cmp(&m1.index).then(m0.y.cmp(&m1.y)));
        self.dp_marks
            .sort_with_bins(|m0, m1| m0.index.cmp(&m1.index).then(m0.x.cmp(&m1.x)));
        self.dn_marks
            .sort_with_bins(|m0, m1| m0.index.cmp(&m1.index).then(m0.x.cmp(&m1.x)));

        MarkResult {
            vr_marks: self.vr_marks,
            hz_marks: self.hz_marks,
            dp_marks: self.dp_marks,
            dn_marks: self.dn_marks,
        }
    }
}

impl SplitHz {
    #[inline(always)]
    pub(super) fn with_segment(index: usize, segment: &Segment) -> Self {
        Self {
            index: index as u32,
            y: segment.pos,
            x_range: segment.range,
            parent_x_range: segment.range,
        }
    }

    #[inline(always)]
    fn left_part(&self, max_x: i32) -> Self {
        if self.x_range.max <= max_x {
            return self.clone();
        }

        Self {
            index: self.index,
            y: self.y,
            x_range: LineRange {
                min: self.x_range.min,
                max: max_x,
            },
            parent_x_range: self.parent_x_range,
        }
    }
}

impl SplitDp {

    #[inline(always)]
    pub(super) fn with_segment(index: usize, segment: &Segment) -> Self {
        let min_y = segment.pos;
        let max_y = PositiveDiagonal::new(segment.range, min_y).find_y(segment.range.max);
        Self {
            index: index as u32,
            x_range: segment.range,
            y_range: LineRange::with_min_max(min_y, max_y),
            parent_x_range: segment.range,
        }
    }

    #[inline(always)]
    fn left_part(&self, max_x: i32) -> Self {
        if self.x_range.max <= max_x {
            return self.clone();
        }

        let x_range = LineRange::with_min_max(self.x_range.min, max_x);

        let max_y = self.find_y(max_x);
        let y_range = LineRange::with_min_max(self.y_range.min, max_y);

        Self {
            index: self.index,
            x_range,
            y_range,
            parent_x_range: self.parent_x_range,
        }
    }

    #[inline(always)]
    fn with_limit(edge: &IndexEdge, limit_x: i32) -> Self {
        let min_x = edge.range.min;
        let max_x = edge.range.max.min(limit_x);
        let x_range = LineRange::with_min_max(min_x, max_x);

        let min_y = edge.pos;
        let max_y = PositiveDiagonal::new(x_range, edge.pos).find_y(max_x);
        let y_range = LineRange::with_min_max(min_y, max_y);

        Self {
            index: edge.index,
            x_range,
            y_range,
            parent_x_range: edge.range,
        }
    }
}

impl SplitDn {

    #[inline(always)]
    pub(super) fn with_segment(index: usize, segment: &Segment) -> Self {
        let min_y = segment.pos;
        let max_y = NegativeDiagonal::new(segment.range, min_y).find_y(segment.range.min);
        Self {
            index: index as u32,
            x_range: segment.range,
            y_range: LineRange::with_min_max(min_y, max_y),
            parent_x_range: segment.range,
        }
    }

    #[inline(always)]
    fn left_part(&self, max_x: i32) -> Self {
        if self.x_range.max <= max_x {
            return self.clone();
        }
        let max_x = max_x;
        let x_range = LineRange::with_min_max(self.x_range.min, max_x);

        let min_y = self.find_y(max_x);
        let y_range = LineRange::with_min_max(min_y, self.y_range.max);

        Self {
            index: self.index,
            x_range,
            y_range,
            parent_x_range: self.parent_x_range,
        }
    }
}

impl SplitDp {
    #[inline(always)]
    pub(super) fn find_y(&self, x: i32) -> i32 {
        debug_assert!(!self.x_range.not_contains(x));
        PositiveDiagonal::new(self.x_range, self.y_range.min).find_y(x)
    }

    #[inline(always)]
    pub(super) fn find_x(&self, y: i32) -> i32 {
        debug_assert!(!self.y_range.not_contains(y));
        PositiveDiagonal::new(self.x_range, self.y_range.min).find_x(y)
    }
}

impl SplitDn {
    #[inline(always)]
    pub(super) fn find_y(&self, x: i32) -> i32 {
        debug_assert!(!self.x_range.not_contains(x));
        NegativeDiagonal::new(self.x_range, self.y_range.min).find_y(x)
    }

    #[inline(always)]
    pub(super) fn find_x(&self, y: i32) -> i32 {
        debug_assert!(!self.y_range.not_contains(y));
        NegativeDiagonal::new(self.x_range, self.y_range.min).find_x(y)
    }
}

impl BinKey<usize> for XMark {
    #[inline(always)]
    fn bin_key(&self) -> usize {
        self.index as usize
    }

    #[inline(always)]
    fn bin_index(&self, layout: &BinLayout<usize>) -> usize {
        layout.index(self.bin_key())
    }
}

impl BinKey<usize> for YMark {
    #[inline(always)]
    fn bin_key(&self) -> usize {
        self.index as usize
    }

    #[inline(always)]
    fn bin_index(&self, layout: &BinLayout<usize>) -> usize {
        layout.index(self.bin_key())
    }
}
