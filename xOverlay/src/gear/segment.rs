use crate::geom::range::LineRange;
use crate::gear::winding_count::ShapeCountBoolean;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct Segment {
    pub(crate) pos: i32, // for vr -> x, hz -> y, dg -> min y
    pub(crate) range: LineRange,
    pub(crate) dir: ShapeCountBoolean,
}