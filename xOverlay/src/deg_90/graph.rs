use crate::deg_90::merge::Merge;
use crate::deg_90::sub_graph::SubGraph;
use crate::graph::data::OverlayGraph;
use alloc::vec;
use alloc::vec::Vec;

impl OverlayGraph {
    pub(super) fn with_sub_graphs(sub_graphs: Vec<SubGraph>) -> Self {
        Self::with_sub_graph(sub_graphs.merge())
    }

    pub(super) fn with_sub_graph(sub_graphs: SubGraph) -> Self {
        Self {
            options: Default::default(),
            nodes: vec![],
            links: vec![],
            fills: vec![],
        }
    }
}
