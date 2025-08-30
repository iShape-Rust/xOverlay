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
        for mut sub_graph in sub_graphs.into_iter() {
            links.append(&mut sub_graph.links);

            let ie = ends.len();

            for end in sub_graph.ends.iter_mut() {
                end.index += offset;
            }

            ends.append(&mut sub_graph.ends);

            if let (Some(e0), Some(e1)) = (ie.checked_sub(1).and_then(|j| ends.get(j)), ends.get(ie)) {
                if e0.point.x == e1.point.x {
                    let x = e0.point.x;

                    let mut left = ie - 1;
                    while left > 0 && ends[left - 1].point.x == x {
                        left -= 1;
                    }

                    let mut right = ie;
                    while right + 1 < ends.len() && ends[right + 1].point.x == x {
                        right += 1;
                    }

                    let range = left..(right + 1);
                    ends[range].sort_unstable_by(|a, b| a.point.y.cmp(&b.point.y));
                }
            }

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

