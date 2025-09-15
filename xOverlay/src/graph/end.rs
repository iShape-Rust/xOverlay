use i_float::int::point::IntPoint;

#[derive(Clone, Copy)]
pub(crate) struct End {
    pub(crate) index: usize,
    pub(crate) point: IntPoint,
}