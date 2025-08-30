use alloc::vec::Vec;
use crate::core::fill::{FillStrategy, SegmentFill};
use crate::core::winding::WindingCount;
use crate::gear::fill_buffer::FillHz;
use crate::geom::range::LineRange;
use crate::gear::winding_count::ShapeCountBoolean;

struct Anchor {
    pos: i32,
    count: ShapeCountBoolean,
}

pub(super) struct CountBuffer {
    max: i32,
    counts: Vec<Anchor>,
}

impl CountBuffer {

    pub(super) fn new() -> Self {
        Self { max: 0, counts: Vec::with_capacity(16) }
    }

    pub(super) fn reset(&mut self, max: i32) {
        self.max = max;
        self.counts.push(Anchor { pos: max + 1, count: ShapeCountBoolean::empty() });
    }

    #[inline]
    pub(super) fn add_hz<F: FillStrategy<ShapeCountBoolean>, S: XCount>(&mut self, s: &S) -> SegmentFill {
        // __a0____a1____a2
        match self.counts.binary_search_by(|a| a.pos.cmp(&s.x_range().min)) {
            Ok(i0) => {
                let pos = unsafe { self.counts.get_unchecked(i0 + 1).pos };
                if pos == s.x_range().max {
                    self.add_hz_11::<F, S>(s, i0)
                } else {
                    self.add_hz_10::<F, S>(s, i0)
                }
            }
            Err(i0) => {
                let pos = unsafe { self.counts.get_unchecked(i0).pos };
                if pos == s.x_range().max {
                    self.add_hz_01::<F, S>(s, i0)
                } else {
                    self.add_hz_00::<F, S>(s, i0)
                }
            }
        }
    }

    #[inline]
    pub(super) fn add_hz_00<F: FillStrategy<ShapeCountBoolean>, S: XCount>(&mut self, s: &S, i0: usize) -> SegmentFill {
        // __c2____a0____c2(cx)____a1____c2____[a2]

        let c2 = unsafe { self.counts.get_unchecked(i0).count };
        let (cx, fill) = F::add_and_fill(s.dir(), c2);

        let a0 = Anchor {
            pos: s.x_range().min,
            count: c2,
        };
        let a1 = Anchor {
            pos: s.x_range().max,
            count: cx,
        };

        self.counts.splice(i0..i0, [a0, a1]);
        fill
    }

    #[inline]
    pub(super) fn add_hz_01<F: FillStrategy<ShapeCountBoolean>, S: XCount>(&mut self, s: &S, i1: usize) -> SegmentFill {
        // __c1____a0____c1(cx)____[a1]____c2____[a2]

        let c1 = unsafe { self.counts.get_unchecked(i1).count };
        let c2 = unsafe { self.counts.get_unchecked(i1 + 1).count };

        let (cx, fill) = F::add_and_fill(s.dir(), c1);

        if cx == c2 {
            // move a1 to a0
            self.counts[i1].pos = s.x_range().min;
        } else {
            // add a0
            self.counts[i1].count = cx;
            let a0 = Anchor {
                pos: s.x_range().min,
                count: c1,
            };
            self.counts.insert(i1, a0);
        }

        fill
    }

    #[inline]
    pub(super) fn add_hz_10<F: FillStrategy<ShapeCountBoolean>, S: XCount>(&mut self, s: &S, i0: usize) -> SegmentFill {
        // __c0____[a0]____c2(cx)____a1____c2____[a2]

        let c0 = unsafe { self.counts.get_unchecked(i0).count };
        let c2 = unsafe { self.counts.get_unchecked(i0 + 1).count };

        let (cx, fill) = F::add_and_fill(s.dir(), c2);

        if c0 == cx {
            // move a0 to a1
            self.counts[i0].pos = s.x_range().max;
        } else {
            // add a1
            let a1 = Anchor {
                pos: s.x_range().max,
                count: cx,
            };
            self.counts.insert(i0 + 1, a1);
        }

        fill
    }

    #[inline]
    pub(super) fn add_hz_11<F: FillStrategy<ShapeCountBoolean>, S: XCount>(&mut self, s: &S, i0: usize) -> SegmentFill {
        // __c0____[a0]____c1(cx)____[a1]____c2____[a2]

        let c0 = unsafe { self.counts.get_unchecked(i0).count };
        let c1 = unsafe { self.counts.get_unchecked(i0 + 1).count };
        let c2 = unsafe { self.counts.get_unchecked(i0 + 2).count };

        let (cx, fill) = F::add_and_fill(s.dir(), c1);

        let rem_a0 = c0 == cx;
        let rem_a1 = c2 == cx;

        match (rem_a0, rem_a1) {
            (true, true) => _ = self.counts.drain(i0..i0 + 2),
            (false, true) => {
                _ = self.counts.remove(i0 + 1);
            }
            (true, false) => {
                _ = self.counts.remove(i0);
                self.counts[i0].count = cx;
            }
            _ => {
                self.counts[i0 + 1].count = cx;
            }
        };

        fill
    }

    #[inline]
    pub(super) fn get_fill<F: FillStrategy<ShapeCountBoolean>>(&self, dir: ShapeCountBoolean, x: i32) -> SegmentFill {
        let count = if x == self.max {
            let x0 = x - 1;
            let index = match self.counts.binary_search_by(|a| a.pos.cmp(&x0)) {
                Ok(index) => index + 1,
                Err(index) => index,
            };

            self.counts[index].count.invert()
        } else {
            let index = match self.counts.binary_search_by(|a| a.pos.cmp(&x)) {
                Ok(index) => index + 1,
                Err(index) => index,
            };

            self.counts[index].count
        };


        let (_, fill) = F::add_and_fill(dir, count);
        fill
    }
}

pub(super) trait XCount {
    fn x_range(&self) -> LineRange;
    fn dir(&self) -> ShapeCountBoolean;
}

impl XCount for FillHz {

    #[inline(always)]
    fn x_range(&self) -> LineRange {
        self.x_range
    }

    #[inline(always)]
    fn dir(&self) -> ShapeCountBoolean {
        self.dir
    }
}