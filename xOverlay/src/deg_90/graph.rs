use crate::core::options::IntOverlayOptions;
use crate::deg_90::merge::Merge;
use crate::deg_90::sub_graph::SubGraph;
use crate::graph::data::OverlayGraph;
use alloc::vec;
use alloc::vec::Vec;

impl OverlayGraph {
    pub(super) fn with_sub_graphs(sub_graphs: Vec<SubGraph>, options: IntOverlayOptions) -> Self {
        Self::with_sub_graph(sub_graphs.merge(), options)
    }

    pub(super) fn with_sub_graph(sub_graph: SubGraph, options: IntOverlayOptions) -> Self {
        Self {
            options,
            nodes: vec![],
            links: vec![],
            fills: vec![],
            shapes: sub_graph.into_shapes(),
        }
    }
}
