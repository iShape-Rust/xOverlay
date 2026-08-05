use crate::core::fill_rule::FillRule;
use crate::core::overlay::Overlay;
use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column::SolverBuffer;
use crate::deg_90::column_map::{Column, ColumnMap};
use crate::deg_90::config::ColumnConfig90;
use crate::deg_90::merge::Merge;
#[cfg(feature = "allow_multithreading")]
use crate::deg_90::merge::ParallelMerge;
use crate::deg_90::sub_graph::SubGraph;
use crate::graph::data::OverlayGraph;
use crate::partition::solver::Partition;
use alloc::vec::Vec;
use i_shape::int::shape::IntContour;
#[cfg(feature = "allow_multithreading")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

impl Overlay {
    pub(crate) fn process_overlay(
        self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> OverlayGraph {
        let options = self.options;
        OverlayGraph::with_sub_graph(self.process_sub_graph(fill_rule, overlay_rule), options)
    }

    pub(crate) fn process_overlay_contours(
        self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> Vec<IntContour<i32>> {
        self.process_sub_graph(fill_rule, overlay_rule)
            .into_contours()
    }

    fn process_sub_graph(self, fill_rule: FillRule, overlay_rule: OverlayRule) -> SubGraph {
        #[cfg(feature = "allow_multithreading")]
        {
            if self.cpu_count.is_parallel() {
                return self.parallel_process(fill_rule, overlay_rule);
            }
        }

        self.serial_process(fill_rule, overlay_rule)
    }

    fn serial_process(self, fill_rule: FillRule, overlay_rule: OverlayRule) -> SubGraph {
        let config = self.options.columns_config;
        let mut buffer = SolverBuffer::default();
        let sub_graphs: Vec<_> = self
            .columns
            .into_iter()
            .map(|c| c.process(fill_rule, overlay_rule, config, &mut buffer))
            .collect();

        sub_graphs.merge()
    }

    #[cfg(feature = "allow_multithreading")]
    fn parallel_process(self, fill_rule: FillRule, overlay_rule: OverlayRule) -> SubGraph {
        let config = self.options.columns_config;
        let sub_graphs: Vec<_> = self
            .columns
            .into_par_iter()
            .map_init(SolverBuffer::default, |buffer, c| {
                c.process(fill_rule, overlay_rule, config, buffer)
            })
            .collect();

        sub_graphs.parallel_merge()
    }
}

impl Column {
    fn process(
        mut self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
        config: ColumnConfig90,
        buffer: &mut SolverBuffer,
    ) -> SubGraph {
        if let Some(columns) = self.partition(config) {
            let sub_graphs: Vec<_> = columns
                .into_iter()
                .map(|column| SubGraph::with_column_buffer(column, fill_rule, overlay_rule, buffer))
                .collect();
            sub_graphs.merge()
        } else {
            SubGraph::with_column_buffer(self, fill_rule, overlay_rule, buffer)
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
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay_rule::OverlayRule;
    use crate::deg_90::column::SolverBuffer;
    use crate::deg_90::column_map::Column;
    use crate::deg_90::config::ColumnConfig90;
    use crate::deg_90::sub_graph::SubGraph;

    impl Column {
        pub(crate) fn test_partition(&mut self, config: ColumnConfig90) {
            let result = self.partition(config);
            debug_assert!(result.is_none());
        }

        pub(in crate::deg_90) fn test_process(
            self,
            fill_rule: FillRule,
            overlay_rule: OverlayRule,
            config: ColumnConfig90,
            buffer: &mut SolverBuffer,
        ) -> SubGraph {
            self.process(fill_rule, overlay_rule, config, buffer)
        }
    }
}
