use alloc::vec;
use alloc::vec::Vec;
use crate::fill::segment::{SegmentFill, NONE};
use crate::core::fill_rule::FillRule;
use crate::deg_90::column_map::Column;
use crate::fill::strategy::{EvenOddStrategy, FillStrategy, NegativeStrategy, NonZeroStrategy, PositiveStrategy};
use crate::fill::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;

impl Column {
    pub(super) fn fill(
        &self,
        fill_rule: FillRule,
        fill_buffer: FillBuffer,
    ) -> Vec<SegmentFill> {
        match fill_rule {
            FillRule::EvenOdd => {
                self.fill_with_strategy::<EvenOddStrategy>(fill_buffer)
            }
            FillRule::NonZero => {
                self.fill_with_strategy::<NonZeroStrategy>(fill_buffer)
            }
            FillRule::Positive => {
                self.fill_with_strategy::<PositiveStrategy>(fill_buffer)
            }
            FillRule::Negative => {
                self.fill_with_strategy::<NegativeStrategy>(fill_buffer)
            }
        }
    }

    fn fill_with_strategy<F: FillStrategy<ShapeCountBoolean>>(
        &self,
        mut fill_buffer: FillBuffer,
    ) -> Vec<SegmentFill> {
        let n = self.segments.len();

        let mut result: Vec<SegmentFill> = Vec::with_capacity(n);

        let mut i = 0;
        while i < n {
            let start = i;
            let pos = self.segments[i].pos;
            i += 1;

            while i < n && self.segments[i].pos == pos {
                i += 1;
            }
            let line = &self.segments[start..i];


            
        }


        result
    }

}


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

struct FillBuffer {
    // mapper: YMapper,
    // hz_edges: Vec<FillHz>,
    // dp_edges: Vec<FillDg>,
    // dn_edges: Vec<FillDg>,
}

/*


impl FillBuffer {
    pub(super) fn new(split_buffer: crate::gear::split_buffer::SplitBuffer) -> Self {
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

        let mut start = 0;
        for &count in self.mapper.hz_parts_count.iter() {
            if count > 1 {
                self.hz_edges[start..start + count].sort_unstable_by(|hz0, hz1|hz0.y.cmp(&hz1.y));
            }
            start += count;
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
        source: &mut crate::gear::fill_source::FillSource,
        buffer: &mut Vec<FillDg>,
        count_buffer: &mut crate::gear::count_buffer::CountBuffer,
    ) {
        count_buffer.reset(max);

        if self.dn_edges.len() > 1 {
            self.dn_edges.sort_by_one_key_and_buffer(false, buffer, |s|s.min_y);
        }

        if self.dp_edges.len() > 1 {
            self.dp_edges.sort_by_one_key_and_buffer(false, buffer, |s|s.min_y);
        }

        let mut i = 0;
        let mut j = 0;
        while i < self.hz_edges.len() {
            let y0 = self.hz_edges[i].y;

            // add all vr in range s.min < y0
            while j < vr_segments.len() && vr_segments[j].range.min < y0 {
                let vr = &vr_segments[j];
                let fill = count_buffer.get_fill::<F>(vr.count, vr.pos);
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
            let (_, fill) = F::add_and_fill(vr.count, ShapeCountBoolean::empty());
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
            dir: segment.count,
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

 */