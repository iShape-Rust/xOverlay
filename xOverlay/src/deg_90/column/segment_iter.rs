use crate::core::integer::OverlayInt;
use crate::core::winding::WindingCount;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::column::count::LRCount;
use crate::geom::segment::Segment;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct SegmentEnd<I: OverlayInt, W: WindingCount = i16> {
    pub(super) x: I,
    pub(super) count: LRCount<W>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum End {
    Max,
    Min,
}

pub(super) struct SegmentSplitPointIter<'a, I: OverlayInt, W: WindingCount = i16> {
    segments: &'a [Segment<I, W>],
    index: usize,
    active_end: End,
    last_next: ShapeCountBoolean<W>,
}

impl<'a, I: OverlayInt, W: WindingCount> SegmentSplitPointIter<'a, I, W> {
    pub(super) fn new(segments: &'a [Segment<I, W>]) -> Self {
        debug_assert!(!segments.is_empty());
        SegmentSplitPointIter {
            segments,
            active_end: End::Min,
            last_next: ShapeCountBoolean::empty(),
            index: 0,
        }
    }
}

impl<I: OverlayInt, W: WindingCount> Iterator for SegmentSplitPointIter<'_, I, W> {
    type Item = SegmentEnd<I, W>;

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
                    let next = if si.range.max == sn.range.min {
                        sn.count
                    } else {
                        self.active_end = End::Min;
                        ShapeCountBoolean::empty()
                    };

                    let pt = SegmentEnd {
                        x: si.range.max,
                        count: LRCount::new(self.last_next, next),
                    };

                    self.last_next = next;

                    Some(pt)
                } else {
                    Some(SegmentEnd {
                        x: si.range.max,
                        count: LRCount::new(self.last_next, ShapeCountBoolean::empty()),
                    })
                }
            }

            End::Min => {
                self.active_end = End::Max;

                let se = SegmentEnd {
                    x: si.range.min,
                    count: LRCount::new(self.last_next, si.count),
                };

                self.last_next = si.count;

                Some(se)
            }
        }
    }
}

pub(super) trait SegmentSplitPointIterator<I: OverlayInt, W: WindingCount> {
    fn split_point_iter(&'_ self) -> SegmentSplitPointIter<'_, I, W>;
}

impl<I: OverlayInt, W: WindingCount> SegmentSplitPointIterator<I, W> for [Segment<I, W>] {
    fn split_point_iter(&'_ self) -> SegmentSplitPointIter<'_, I, W> {
        SegmentSplitPointIter::new(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::definition::winding_count::ShapeCountBoolean;
    use crate::deg_90::column::count::LRCount;
    use crate::deg_90::column::segment_iter::{SegmentEnd, SegmentSplitPointIterator};
    use crate::geom::range::LineRange;
    use crate::geom::segment::Segment;
    #[test]
    fn test_0() {
        let segments = [Segment {
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
            }
        );

        assert_eq!(
            s1,
            SegmentEnd {
                x: 2,
                count: LRCount::new(ShapeCountBoolean::subj(1), ShapeCountBoolean::subj(0)),
            }
        );
    }

    #[test]
    fn test_1() {
        let segments = [
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
            }
        );

        assert_eq!(
            s1,
            SegmentEnd {
                x: 2,
                count: LRCount::new(ShapeCountBoolean::subj(1), ShapeCountBoolean::subj(0)),
            }
        );

        assert_eq!(
            s2,
            SegmentEnd {
                x: 3,
                count: LRCount::new(ShapeCountBoolean::subj(0), ShapeCountBoolean::subj(2)),
            }
        );

        assert_eq!(
            s3,
            SegmentEnd {
                x: 4,
                count: LRCount::new(ShapeCountBoolean::subj(2), ShapeCountBoolean::subj(3)),
            }
        );

        assert_eq!(
            s4,
            SegmentEnd {
                x: 5,
                count: LRCount::new(ShapeCountBoolean::subj(3), ShapeCountBoolean::subj(0)),
            }
        );

        assert_eq!(
            s5,
            SegmentEnd {
                x: 6,
                count: LRCount::new(ShapeCountBoolean::subj(0), ShapeCountBoolean::subj(4)),
            }
        );

        assert_eq!(
            s6,
            SegmentEnd {
                x: 7,
                count: LRCount::new(ShapeCountBoolean::subj(4), ShapeCountBoolean::subj(0)),
            }
        );
    }
}
