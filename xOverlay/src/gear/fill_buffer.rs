use crate::core::fill::FillStrategy;
use crate::gear::count_buffer::CountBuffer;
use crate::gear::segment::Segment;
use crate::gear::split_buffer::SplitBuffer;
use crate::gear::y_mapper::YMapper;
use crate::geom::diagonal::{Diagonal, NegativeDiagonal};
use crate::geom::range::LineRange;
use crate::gear::winding_count::ShapeCountBoolean;
use alloc::vec::Vec;
use i_key_sort::sort::layout::BinStore;
use crate::core::winding::WindingCount;
use crate::gear::fill_source::FillSource;

#[derive(Debug, Clone, Default)]
pub(super) struct FillHz {
    pub(super) index: u32,
    pub(super) dir: ShapeCountBoolean,
    pub(super) y: i32,
    pub(super) x_range: LineRange,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct FillDg {
    pub(super) index: u32,
    pub(super) dir: ShapeCountBoolean,
    pub(super) x_range: LineRange,
    pub(super) min_y: i32,
}

pub(super) struct FillBuffer {
    mapper: YMapper,
    hz_edges: Vec<FillHz>,
    dp_edges: Vec<FillDg>,
    dn_edges: Vec<FillDg>,
}

impl FillBuffer {
    pub(super) fn new(split_buffer: SplitBuffer) -> Self {
        Self {
            hz_edges: Vec::with_capacity(split_buffer.hz_edges.len()),
            dp_edges: Vec::with_capacity(split_buffer.dp_edges.len()),
            dn_edges: Vec::with_capacity(split_buffer.dn_edges.len()),
            mapper: split_buffer.mapper,
        }
    }

    pub(super) fn add_hz_edges(&mut self, max_x: i32, slice: &[FillHz]) {
        self.mapper.map_hz(slice);
        self.hz_edges.resize(slice.len(), FillHz::default());
        for hz in slice {
            let map_index = self.mapper.next_hz_index(hz.y);
            let left = hz.left_part(max_x);
            unsafe {
                *self.hz_edges.get_unchecked_mut(map_index) = left;
            }
        }
    }

    pub(super) fn add_dp_edges(&mut self, max_x: i32, slice: &[FillDg]) {
        self.mapper.map_dp(slice);
        self.dp_edges.resize(slice.len(), FillDg::default());
        for dp in slice {
            let map_index = self.mapper.next_dp_index(dp.min_y);
            let left = dp.left_part_dp(max_x);
            unsafe {
                *self.dp_edges.get_unchecked_mut(map_index) = left;
            }
        }
    }

    pub(super) fn add_dn_edges(&mut self, max_x: i32, slice: &[FillDg]) {
        self.mapper.map_dn(slice);
        self.dn_edges.resize(slice.len(), FillDg::default());
        for dn in slice {
            let map_index = self.mapper.next_dn_index(dn.min_y);
            let left = dn.left_part_dn(max_x);
            unsafe {
                *self.dn_edges.get_unchecked_mut(map_index) = left;
            }
        }
    }

    pub(super) fn fill<F: FillStrategy<ShapeCountBoolean>>(
        &mut self,
        max: i32,
        start_vr: usize,
        vr_segments: &[Segment],
        source: &mut FillSource,
        buffer: &mut Vec<FillDg>,
        bin_store: &mut BinStore<i32>,
        count_buffer: &mut CountBuffer,
    ) {
        count_buffer.reset(max);

        // sort dp and dn
        if self.dn_edges.len() > 1 {
            self.dn_edges.sort_diagonals_by_min_y(buffer, bin_store);
        }

        if self.dn_edges.len() > 1 {
            self.dn_edges.sort_diagonals_by_min_y(buffer, bin_store);
        }

        let mut i = 0;
        let mut j = 0;
        while i < self.hz_edges.len() {
            let y0 = self.hz_edges[i].y;

            // add all vr in range s.min < y0
            while j < vr_segments.len() && vr_segments[j].range.min < y0 {
                let vr = &vr_segments[j];
                let fill = count_buffer.get_fill::<F>(vr.dir, vr.pos);
                let vr_index = start_vr + j;
                unsafe {
                    *source.vr.get_unchecked_mut(vr_index) = fill;
                }
                j += 1;
            }

            // add all hz with same y
            while i < self.hz_edges.len() && self.hz_edges[i].y == y0 {
                let hz = &self.hz_edges[i];
                let fill = count_buffer.add_hz::<F, FillHz>(hz);
                let hz_index = hz.index as usize;
                if hz_index < source.hz.len() {
                    source.hz[hz_index] = fill;
                }

                i += 1;
            }
        }

        while j < vr_segments.len() {
            let vr = &vr_segments[j];
            let (_, fill) = F::add_and_fill(vr.dir, ShapeCountBoolean::empty());
            let vr_index = start_vr + j;
            unsafe {
                *source.vr.get_unchecked_mut(vr_index) = fill;
            }
            j += 1;
        }
    }
}

impl FillHz {
    #[inline(always)]
    pub(super) fn with_segment(index: usize, segment: &Segment) -> Self {
        Self {
            index: index as u32,
            dir: segment.dir,
            y: segment.pos,
            x_range: segment.range,
        }
    }

    #[inline(always)]
    fn left_part(&self, max_x: i32) -> Self {
        if self.x_range.max <= max_x {
            return self.clone();
        }

        Self {
            index: self.index,
            dir: self.dir,
            y: self.y,
            x_range: LineRange {
                min: self.x_range.min,
                max: max_x,
            },
        }
    }
}

impl FillDg {
    #[inline(always)]
    pub(super) fn with_segment(index: usize, segment: &Segment) -> Self {
        Self {
            index: index as u32,
            dir: segment.dir,
            x_range: segment.range,
            min_y: segment.pos,
        }
    }

    #[inline(always)]
    fn left_part_dp(&self, max_x: i32) -> Self {
        if self.x_range.max <= max_x {
            return self.clone();
        }

        let x_range = LineRange::with_min_max(self.x_range.min, max_x);

        Self {
            index: self.index,
            dir: self.dir,
            x_range,
            min_y: self.min_y,
        }
    }

    #[inline(always)]
    fn left_part_dn(&self, max_x: i32) -> Self {
        if self.x_range.max <= max_x {
            return self.clone();
        }
        let max_x = max_x;
        let x_range = LineRange::with_min_max(self.x_range.min, max_x);
        let min_y = NegativeDiagonal::new(self.x_range, self.min_y).find_y(max_x);

        Self {
            index: self.index,
            dir: self.dir,
            x_range,
            min_y,
        }
    }
}

trait SortDiagonalsByMinY {
    fn sort_diagonals_by_min_y(&mut self, buffer: &mut Vec<FillDg>, bin_store: &mut BinStore<i32>);
}

impl SortDiagonalsByMinY for [FillDg] {
    fn sort_diagonals_by_min_y(&mut self, buffer: &mut Vec<FillDg>, bin_store: &mut BinStore<i32>) {
        buffer.resize(self.len(), Default::default());
        let target = buffer.as_mut_slice();

        bin_store.reserve_bins_with_key(self.iter().map(|s| s.min_y));
        bin_store.prepare_bins();
        bin_store.copy_by_key(self, target, |s| s.min_y);

        bin_store.sort_by_bins(target, |s0, s1| s0.min_y.cmp(&s1.min_y));
        bin_store.clear();

        // copy sorted elements back to slice
        self.copy_from_slice(target);
    }
}
