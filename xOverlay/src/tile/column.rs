use crate::tile::source::GeometrySource;
use i_float::int::rect::IntRect;

pub(crate) struct Column {
    pub(crate) rect: IntRect,
    pub(crate) source: GeometrySource,
}
