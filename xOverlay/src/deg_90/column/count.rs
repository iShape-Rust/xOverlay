use crate::core::winding::WindingCount;
use crate::definition::winding_count::ShapeCountBoolean;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct LRCount<W: WindingCount = i16> {
    pub(super) left: ShapeCountBoolean<W>,
    pub(super) right: ShapeCountBoolean<W>,
}

impl<W: WindingCount> LRCount<W> {
    #[inline(always)]
    pub(super) fn new(left: ShapeCountBoolean<W>, right: ShapeCountBoolean<W>) -> Self {
        Self { left, right }
    }
}
