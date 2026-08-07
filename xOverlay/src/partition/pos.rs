use crate::definition::winding_count::ShapeCountBoolean;
use i_float::int::number::int::IntNumber;

pub(super) struct Pos<I: IntNumber> {
    pub(super) x: I,
    pub(super) count: ShapeCountBoolean,
}
