use crate::definition::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct Segment {
    pub(crate) pos: i32,
    pub(crate) range: LineRange,
    pub(crate) count: ShapeCountBoolean,
}
