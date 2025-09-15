use crate::core::options::IntOverlayOptions;
use crate::gear::sub_graph::FilledSegment;
use crate::geom::id_point::IdPoint;
use crate::graph::OverlayGraph;
use crate::graph::end::End;
use crate::graph::link::OverlayLink;
use alloc::vec::Vec;
use i_key_sort::sort::two_keys::TwoKeysSort;

impl OverlayGraph {
    pub(super) fn new(
        parallel: bool,
        groups: Vec<Vec<FilledSegment>>,
        options: IntOverlayOptions,
    ) -> Self {
        let count = groups.iter().fold(0, |s, g| s + g.len());
        let mut filled_segments = Vec::with_capacity(count);

        for mut group in groups {
            filled_segments.append(&mut group);
        }

        // filled_segments.sort_by_two_keys(parallel, |s| s.a.x, |s| s.a.y);

        let mut links = Vec::with_capacity(count);
        let mut ends = Vec::with_capacity(count);

        for (index, s) in filled_segments.iter().enumerate() {
            let link = OverlayLink::new(IdPoint::new(0, s.a), IdPoint::new(0, s.b), s.fill);
            let end = End { index, point: s.b };
            links.push(link);
            ends.push(end);
        }

        ends.sort_by_two_keys(parallel, |e| e.point.x, |e| e.point.y);

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
