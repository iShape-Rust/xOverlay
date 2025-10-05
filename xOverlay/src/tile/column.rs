use crate::tile::source::GeometrySource;
use crate::geom::range::LineRange;

pub(crate) struct TileColumn {
    pub(crate) range: LineRange,
    pub(crate) source: GeometrySource,
}