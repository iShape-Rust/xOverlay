use alloc::vec::Vec;
use crate::deg_90::sub_graph::SubGraph;

pub(super) trait Merge {
    fn merge(self) -> SubGraph;
}

impl Merge for Vec<SubGraph> {
    fn merge(self) -> SubGraph {
        SubGraph {}
    }
}