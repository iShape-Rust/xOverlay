use core::num::NonZeroU32;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::column::count::LRCount;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct Anchor {
    pub(super) node: Option<NonZeroU32>,
    pub(super) x: i32,
    pub(super) count: LRCount,
}

impl Anchor {
    #[inline(always)]
    pub(super) fn new(x: i32, node: Option<NonZeroU32>, c0: ShapeCountBoolean, c1: ShapeCountBoolean) -> Self {
        Self {
            x,
            node,
            count: LRCount { left: c0, right: c1 },
        }
    }
}