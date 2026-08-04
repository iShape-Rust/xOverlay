use crate::core::options::IntOverlayOptions;
use crate::definition::segment::SegmentFill;
use alloc::vec::Vec;
use core::ops::Range;
use i_float::int::point::IntPoint;

pub struct OverlayGraph {
    pub(crate) options: IntOverlayOptions,
    pub(crate) nodes: Vec<OverlayNode>,
    pub(crate) links: Vec<OverlayLink>,
    pub(crate) fills: Vec<SegmentFill>,
}

pub(crate) struct OverlayNode {
    pub(crate) point: IntPoint,
    pub(crate) links: Range<usize>, // indices to a links
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OverlayLink {
    pub(crate) segm_index: usize,
    pub(crate) node_index: usize,
}

impl OverlayLink {
    #[inline(always)]
    pub(crate) fn with_segm_and_node(segm_index: usize, node_index: usize) -> Self {
        OverlayLink {
            segm_index,
            node_index,
        }
    }
}
