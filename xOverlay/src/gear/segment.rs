use crate::graph::boolean::winding_count::ShapeCountBoolean;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct Segment {
    pub(crate) pos: i32,
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) dir: ShapeCountBoolean,
}