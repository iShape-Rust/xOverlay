use crate::gear::split::IndexEdge;
use crate::gear::y_layout::YLayout;
use crate::gear::y_mapper::YMapper;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_float::int::rect::IntRect;
use i_key_sort::bin_key::index::{BinKey, BinLayout};
use i_key_sort::sort::key_sort::KeyBinSort;

#[derive(Debug, Clone, Default)]
struct SplitHz {
    index: u32,
    y: i32,
    range: LineRange,
}

#[derive(Debug, Clone)]
struct SplitPosDg {
    index: u32,
    rect: IntRect,
}

#[derive(Debug, Clone)]
struct SplitNegDg {
    index: u32,
    rect: IntRect,
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
    dp_edges: Vec<SplitPosDg>,
    dn_edges: Vec<SplitNegDg>,

    vr_marks: Vec<YMark>,
    hz_marks: Vec<XMark>,
    dp_marks: Vec<XMark>,
    dn_marks: Vec<XMark>,
}

impl SplitBuffer {
    pub(super) fn new(rect: IntRect, log_height: u32) -> Self {
        let layout = YLayout::new(rect, log_height);
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

    pub(super) fn add_hz_edges(&mut self, max_x: i32, edges: &[IndexEdge]) {
        self.mapper.map_hz_edges(edges);
        self.hz_edges.resize(edges.len(), SplitHz::default());
        for e in edges {
            let map_index = self.mapper.next_hz_index(e.pos);
            let hz = e.to_hz(max_x);
            unsafe {
                *self.hz_edges.get_unchecked_mut(map_index) = hz;
            }
        }
    }

    pub(super) fn add_dp_edges(&mut self, max_x: i32, edges: &[IndexEdge]) {
        self.mapper.map_dp_edges(edges);
        self.dp_edges.resize(edges.len(), SplitPosDg::default());
        for e in edges {
            let map_index = self.mapper.next_dg_pos_index(e.pos);
            let dp = e.to_dp(max_x);
            unsafe {
                *self.dp_edges.get_unchecked_mut(map_index) = dp;
            }
        }
    }

    pub(super) fn add_dn_edges(&mut self, max_x: i32, edges: &[IndexEdge]) {
        self.mapper.map_dn_edges(edges);
        self.dn_edges.resize(edges.len(), SplitNegDg::default());
        for e in edges {
            let map_index = self.mapper.next_dg_neg_index(e.pos);
            let dn = e.to_dn(max_x);
            unsafe {
                *self.dn_edges.get_unchecked_mut(map_index) = dn;
            }
        }
    }

    #[inline]
    pub(super) fn intersect_vr(&mut self, vr: IndexEdge) {
        self.intersect_vr_and_hz(vr);
        self.intersect_vr_and_dp(vr);
        self.intersect_vr_and_dn(vr);
    }

    #[inline]
    pub(super) fn intersect(&mut self) {
        self.intersect_hz_and_dp();
        self.intersect_hz_and_dn();
        self.intersect_dgs();
    }

    fn intersect_hz_and_dp(&mut self) {
        let max_height = self.mapper.layout.max_height();
        for hz in self.hz_edges.iter() {
            let y = hz.y;
            let i0 = self.mapper.layout.bottom_index_clamp_min(y - max_height);
            let i1 = self.mapper.layout.bottom_index(y);
            let dp_range = self.mapper.range_dp_for_indices(i0, i1);
            for dp in self.dp_edges[dp_range].iter() {
                if dp.y_range().not_contains(y) {
                    continue;
                }

                let x = dp.find_x(y);
                if hz.range.not_contains(x) {
                    continue;
                }

                if dp.y_range().strict_contains(y) {
                    self.dp_marks.push(XMark { index: dp.index, x });
                }

                if hz.range.strict_contains(x) {
                    self.hz_marks.push(XMark { index: hz.index, x });
                }
            }
        }
    }

    fn intersect_hz_and_dn(&mut self) {
        let max_height = self.mapper.layout.max_height();
        for hz in self.hz_edges.iter() {
            let y = hz.y;
            let i0 = self.mapper.layout.bottom_index_clamp_min(y - max_height);
            let i1 = self.mapper.layout.bottom_index(y);
            let dn_range = self.mapper.range_dn_for_indices(i0, i1);
            for dn in self.dn_edges[dn_range].iter() {
                if dn.y_range().not_contains(y) {
                    continue;
                }

                let x = dn.find_x(y);
                if hz.range.not_contains(x) {
                    continue;
                }

                if dn.y_range().strict_contains(y) {
                    self.dn_marks.push(XMark { index: dn.index, x });
                }

                if hz.range.strict_contains(x) {
                    self.hz_marks.push(XMark { index: hz.index, x });
                }
            }
        }
    }

    #[inline(always)]
    fn intersect_vr_and_hz(&mut self, vr: IndexEdge) {
        let i0 = self.mapper.layout.bottom_index(vr.range.min);
        let i1 = self.mapper.layout.bottom_index(vr.range.max);
        let x = vr.pos;
        let hz_range = self.mapper.range_hz_for_indices(i0, i1);
        for hz in self.hz_edges[hz_range].iter() {
            let y = hz.y;
            if hz.range.not_contains(x) || vr.range.not_contains(y) {
                continue;
            }

            if hz.range.strict_contains(x) {
                self.hz_marks.push(XMark { index: hz.index, x });
            }

            if vr.range.strict_contains(y) {
                self.vr_marks.push(YMark { index: vr.index, y });
            }
        }
    }

    #[inline(always)]
    fn intersect_vr_and_dp(&mut self, vr: IndexEdge) {
        let max_height = self.mapper.layout.max_height();
        let i0 = self
            .mapper
            .layout
            .bottom_index_clamp_min(vr.range.min - max_height);
        let i1 = self.mapper.layout.bottom_index(vr.range.max);
        let x = vr.pos;
        let dg_range = self.mapper.range_dp_for_indices(i0, i1);
        for dp in self.dp_edges[dg_range].iter() {
            if dp.x_range().not_contains(x) {
                continue;
            }

            let y = dp.find_y(x);
            if vr.range.not_contains(y) {
                continue;
            }

            if dp.x_range().strict_contains(x) {
                self.dp_marks.push(XMark { index: dp.index, x });
            }

            if vr.range.strict_contains(y) {
                self.vr_marks.push(YMark { index: vr.index, y });
            }
        }
    }

    #[inline(always)]
    fn intersect_vr_and_dn(&mut self, vr: IndexEdge) {
        let max_height = self.mapper.layout.max_height();
        let i0 = self
            .mapper
            .layout
            .bottom_index_clamp_min(vr.range.min - max_height);
        let i1 = self.mapper.layout.bottom_index(vr.range.max);
        let x = vr.pos;
        let dg_range = self.mapper.range_dn_for_indices(i0, i1);
        for dn in self.dn_edges[dg_range].iter() {
            if dn.x_range().not_contains(x) {
                continue;
            }

            let y = dn.find_y(x);
            if vr.range.not_contains(y) {
                continue;
            }

            if dn.x_range().strict_contains(x) {
                self.dn_marks.push(XMark { index: dn.index, x });
            }

            if vr.range.strict_contains(y) {
                self.vr_marks.push(YMark { index: vr.index, y });
            }
        }
    }

    #[inline(always)]
    fn intersect_dgs(&mut self) {
        let max_height = self.mapper.layout.max_height();
        for dp in self.dp_edges.iter() {
            let i0 = self
                .mapper
                .layout
                .bottom_index_clamp_min(dp.rect.min_y - max_height);
            let i1 = self.mapper.layout.bottom_index(dp.rect.max_y);
            let dn_range = self.mapper.range_dn_for_indices(i0, i1);
            for dn in self.dn_edges[dn_range].iter() {
                let p = Self::cross(dp, dn);

                if dp.strict_contains(p) {
                    self.dp_marks.push(XMark { index: dp.index, x: p.x });
                }

                if dn.strict_contains(p) {
                    self.dn_marks.push(XMark { index: dn.index, x: p.x });
                }
            }
        }
    }

    #[inline(always)]
    fn cross(dp: &SplitPosDg, dn: &SplitNegDg) -> IntPoint {
        let y = (dp.rect.min_y + dn.rect.max_y) >> 1;
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

impl IndexEdge {
    #[inline(always)]
    fn to_hz(&self, limit_x: i32) -> SplitHz {
        SplitHz {
            index: self.index,
            y: self.pos,
            range: LineRange {
                min: self.range.min,
                max: self.range.max.min(limit_x),
            },
        }
    }

    #[inline(always)]
    fn to_dp(&self, limit_x: i32) -> SplitPosDg {
        let max_x = self.range.max.min(limit_x);
        let min_x = self.range.min;
        let width = max_x - min_x;
        let min_y = self.pos;
        let max_y = min_y + width;
        SplitPosDg {
            index: self.index,
            rect: IntRect {
                min_x,
                max_x,
                min_y,
                max_y,
            },
        }
    }

    #[inline(always)]
    fn to_dn(&self, limit_x: i32) -> SplitNegDg {
        let (max_x, min_y) = if limit_x < self.range.max {
            let dx = self.range.max - limit_x;
            (limit_x, self.pos + dx)
        } else {
            (self.range.max, self.pos)
        };
        let min_x = self.range.min;
        let width = max_x - min_x;
        let max_y = min_y + width;
        SplitNegDg {
            index: self.index,
            rect: IntRect {
                min_x,
                max_x,
                min_y,
                max_y,
            },
        }
    }
}

impl Default for SplitPosDg {
    #[inline]
    fn default() -> Self {
        Self {
            index: 0,
            rect: IntRect {
                min_x: 0,
                max_x: 0,
                min_y: 0,
                max_y: 0,
            },
        }
    }
}

impl Default for SplitNegDg {
    #[inline]
    fn default() -> Self {
        Self {
            index: 0,
            rect: IntRect {
                min_x: 0,
                max_x: 0,
                min_y: 0,
                max_y: 0,
            },
        }
    }
}

impl SplitPosDg {
    #[inline(always)]
    fn x_range(&self) -> LineRange {
        LineRange {
            min: self.rect.min_x,
            max: self.rect.max_x,
        }
    }

    #[inline(always)]
    fn y_range(&self) -> LineRange {
        LineRange {
            min: self.rect.min_y,
            max: self.rect.max_y,
        }
    }

    #[inline(always)]
    fn find_y(&self, x: i32) -> i32 {
        debug_assert!(!self.x_range().not_contains(x));
        let dx = x - self.rect.min_x;
        self.rect.min_y + dx
    }

    #[inline(always)]
    fn find_x(&self, y: i32) -> i32 {
        debug_assert!(!self.y_range().not_contains(y));
        let dy = y - self.rect.min_y;
        self.rect.min_x + dy
    }

    #[inline(always)]
    fn strict_contains(&self, p: IntPoint) -> bool {
        self.x_range().strict_contains(p.x) && self.y_range().strict_contains(p.y)
    }
}

impl SplitNegDg {
    #[inline(always)]
    fn x_range(&self) -> LineRange {
        LineRange {
            min: self.rect.min_x,
            max: self.rect.max_x,
        }
    }

    #[inline(always)]
    fn y_range(&self) -> LineRange {
        LineRange {
            min: self.rect.min_y,
            max: self.rect.max_y,
        }
    }

    #[inline(always)]
    fn find_y(&self, x: i32) -> i32 {
        debug_assert!(!self.x_range().not_contains(x));
        let dx = self.rect.max_x - x;
        self.rect.min_y + dx
    }

    #[inline(always)]
    fn find_x(&self, y: i32) -> i32 {
        debug_assert!(!self.y_range().not_contains(y));
        let dy = self.rect.max_y - y;
        self.rect.min_x + dy
    }

    #[inline(always)]
    fn strict_contains(&self, p: IntPoint) -> bool {
        self.x_range().strict_contains(p.x) && self.y_range().strict_contains(p.y)
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