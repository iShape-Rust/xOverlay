use crate::core::fill_rule::FillRule;
use crate::core::overlay::Overlay;
use crate::core::overlay_rule::OverlayRule;
use crate::gear::fill_buffer::FillBuffer;
use crate::gear::section::Section;
use crate::gear::sub_graph::SubGraph;
use crate::graph::OverlayGraph;
use alloc::vec::Vec;


impl Overlay {
    pub(crate) fn process_overlay(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        if self.solver.cpu_count() == 1 {
            self.serial_process(fill_rule, overlay_rule)
        } else {
            self.parallel_process(fill_rule, overlay_rule)
        }
    }

    fn serial_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let mut sub_graphs = Vec::with_capacity(self.sections.len());
        for s in self.sections.iter_mut() {
            let sub_graph = s.process(fill_rule, overlay_rule);
            sub_graphs.push(sub_graph);
        }
        OverlayGraph::new(sub_graphs, self.options)
    }

    fn parallel_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let sub_graphs: Vec<_> = self
            .sections
            .iter_mut()
            .map(|s| s.process(fill_rule, overlay_rule))
            .collect();
        OverlayGraph::new(sub_graphs, self.options)
    }
}

impl Section {
    fn process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> SubGraph {
        // split by columns

        let mut source_by_columns = self.source.new_same_size();
        let mut map_by_columns = self
            .source
            .map_by_columns(&self.layout, &mut source_by_columns);

        // intersect

        let mut split_buffer = self.intersect(&mut source_by_columns, &map_by_columns);

        let any_split = !split_buffer.is_empty();

        if any_split {
            self.split_by_marks(&mut source_by_columns, &mut split_buffer);
            map_by_columns = source_by_columns.map_by_columns(&self.layout, &mut self.source);
        }

        let any_merge = self.sort_and_merge(&map_by_columns);

        if any_split || any_merge {
            self.source.init_map(&mut map_by_columns);
        }

        let x_range = map_by_columns.layout.x_range();
        let fill_source = self.fill(fill_rule, FillBuffer::new(split_buffer), map_by_columns);

        self.sub_graph(overlay_rule, fill_source, x_range)
    }
}
