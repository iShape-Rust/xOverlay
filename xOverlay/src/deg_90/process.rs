use crate::core::fill_rule::FillRule;
use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column_map::{Column, ColumnMap};
use crate::deg_90::config::ColumnConfig90;
use crate::deg_90::merge::Merge;
use crate::deg_90::overlay::Overlay90;
use crate::deg_90::sub_graph::SubGraph;
use crate::graph::data::OverlayGraph;
use crate::partition::solver::Partition;
use alloc::vec::Vec;

impl Overlay90 {
    pub(crate) fn process_overlay(
        self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> OverlayGraph {
        #[cfg(feature = "allow_multithreading")]
        {
            if self.cpus.is_parallel() {
                return self.parallel_process(fill_rule, overlay_rule);
            }
        }

        self.serial_process(fill_rule, overlay_rule)
    }

    fn serial_process(self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let sub_graphs: Vec<_> = self
            .columns
            .into_iter()
            .map(|c| c.process(fill_rule, overlay_rule, self.options.columns_config))
            .collect();

        OverlayGraph::with_sub_graphs(sub_graphs)
    }

    #[cfg(feature = "allow_multithreading")]
    fn parallel_process(self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let sub_graphs: Vec<_> = self
            .columns
            .into_iter()
            .map(|c| c.process(fill_rule, overlay_rule, self.options.columns_config))
            .collect();

        OverlayGraph::with_sub_graphs(sub_graphs)
    }
}

impl Column {
    fn process(
        mut self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
        config: ColumnConfig90,
    ) -> SubGraph {
        if let Some(columns) = self.partition(config) {
            let sub_graphs: Vec<_> = columns
                .into_iter()
                .map(|column| SubGraph::with_column(column))
                .collect();
            sub_graphs.merge()
        } else {
            SubGraph::with_column(self)
        }
    }

    fn partition(&mut self, config: ColumnConfig90) -> Option<Vec<Column>> {
        let max_segments_in_line = self.segments.partition();
        let map =
            ColumnMap::with_segments(&self.segments, max_segments_in_line, self.range, config)?;
        Some(map.columns)
    }
}
