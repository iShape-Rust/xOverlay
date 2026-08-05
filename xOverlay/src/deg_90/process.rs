use crate::core::fill_rule::FillRule;
use crate::core::overlay::Overlay;
use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column_map::{Column, ColumnMap};
use crate::deg_90::config::ColumnConfig90;
use crate::deg_90::merge::Merge;
use crate::deg_90::sub_graph::SubGraph;
use crate::graph::data::OverlayGraph;
use crate::partition::solver::Partition;
use alloc::vec::Vec;
#[cfg(feature = "allow_multithreading")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

impl Overlay {
    pub(crate) fn process_overlay(
        self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> OverlayGraph {
        #[cfg(feature = "allow_multithreading")]
        {
            if self.cpu_count.is_parallel() {
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

        OverlayGraph::with_sub_graphs(sub_graphs, self.options)
    }

    #[cfg(feature = "allow_multithreading")]
    fn parallel_process(self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let sub_graphs: Vec<_> = self
            .columns
            .into_par_iter()
            .map(|c| c.process(fill_rule, overlay_rule, self.options.columns_config))
            .collect();

        OverlayGraph::with_sub_graphs(sub_graphs, self.options)
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
                .map(|column| SubGraph::with_column(column, fill_rule, overlay_rule))
                .collect();
            sub_graphs.merge()
        } else {
            SubGraph::with_column(self, fill_rule, overlay_rule)
        }
    }

    fn partition(&mut self, config: ColumnConfig90) -> Option<Vec<Column>> {
        let max_segments_in_line = self.segments.partition();
        let map =
            ColumnMap::with_segments(&self.segments, max_segments_in_line, self.range, config)?;
        Some(map.columns)
    }
}
#[cfg(test)]
mod tests {
    use crate::deg_90::column_map::Column;
    use crate::deg_90::config::ColumnConfig90;

    impl Column {
        pub(crate) fn test_partition(&mut self, config: ColumnConfig90) {
            let result = self.partition(config);
            debug_assert!(result.is_none());
        }
    }
}
