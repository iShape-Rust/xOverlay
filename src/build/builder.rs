use alloc::vec::Vec;
use crate::build::boolean::ShapeCountBoolean;
use crate::build::end::End;
use crate::core::fill::SegmentFill;
use crate::core::link::OverlayLink;
use crate::ortho::overlay::OrthoOverlay;

pub(crate) trait GraphNode {
    fn with_indices(indices: &[usize]) -> Self;
}

pub(crate) struct GraphBuilder<N> {
    pub(super) links: Vec<OverlayLink>,
    pub(super) nodes: Vec<N>,
    pub(super) fills: Vec<SegmentFill>,
    pub(super) ends: Vec<End>,
}

impl<N: GraphNode> Default for GraphBuilder<N> {
    fn default() -> Self {
        Self {
            links: Vec::new(),
            nodes: Vec::new(),
            fills: Vec::new(),
            ends: Vec::new(),
        }
    }
}

impl<N: GraphNode> GraphBuilder<N> {

    #[inline]
    pub(crate) fn build_with_ortho(&mut self, overlay: &OrthoOverlay<ShapeCountBoolean>) {


    }
}


