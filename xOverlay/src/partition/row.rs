use alloc::vec::Vec;
use crate::gear::segment::Segment;
use crate::geom::range::LineRange;

pub(crate) struct Row {
    pub(crate) range: LineRange,
    pub(crate) segments: Vec<Segment>,
}