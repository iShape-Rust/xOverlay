use crate::deg_90::sub_graph::SubGraph;
use alloc::vec::Vec;

pub(super) trait Merge {
    fn merge(self) -> SubGraph;
}

impl Merge for Vec<SubGraph> {
    fn merge(self) -> SubGraph {
        SubGraph {}
    }
}
