use alloc::vec::Vec;
use crate::core::fill::{FillStrategy, SegmentFill, NONE};
use crate::core::winding::WindingCount;
use crate::ortho::column::Column;
use crate::ortho::segment::OrthoSegment;

struct Anchor<C> {
    pos: i32,
    count: C
}

struct CountBuffer<C> {
    counts: Vec<Anchor<C>>
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
    }
}

impl<C: WindingCount> CountBuffer<C> {
    fn new(min: i32, max: i32) -> Self {
        let mut counts = Vec::with_capacity(16);
        // counts.push(Anchor { pos: min, count: C::new(i16::MAX, i16::MAX) });
        counts.push(Anchor { pos: max + 1, count: C::new(0, 0) });
        Self {
            counts
        }
    }

    #[inline]
    fn add_hz<F: FillStrategy<C>>(&mut self, s: &OrthoSegment<C>) -> SegmentFill {
        match self.counts.binary_search_by(|a|a.pos.cmp(&s.min)) {
            Ok(index) => {
                let (left_count, count) = unsafe {
                    let left_count = self.counts.get_unchecked(index).count;
                    let count = self.counts.get_unchecked(index + 1).count;

                    (left_count, count)
                };

                let (new_count, fill) = F::add_and_fill(s.count, count);

                let remove_left = new_count == left_count;
                let remove_right = if let Some(right_count) = self.counts.get(index + 2) {
                    right_count.count == new_count
                } else {
                    false
                };

                if remove_left && remove_right {
                    self.counts.drain(index..index + 2);
                } else if remove_left {
                    self.counts.remove(index);
                } else if remove_right {
                    self.counts.remove(index + 1);
                } else {
                    self.counts[index + 1].count = new_count;
                }

                fill
            }
            Err(index) => {
                let count = unsafe { self.counts.get_unchecked(index).count };
                let (new_count, fill) = F::add_and_fill(s.count, count);

                let a0 = Anchor { pos: s.min, count };
                let a1 = Anchor { pos: s.max, count: new_count };

                self.counts.splice(index..index, [a0, a1]);
                fill
            }
        }
    }

    #[inline]
    fn add_vr<F: FillStrategy<C>>(&self, s: &OrthoSegment<C>) -> SegmentFill {
        let index = self.counts.binary_search_by(|a|a.pos.cmp(&s.pos)).unwrap();
        let count = self.counts[index + 1].count;
        let (_, fill) = F::add_and_fill(s.count, count);
        fill
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use crate::graph::boolean::winding_count::ShapeCountBoolean;
    use crate::ortho::column::Column;
    use crate::ortho::segment::OrthoSegment;

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
                }
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
                }
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