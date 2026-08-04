use crate::core::winding::WindingCount;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::column::count::LRCount;
use crate::geom::segment::Segment;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct SegmentEnd {
    pub(super) x: i32,
    pub(super) max: i32,
    pub(super) count: LRCount,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum End {
    Max,
    Min,
}

pub(super) struct SegmentSplitPointIter<'a> {
    segments: &'a [Segment],
    index: usize,
    active_end: End,
    min: i32,
    last_next: ShapeCountBoolean,
}

impl<'a> SegmentSplitPointIter<'a> {
    pub(super) fn new(segments: &'a [Segment]) -> Self {
        debug_assert!(!segments.is_empty());
        SegmentSplitPointIter {
            segments,
            active_end: End::Min,
            last_next: ShapeCountBoolean::empty(),
            min: segments[0].range.min,
            index: 0,
        }
    }
}

impl<'a> Iterator for SegmentSplitPointIter<'a> {
    type Item = SegmentEnd;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.segments.len() {
            return None;
        }

        let si = self.segments[self.index];
        match self.active_end {
            End::Max => {
                self.index += 1;
                if self.index < self.segments.len() {
                    let sn = self.segments[self.index];
                    let (next, min, max) = if si.range.max == sn.range.min {
                        (sn.count, self.min, sn.range.max)
                    } else {
                        self.active_end = End::Min;
                        (ShapeCountBoolean::empty(), self.min, si.range.max)
                    };

                    let pt = SegmentEnd {
                        x: si.range.max,
                        max,
                        count: LRCount::new(self.last_next, next),
                    };

                    self.last_next = next;

                    Some(pt)
                } else {
                    Some(SegmentEnd {
                        x: si.range.max,
                        count: LRCount::new(self.last_next, ShapeCountBoolean::empty()),
                        max: si.range.max,
                    })
                }
            }

            End::Min => {
                self.active_end = End::Max;

                let se = SegmentEnd {
                    x: si.range.min,
                    max: si.range.max,
                    count: LRCount::new(self.last_next, si.count),
                };

                self.last_next = si.count;

                Some(se)
            }
        }
    }
}

pub(super) trait SegmentSplitPointIterator {
    fn split_point_iter(&'_ self) -> SegmentSplitPointIter<'_>;
}

impl SegmentSplitPointIterator for [Segment] {
    fn split_point_iter(&'_ self) -> SegmentSplitPointIter<'_> {
        SegmentSplitPointIter::new(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::winding::WindingCount;
    use crate::definition::winding_count::ShapeCountBoolean;
    use crate::deg_90::column::count::LRCount;
    use crate::deg_90::column::segment_iter::{SegmentEnd, SegmentSplitPointIterator};
    use crate::geom::segment::Segment;
    use crate::geom::range::LineRange;
    use alloc::vec;

    #[test]
    fn test_0() {
        let segments = vec![Segment {
            pos: 0,
            range: LineRange { min: 0, max: 2 },
            count: ShapeCountBoolean::new(1, 0),
        }];

        let mut iter = segments.split_point_iter();
        let s0 = iter.next().unwrap();
        let s1 = iter.next().unwrap();

        assert_eq!(
            s0,
            SegmentEnd {
                x: 0,
                count: LRCount::new(ShapeCountBoolean::subj(0), ShapeCountBoolean::subj(1)),
                max: 2,
            }
        );

        assert_eq!(
            s1,
            SegmentEnd {
                x: 2,
                count: LRCount::new(ShapeCountBoolean::subj(1), ShapeCountBoolean::subj(0)),
                max: 2,
            }
        );
    }

    #[test]
    fn test_1() {
        let segments = vec![
            Segment {
                pos: 0,
                range: LineRange { min: 0, max: 2 },
                count: ShapeCountBoolean::subj(1),
            },
            Segment {
                pos: 0,
                range: LineRange { min: 3, max: 4 },
                count: ShapeCountBoolean::subj(2),
            },
            Segment {
                pos: 0,
                range: LineRange { min: 4, max: 5 },
                count: ShapeCountBoolean::subj(3),
            },
            Segment {
                pos: 0,
                range: LineRange { min: 6, max: 7 },
                count: ShapeCountBoolean::subj(4),
            },
        ];

        let mut iter = segments.split_point_iter();

        let s0 = iter.next().unwrap();
        let s1 = iter.next().unwrap();
        let s2 = iter.next().unwrap();
        let s3 = iter.next().unwrap();
        let s4 = iter.next().unwrap();
        let s5 = iter.next().unwrap();
        let s6 = iter.next().unwrap();

        assert_eq!(
            s0,
            SegmentEnd {
                x: 0,
                count: LRCount::new(ShapeCountBoolean::subj(0), ShapeCountBoolean::subj(1)),
                max: 2,
            }
        );

        assert_eq!(
            s1,
            SegmentEnd {
                x: 2,
                count: LRCount::new(ShapeCountBoolean::subj(1), ShapeCountBoolean::subj(0)),
                max: 2,
            }
        );

        assert_eq!(
            s2,
            SegmentEnd {
                x: 3,
                count: LRCount::new(ShapeCountBoolean::subj(0), ShapeCountBoolean::subj(2)),
                max: 4,
            }
        );

        assert_eq!(
            s3,
            SegmentEnd {
                x: 4,
                count: LRCount::new(ShapeCountBoolean::subj(2), ShapeCountBoolean::subj(3)),
                max: 5,
            }
        );

        assert_eq!(
            s4,
            SegmentEnd {
                x: 5,
                count: LRCount::new(ShapeCountBoolean::subj(3), ShapeCountBoolean::subj(0)),
                max: 5,
            }
        );

        assert_eq!(
            s5,
            SegmentEnd {
                x: 6,
                count: LRCount::new(ShapeCountBoolean::subj(0), ShapeCountBoolean::subj(4)),
                max: 7,
            }
        );

        assert_eq!(
            s6,
            SegmentEnd {
                x: 7,
                count: LRCount::new(ShapeCountBoolean::subj(4), ShapeCountBoolean::subj(0)),
                max: 7,
            }
        );
    }
}
