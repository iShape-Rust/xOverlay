use crate::core::fill_rule::FillRule;
use crate::core::winding::WindingCount;
use crate::deg_90::column_map::Column;
use crate::fill::segment::{NONE, SegmentFill};
use crate::fill::strategy::{
    EvenOddStrategy, FillStrategy, NegativeStrategy, NonZeroStrategy, PositiveStrategy,
};
use crate::fill::winding_count::ShapeCountBoolean;
use crate::gear::segment::Segment;
use alloc::vec::Vec;
use core::mem::swap;

impl Column {
    pub(super) fn fill(&self, fill_rule: FillRule, fill_buffer: FillBuffer) -> Vec<SegmentFill> {
        match fill_rule {
            FillRule::EvenOdd => self.fill_with_strategy::<EvenOddStrategy>(fill_buffer),
            FillRule::NonZero => self.fill_with_strategy::<NonZeroStrategy>(fill_buffer),
            FillRule::Positive => self.fill_with_strategy::<PositiveStrategy>(fill_buffer),
            FillRule::Negative => self.fill_with_strategy::<NegativeStrategy>(fill_buffer),
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Anchor {
    x: i32,
    count: ShapeCountBoolean,
}

impl Anchor {
    #[inline(always)]
    fn add(&self, count: ShapeCountBoolean) -> Self {
        Self {
            x: self.x,
            count: self.count + count,
        }
    }
}

struct FillBuffer {
    active: Vec<Anchor>,
    buffer: Vec<Anchor>,
}

impl FillBuffer {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            active: Vec::with_capacity(capacity),
            buffer: Vec::with_capacity(capacity),
        }
    }

    fn add_segments<F: FillStrategy<ShapeCountBoolean>>(
        &mut self,
        segments: &[Segment],
        output: &mut Vec<SegmentFill>,
    ) {
        // segments are always sorted by range.min and not overlap each other
        // s(i).range.max <= s(i+1).range.min
        // segment.count.is_not_empty() === true
        // the segments not more < 100..200 elements
        // the buffers in average much less than 1000 and close to segments.len

        let mut iter = self.active.iter();
        let mut anchor = if let Some(first) = iter.next() {
            *first
        } else {
            self.active.init::<F>(segments, output);
            return;
        };

        self.buffer.clear();

        for (i, s) in segments.iter().enumerate() {
            while anchor.x <= s.range.min {
                self.buffer.add_or_merge(anchor);
                if let Some(next) = iter.next() {
                    anchor = *next;
                } else {
                    self.buffer.join(&segments[i..], output);
                    self.buffer.remove_last_if_empty();
                    swap(&mut self.active, &mut self.buffer);
                    return;
                }
            }

            self.buffer.add_or_merge(Anchor {
                x: s.range.min,
                count: anchor.count,
            });

            while anchor.x <= s.range.max {
                self.buffer.add_or_merge(anchor.add(s.count));
                if let Some(next) = iter.next() {
                    anchor = *next;
                } else {
                    let fill = if anchor.x == s.range.max {
                        let (top, fill) = F::add_and_fill(s.count, anchor.count);
                        self.buffer.last_mut().unwrap().count = top;
                        fill
                    } else {
                        let (top, fill) = F::add_and_fill(s.count, ShapeCountBoolean::empty());
                        self.buffer.add_or_merge(Anchor {
                            x: s.range.max,
                            count: top,
                        });
                        fill
                    };

                    output.push(fill);
                    let i1 = i + 1;
                    if i1 < segments.len() {
                        self.buffer.join(&segments[i..], output);
                    }

                    self.buffer.remove_last_if_empty();
                    swap(&mut self.active, &mut self.buffer);
                    return;
                }
            }

            // s.range.max < anchor.x
            let (top, fill) = F::add_and_fill(s.count, anchor.count);
            self.buffer.add_or_merge(Anchor {
                x: s.range.max,
                count: top,
            });
            output.push(fill);
        }

        self.buffer.add_or_merge(anchor);
        self.buffer.extend(iter);
        self.buffer.remove_last_if_empty();
        swap(&mut self.active, &mut self.buffer);
    }
}

trait AnchorBuffer {
    fn add_or_merge(&mut self, anchor: Anchor);
    fn remove_last_if_empty(&mut self);
    fn join(&mut self, segments: &[Segment], output: &mut Vec<SegmentFill>);
    fn init<F: FillStrategy<ShapeCountBoolean>>(&mut self, segments: &[Segment], output: &mut Vec<SegmentFill>);
}

impl AnchorBuffer for Vec<Anchor> {
    #[inline(always)]
    fn add_or_merge(&mut self, anchor: Anchor) {
        if let Some(last) = self.last_mut() {
            if last.count == anchor.count {
                last.x = anchor.x;
                return;
            }
        }
        self.push(anchor);
    }

    #[inline(always)]
    fn remove_last_if_empty(&mut self) {
        if let Some(last) = self.last() {
            if last.count.is_empty() {
                self.pop();
            }
        }
    }

    #[inline]
    fn join(&mut self, segments: &[Segment], output: &mut Vec<SegmentFill>) {
        for s in segments {
            self.push(Anchor {
                x: s.range.min,
                count: ShapeCountBoolean::empty(),
            });
            self.push(Anchor {
                x: s.range.max,
                count: s.count,
            });
            output.push(NONE);
        }
    }
    #[inline]
    fn init<F: FillStrategy<ShapeCountBoolean>>(&mut self, segments: &[Segment], output: &mut Vec<SegmentFill>) {
        self.clear();
        let capacity = 2 * segments.len() + 1;
        self.reserve(capacity);
        let mut x0 = i32::MAX;
        for s in segments {
            debug_assert!(s.count.is_not_empty());
            let a1 = Anchor {
                x: s.range.max,
                count: s.count,
            };

            if s.range.min != x0 {
                let a0 = Anchor {
                    x: s.range.min,
                    count: ShapeCountBoolean::empty(),
                };
                self.push(a0);
                self.push(a1);
            } else {
                self.add_or_merge(a1);
            }

            output.push(F::fill(s.count, ShapeCountBoolean::empty()));

            x0 = s.range.max;
        }

        debug_assert!(self.len() <= capacity);
        self.remove_last_if_empty();
    }
}

#[cfg(test)]
mod tests {
    use crate::core::winding::WindingCount;
    use crate::deg_90::fill::{Anchor, FillBuffer};
    use crate::fill::segment::{SUBJ_BOTH, SUBJ_BOTTOM, SUBJ_TOP};
    use crate::fill::strategy::NonZeroStrategy;
    use crate::fill::winding_count::ShapeCountBoolean;
    use crate::gear::segment::Segment;
    use crate::geom::range::LineRange;
    use alloc::vec::Vec;

    #[test]
    fn test_0() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 4),
            count: ShapeCountBoolean::new(1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line, &mut output);

        assert_eq!(line.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
    }

    #[test]
    fn test_1() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line_0 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 4),
            count: ShapeCountBoolean::new(1, 0),
        }];
        let line_1 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 4),
            count: ShapeCountBoolean::new(-1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line_0, &mut output);
        buffer.add_segments::<NonZeroStrategy>(&line_1, &mut output);

        assert!(buffer.active.is_empty());
        assert_eq!(line_0.len() + line_1.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_BOTTOM);
    }

    #[test]
    fn test_2() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line_0 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(1, 7),
            count: ShapeCountBoolean::new(1, 0),
        }];
        let line_1 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 6),
            count: ShapeCountBoolean::new(1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line_0, &mut output);
        buffer.add_segments::<NonZeroStrategy>(&line_1, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 1,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 2,
                    count: ShapeCountBoolean::new(1, 0)
                },
                Anchor {
                    x: 6,
                    count: ShapeCountBoolean::new(2, 0)
                },
                Anchor {
                    x: 7,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line_0.len() + line_1.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_BOTH);
    }

    #[test]
    fn test_3() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line_0 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(1, 7),
            count: ShapeCountBoolean::new(1, 0),
        }];
        let line_1 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 6),
            count: ShapeCountBoolean::new(-1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line_0, &mut output);
        buffer.add_segments::<NonZeroStrategy>(&line_1, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 1,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 2,
                    count: ShapeCountBoolean::new(1, 0)
                },
                Anchor {
                    x: 6,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 7,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line_0.len() + line_1.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_BOTTOM);
    }

    #[test]
    fn test_4() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line = [
            Segment {
                pos: 0,
                range: LineRange::with_min_max(1, 2),
                count: ShapeCountBoolean::new(1, 0),
            },
            Segment {
                pos: 0,
                range: LineRange::with_min_max(3, 4),
                count: ShapeCountBoolean::new(1, 0),
            },
        ];

        buffer.add_segments::<NonZeroStrategy>(&line, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 1,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 2,
                    count: ShapeCountBoolean::new(1, 0)
                },
                Anchor {
                    x: 3,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 4,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_TOP);
    }

    #[test]
    fn test_5() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line = [
            Segment {
                pos: 0,
                range: LineRange::with_min_max(0, 4),
                count: ShapeCountBoolean::new(1, 0),
            },
            Segment {
                pos: 0,
                range: LineRange::with_min_max(4, 8),
                count: ShapeCountBoolean::new(1, 0),
            },
        ];

        buffer.add_segments::<NonZeroStrategy>(&line, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 0,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 8,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_TOP);
    }
}
