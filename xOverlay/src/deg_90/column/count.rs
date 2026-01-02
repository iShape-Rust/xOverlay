use crate::definition::winding_count::ShapeCountBoolean;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) struct LRCount {
    pub(super) left: ShapeCountBoolean,
    pub(super) right: ShapeCountBoolean,
}

impl LRCount {
    #[inline(always)]
    pub(super) fn new(left: ShapeCountBoolean, right: ShapeCountBoolean) -> Self {
        Self { left, right }
    }
}
