use alloc::vec::Vec;
use crate::core::options::IntOverlayOptions;
use crate::gear::sub_graph::SubGraph;
use crate::graph::OverlayGraph;

impl OverlayGraph {

    pub(super) fn new(sub_graphs: Vec<SubGraph>, options: IntOverlayOptions) -> Self {
        let mut total_links = 0;
        let mut total_ends = 0;
        for s in sub_graphs.iter() {
            total_links += s.links.len();
            total_ends += s.ends.len();
        }

        let mut links = Vec::with_capacity(total_links);
        let mut ends = Vec::with_capacity(total_ends);

        let mut offset = 0;
        for mut s in sub_graphs.into_iter() {
            links.append(&mut s.links);

            for end in s.ends.iter_mut() {
                end.index += offset;
            }

            ends.append(&mut s.ends);

            offset = links.len();
        }

        let mut graph = Self {
            options,
            nodes: Vec::new(),
            links,
            ends,
            buffer: None,
        };

        graph.build();

        graph
    }

}

