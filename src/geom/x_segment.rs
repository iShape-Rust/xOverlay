use i_float::int::point::IntPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct XSegment {
    pub(crate) a: IntPoint,
    pub(crate) b: IntPoint,
}