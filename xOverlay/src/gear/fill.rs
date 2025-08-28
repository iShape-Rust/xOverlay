use crate::core::fill::{FillStrategy, NONE, SegmentFill};
use crate::core::fill_rule::FillRule;
use crate::core::winding::WindingCount;
use crate::gear::count_buffer::CountBuffer;
use crate::gear::fill_buffer::{FillBuffer, FillDg, FillHz};
use crate::gear::section::Section;
use crate::gear::segment::Segment;
use crate::gear::x_mapper::XMapper;
use crate::graph::boolean::winding_count::ShapeCountBoolean;
use alloc::vec;
use alloc::vec::Vec;
use i_key_sort::sort::layout::BinStore;

pub(super) struct FillResult {
    pub(super) vr: Vec<SegmentFill>,
    pub(super) hz: Vec<SegmentFill>,
    pub(super) dp: Vec<SegmentFill>,
    pub(super) dn: Vec<SegmentFill>,
}

impl Section {
    pub(super) fn fill(
        &self,
        fill_rule: FillRule,
        fill_buffer: FillBuffer,
        map_by_columns: XMapper,
    ) -> FillResult {
        match fill_rule {
            FillRule::EvenOdd => {
                self.fill_with_strategy::<EvenOddStrategy>(fill_buffer, map_by_columns)
            }
            FillRule::NonZero => {
                self.fill_with_strategy::<NonZeroStrategy>(fill_buffer, map_by_columns)
            }
            FillRule::Positive => {
                self.fill_with_strategy::<PositiveStrategy>(fill_buffer, map_by_columns)
            }
            FillRule::Negative => {
                self.fill_with_strategy::<NegativeStrategy>(fill_buffer, map_by_columns)
            }
        }
    }

    fn fill_with_strategy<F: FillStrategy<ShapeCountBoolean>>(
        &self,
        mut fill_buffer: FillBuffer,
        map_by_columns: XMapper,
    ) -> FillResult {
        let mut start_vr = 0;
        let mut start_hz = 0;
        let mut start_dp = 0;
        let mut start_dn = 0;

        let scale = self.layout.count().ilog2();

        let mut fill_result = FillResult {
            vr: vec![NONE; self.source.vr_list.len()],
            hz: vec![NONE; self.source.hz_list.len()],
            dp: vec![NONE; self.source.dp_list.len()],
            dn: vec![NONE; self.source.dn_list.len()],
        };

        let hz_capacity = self.source.hz_list.len() >> scale;
        let dp_capacity = self.source.dp_list.len() >> scale;
        let dn_capacity = self.source.dn_list.len() >> scale;

        let bin_max_capacity = dp_capacity.max(dn_capacity);

        let y_range = self.layout.y_range();
        let mut bin_store = BinStore::new_anyway(y_range.min, y_range.max, bin_max_capacity);
        let mut sort_buffer = Vec::with_capacity(bin_max_capacity);
        let mut count_buffer = CountBuffer::new();

        let mut hz_buffer = Vec::with_capacity(hz_capacity);
        let mut dp_buffer = Vec::with_capacity(dp_capacity);
        let mut dn_buffer = Vec::with_capacity(dn_capacity);

        for (column_index, part) in map_by_columns.iter_by_parts().enumerate() {
            let (min_x, max_x) = self.layout.borders(column_index);

            // get slices to new column data

            let vr_slice = &self.source.vr_list[start_vr..start_vr + part.count_vr];
            let hz_slice = &self.source.hz_list[start_hz..start_hz + part.count_hz];
            let dp_slice = &self.source.dp_list[start_dp..start_dp + part.count_dp];
            let dn_slice = &self.source.dn_list[start_dn..start_dn + part.count_dn];

            // prepare column data

            // hz
            hz_buffer.clean_by_min_x(min_x);
            hz_buffer.add(start_hz, hz_slice);

            // dp
            dp_buffer.clean_by_min_x(min_x);
            dp_buffer.add(start_dp, dp_slice);

            // dn
            dn_buffer.clean_by_min_x(min_x);
            dn_buffer.add(start_dn, dn_slice);

            // fill buffer
            fill_buffer.add_hz_edges(max_x, &hz_buffer);
            fill_buffer.add_dp_edges(max_x, &dp_buffer);
            fill_buffer.add_dn_edges(max_x, &dn_buffer);

            fill_buffer.fill::<F>(
                start_vr,
                vr_slice,
                &mut fill_result,
                &mut sort_buffer,
                &mut bin_store,
                &mut count_buffer,
            );

            start_vr += part.count_vr;
            start_hz += part.count_hz;
            start_dp += part.count_dp;
            start_dn += part.count_dn;
        }

        fill_result
    }

    #[inline]
    fn clean_by_min_x_hz(min_x: i32, buffer: &mut Vec<FillHz>) {
        buffer.retain_mut(|e| {
            if e.x_range.max < min_x {
                false
            } else {
                e.x_range.min = min_x;
                true
            }
        });
    }

    fn add_hz(offset: usize, new_segments: &[Segment], buffer: &mut Vec<FillHz>) {
        for (i, s) in new_segments.iter().enumerate() {
            let index = offset + i;
            buffer.push(FillHz::with_segment(index, s));
        }
    }
}

trait CleanByXSwipe {
    fn clean_by_min_x(&mut self, min_x: i32);
}

trait FillAddSegment {
    fn add(&mut self, offset: usize, new_segments: &[Segment]);
}

impl CleanByXSwipe for Vec<FillHz> {
    fn clean_by_min_x(&mut self, min_x: i32) {
        self.retain_mut(|e| {
            if e.x_range.max < min_x {
                false
            } else {
                e.x_range.min = min_x;
                true
            }
        });
    }
}

impl FillAddSegment for Vec<FillHz> {
    fn add(&mut self, offset: usize, new_segments: &[Segment]) {
        for (i, s) in new_segments.iter().enumerate() {
            let index = offset + i;
            self.push(FillHz::with_segment(index, s));
        }
    }
}

impl CleanByXSwipe for Vec<FillDg> {
    fn clean_by_min_x(&mut self, min_x: i32) {
        self.retain_mut(|dn| {
            if dn.x_range.max < min_x {
                false
            } else {
                dn.x_range.min = min_x;
                true
            }
        });
    }
}

impl FillAddSegment for Vec<FillDg> {
    fn add(&mut self, offset: usize, new_segments: &[Segment]) {
        for (i, s) in new_segments.iter().enumerate() {
            let index = offset + i;
            self.push(FillDg::with_segment(index, s));
        }
    }
}

struct EvenOddStrategy;
struct NonZeroStrategy;
struct PositiveStrategy;
struct NegativeStrategy;

impl FillStrategy<ShapeCountBoolean> for EvenOddStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = 1 & top.subj as SegmentFill;
        let subj_bot = 1 & bot.subj as SegmentFill;
        let clip_top = 1 & top.clip as SegmentFill;
        let clip_bot = 1 & bot.clip as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}

impl FillStrategy<ShapeCountBoolean> for NonZeroStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = (top.subj != 0) as SegmentFill;
        let subj_bot = (bot.subj != 0) as SegmentFill;
        let clip_top = (top.clip != 0) as SegmentFill;
        let clip_bot = (bot.clip != 0) as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}

impl FillStrategy<ShapeCountBoolean> for PositiveStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = (top.subj > 0) as SegmentFill;
        let subj_bot = (bot.subj > 0) as SegmentFill;
        let clip_top = (top.clip > 0) as SegmentFill;
        let clip_bot = (bot.clip > 0) as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}

impl FillStrategy<ShapeCountBoolean> for NegativeStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = (top.subj < 0) as SegmentFill;
        let subj_bot = (bot.subj < 0) as SegmentFill;
        let clip_top = (top.clip < 0) as SegmentFill;
        let clip_bot = (bot.clip < 0) as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}
