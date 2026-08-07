use crate::core::integer::OverlayInt;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::column::count::LRCount;
use core::num::NonZeroU32;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct Anchor<I: OverlayInt> {
    pub(super) node: Option<NonZeroU32>,
    pub(super) x: I,
    pub(super) count: LRCount,
}

impl<I: OverlayInt> Anchor<I> {
    #[inline(always)]
    pub(super) fn new(
        x: I,
        node: Option<NonZeroU32>,
        c0: ShapeCountBoolean,
        c1: ShapeCountBoolean,
    ) -> Self {
        Self {
            x,
            node,
            count: LRCount {
                left: c0,
                right: c1,
            },
        }
    }
}
