use crate::core::winding::WindingCount;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;
use i_float::int::number::int::IntNumber;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct Segment<I: IntNumber, W: WindingCount = i16> {
    pub(crate) pos: I,
    pub(crate) range: LineRange<I>,
    pub(crate) count: ShapeCountBoolean<W>,
}
