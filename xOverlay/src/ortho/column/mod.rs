mod fill;
mod filter;
mod split;

use crate::core::fill::SegmentFill;
use crate::ortho::mapper::Counter;
use crate::ortho::segment::OrthoSegment;
use alloc::vec::Vec;
use i_shape::util::reserve::Reserve;

#[derive(Clone)]
pub(crate) struct Column<C> {
    pub(crate) vr_segments: Vec<OrthoSegment<C>>,
    pub(crate) hz_segments: Vec<OrthoSegment<C>>,
    pub(crate) vr_fills: Vec<SegmentFill>,
    pub(crate) hz_fills: Vec<SegmentFill>,
    pub(crate) border_points: Vec<i32>,
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) links_start: usize,
    pub(crate) links_count: usize,
}

impl<C> Default for Column<C> {
    #[inline]
    fn default() -> Self {
        Column {
            vr_segments: Default::default(),
            hz_segments: Default::default(),
            vr_fills: Default::default(),
            hz_fills: Default::default(),
            border_points: Default::default(),
            min: 0,
            max: 0,
            links_start: 0,
            links_count: 0,
        }
    }
}

impl<C> Column<C> {
    #[inline(always)]
    pub(super) fn init_with_counter(&mut self, min: i32, max: i32, counter: Counter) {
        self.vr_segments.clear();
        self.vr_segments.reserve_capacity(counter.vr);
        self.hz_segments.clear();
        self.hz_segments.reserve_capacity(counter.hz);
        self.border_points.clear();
        self.border_points.reserve_capacity(counter.border_points);
        self.hz_fills.clear();
        self.vr_fills.clear();
        self.min = min;
        self.max = max;
    }

    #[inline(always)]
    pub(crate) fn links_end(&self) -> usize {
        self.links_start + self.links_count
    }
}