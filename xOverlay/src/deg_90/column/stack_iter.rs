use crate::core::integer::OverlayInt;
use crate::core::winding::WindingCount;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::column::anchor::Anchor;
use crate::deg_90::column::segment_iter::{
    SegmentEnd, SegmentSplitPointIter, SegmentSplitPointIterator,
};
use crate::geom::segment::Segment;
use core::cmp::Ordering;
use core::num::NonZeroU32;
use core::slice::Iter;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct StackPoint<I: OverlayInt> {
    pub(super) x: I,
    pub(super) node: Option<NonZeroU32>,
    pub(super) c0: ShapeCountBoolean,
    pub(super) c1: ShapeCountBoolean,
    pub(super) cb: ShapeCountBoolean,
}

pub(super) struct StackIter<'a, I: OverlayInt> {
    split_iter: SegmentSplitPointIter<'a, I>,
    anchors_iter: Iter<'a, Anchor<I>>,
    split: Option<SegmentEnd<I>>,
    anchor: Option<&'a Anchor<I>>,
}

impl<'a, I: OverlayInt> StackIter<'a, I> {
    pub(super) fn new(segments: &'a [Segment<I>], anchors: &'a [Anchor<I>]) -> Self {
        let mut segments_iter = segments.split_point_iter();
        let split = segments_iter.next();

        let mut anchors_iter = anchors.iter();
        let anchor = anchors_iter.next();

        StackIter {
            split_iter: segments_iter,
            anchors_iter,
            split,
            anchor,
        }
    }
}

impl<I: OverlayInt> Iterator for StackIter<'_, I> {
    type Item = StackPoint<I>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match (self.split, self.anchor) {
            (Some(s), Some(a)) => {
                let sp = match s.x.cmp(&a.x) {
                    Ordering::Equal => {
                        let c0 = s.count.left + a.count.left;
                        let c1 = s.count.right + a.count.right;
                        let cb = a.count.right;

                        self.split = self.split_iter.next();
                        self.anchor = self.anchors_iter.next();

                        StackPoint {
                            x: s.x,
                            node: a.node,
                            c0,
                            c1,
                            cb,
                        }
                    }
                    Ordering::Less => {
                        // s < a
                        let c0 = s.count.left + a.count.left;
                        let c1 = s.count.right + a.count.left;
                        let cb = a.count.left;

                        self.split = self.split_iter.next();

                        StackPoint {
                            x: s.x,
                            node: None,
                            c0,
                            c1,
                            cb,
                        }
                    }
                    Ordering::Greater => {
                        // a < s
                        let c0 = s.count.left + a.count.left;
                        let c1 = s.count.left + a.count.right;
                        let cb = a.count.right;

                        self.anchor = self.anchors_iter.next();

                        StackPoint {
                            x: a.x,
                            node: a.node,
                            c0,
                            c1,
                            cb,
                        }
                    }
                };

                Some(sp)
            }
            (Some(s), None) => {
                self.split = self.split_iter.next();

                let c0 = s.count.left;
                let c1 = s.count.right;
                let cb = ShapeCountBoolean::empty();

                Some(StackPoint {
                    x: s.x,
                    node: None,
                    c0,
                    c1,
                    cb,
                })
            }
            (None, Some(a)) => {
                self.anchor = self.anchors_iter.next();
                let c0 = a.count.left;
                let c1 = a.count.right;
                let cb = a.count.right;

                Some(StackPoint {
                    x: a.x,
                    node: a.node,
                    c0,
                    c1,
                    cb,
                })
            }
            (None, None) => None,
        }
    }
}
