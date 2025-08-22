use crate::core::fill::{FillStrategy, NONE, SegmentFill};
use crate::core::winding::WindingCount;
use crate::ortho::column::Column;
use crate::ortho::segment::OrthoSegment;
use alloc::vec::Vec;

struct Anchor<C> {
    pos: i32,
    count: C,
}

struct CountBuffer<C> {
    counts: Vec<Anchor<C>>,
}

impl<C: WindingCount> Column<C> {
    pub(crate) fn fill_with_strategy<F: FillStrategy<C>>(&mut self) {
        self.vr_fills.resize(self.vr_segments.len(), NONE);
        self.hz_fills.resize(self.hz_segments.len(), NONE);

        let mut buffer = CountBuffer::new(self.min, self.max);

        let mut i = 0;
        let mut j = 0;
        while i < self.hz_segments.len() {
            let y0 = self.hz_segments[i].pos;

            // add all vr in range s.min < y0
            while j < self.vr_segments.len() && self.vr_segments[j].min < y0 {
                let vr = &self.vr_segments[j];
                let fill = buffer.add_vr::<F>(vr);
                unsafe {
                    *self.vr_fills.get_unchecked_mut(j) = fill;
                }
                j += 1;
            }

            // add all hz with same y
            while i < self.hz_segments.len() && self.hz_segments[i].pos == y0 {
                let hz = &self.hz_segments[i];
                let fill = buffer.add_hz::<F>(hz);
                unsafe {
                    *self.hz_fills.get_unchecked_mut(i) = fill;
                }
                i += 1;
            }
        }

        while j < self.vr_segments.len() {
            let vr = &self.vr_segments[j];
            let (_, fill) = F::add_and_fill(vr.count, C::empty());
            unsafe {
                *self.vr_fills.get_unchecked_mut(j) = fill;
            }
            j += 1;
        }

        debug_assert_eq!(buffer.counts.len(), 1);
    }
}

impl<C: WindingCount> CountBuffer<C> {
    fn new(min: i32, max: i32) -> Self {
        let mut counts = Vec::with_capacity(16);
        // counts.push(Anchor { pos: min, count: C::new(i16::MAX, i16::MAX) });
        counts.push(Anchor {
            pos: max + 1,
            count: C::empty(),
        });
        Self { counts }
    }

    #[inline]
    fn add_hz<F: FillStrategy<C>>(&mut self, s: &OrthoSegment<C>) -> SegmentFill {
        // __a0____a1____a2
        match self.counts.binary_search_by(|a| a.pos.cmp(&s.min)) {
            Ok(i0) => {
                let pos = unsafe { self.counts.get_unchecked(i0 + 1).pos };
                if pos == s.max {
                    self.add_hz_11::<F>(s, i0)
                } else {
                    self.add_hz_10::<F>(s, i0)
                }
            }
            Err(i0) => {
                let pos = unsafe { self.counts.get_unchecked(i0).pos };
                if pos == s.max {
                    self.add_hz_01::<F>(s, i0)
                } else {
                    self.add_hz_00::<F>(s, i0)
                }
            }
        }
    }

    #[inline]
    fn add_hz_00<F: FillStrategy<C>>(&mut self, s: &OrthoSegment<C>, i0: usize) -> SegmentFill {
        // __c2____a0____c2(cx)____a1____c2____[a2]

        let c2 = unsafe { self.counts.get_unchecked(i0).count };
        let (cx, fill) = F::add_and_fill(s.count, c2);

        let a0 = Anchor {
            pos: s.min,
            count: c2,
        };
        let a1 = Anchor {
            pos: s.max,
            count: cx,
        };

        self.counts.splice(i0..i0, [a0, a1]);
        fill
    }

    #[inline]
    fn add_hz_01<F: FillStrategy<C>>(&mut self, s: &OrthoSegment<C>, i1: usize) -> SegmentFill {
        // __c1____a0____c1(cx)____[a1]____c2____[a2]

        let c1 = unsafe { self.counts.get_unchecked(i1).count };
        let c2 = unsafe { self.counts.get_unchecked(i1 + 1).count };

        let (cx, fill) = F::add_and_fill(s.count, c1);

        if cx == c2 {
            // move a1 to a0
            self.counts[i1].pos = s.min;
        } else {
            // add a0
            self.counts[i1].count = cx;
            let a0 = Anchor {
                pos: s.min,
                count: c1,
            };
            self.counts.insert(i1, a0);
        }

        fill
    }

    #[inline]
    fn add_hz_10<F: FillStrategy<C>>(&mut self, s: &OrthoSegment<C>, i0: usize) -> SegmentFill {
        // __c0____[a0]____c2(cx)____a1____c2____[a2]

        let c0 = unsafe { self.counts.get_unchecked(i0).count };
        let c2 = unsafe { self.counts.get_unchecked(i0 + 1).count };

        let (cx, fill) = F::add_and_fill(s.count, c2);

        if c0 == cx {
            // move a0 to a1
            self.counts[i0].pos = s.max;
        } else {
            // add a1
            let a1 = Anchor {
                pos: s.max,
                count: cx,
            };
            self.counts.insert(i0 + 1, a1);
        }

        fill
    }

    #[inline]
    fn add_hz_11<F: FillStrategy<C>>(&mut self, s: &OrthoSegment<C>, i0: usize) -> SegmentFill {
        // __c0____[a0]____c1(cx)____[a1]____c2____[a2]

        let c0 = unsafe { self.counts.get_unchecked(i0).count };
        let c1 = unsafe { self.counts.get_unchecked(i0 + 1).count };
        let c2 = unsafe { self.counts.get_unchecked(i0 + 2).count };

        let (cx, fill) = F::add_and_fill(s.count, c1);

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
    fn add_vr<F: FillStrategy<C>>(&self, s: &OrthoSegment<C>) -> SegmentFill {
        let index = match self.counts.binary_search_by(|a| a.pos.cmp(&s.pos)) {
            Ok(index) => index + 1,
            Err(index) => index,
        };

        let count = self.counts[index].count;
        let (_, fill) = F::add_and_fill(s.count, count);
        fill
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::boolean::winding_count::ShapeCountBoolean;
    use crate::ortho::column::Column;
    use crate::ortho::segment::OrthoSegment;
    use alloc::vec;

    #[test]
    fn test_0() {
        let mut column = Column {
            vr_segments: vec![
                OrthoSegment {
                    pos: 0,
                    min: 0,
                    max: 10,
                    count: ShapeCountBoolean { subj: -1, clip: 0 },
                },
                OrthoSegment {
                    pos: 10,
                    min: 0,
                    max: 10,
                    count: ShapeCountBoolean { subj: 1, clip: 0 },
                },
            ],
            hz_segments: vec![
                OrthoSegment {
                    pos: 10,
                    min: 0,
                    max: 10,
                    count: ShapeCountBoolean { subj: -1, clip: 0 },
                },
                OrthoSegment {
                    pos: 0,
                    min: 0,
                    max: 10,
                    count: ShapeCountBoolean { subj: 1, clip: 0 },
                },
            ],
            vr_fills: vec![],
            hz_fills: vec![],
            border_points: vec![],
            min: 0,
            max: 10,
            links_start: 0,
            links_count: 0,
        };

        assert_eq!(column.border_points.len(), 0);
    }
}
