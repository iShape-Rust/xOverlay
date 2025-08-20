use crate::core::fill_rule::FillRule;
use crate::core::overlay_rule::OverlayRule;
use crate::graph::boolean::winding_count::ShapeCountBoolean;
use crate::graph::end::End;
use crate::graph::link::OverlayLink;
use crate::ortho::overlay::OrthoOverlay;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use crate::graph::OverlayGraph;
use crate::ortho::column::Column;

impl OrthoOverlay<ShapeCountBoolean> {
    pub(crate) fn build_custom_graph(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) {
        let mut graph = self.graph.take().unwrap_or_default();
        if self.solver.multithreading && self.columns.len() > 4 {
            self.parallel_build_graph(&mut graph, fill_rule, overlay_rule);
        } else {
            self.serial_build_graph(&mut graph, fill_rule, overlay_rule);
        }
        graph.build(self.options);
        self.graph = Some(graph)
    }

    fn serial_build_graph(
        &mut self,
        graph: &mut OverlayGraph,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        for column in self.columns.iter_mut() {
            column.prepare_links(fill_rule, overlay_rule);
        }

        self.validate_links_range_and_allocate_space(graph);

        for column in self.columns.iter() {
            let sub_links = &mut graph.links[column.links_start..column.links_end()];
            let sub_ends = &mut graph.ends[column.links_start..column.links_end()];
            column.copy_links_into(overlay_rule, sub_links);
            column.prepare_links_and_ends(sub_links, sub_ends)
        }
    }

    fn parallel_build_graph(
        &mut self,
        graph: &mut OverlayGraph,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        self.columns
            .par_iter_mut()
            .for_each(|column| column.prepare_links(fill_rule, overlay_rule));

        self.validate_links_range_and_allocate_space(graph);

        let max_columns_count = (self.columns.len() / 256).max(2);

        Self::parallel_copy_and_sort_links(
            &self.columns,
            max_columns_count,
            overlay_rule,
            &mut graph.links,
            &mut graph.ends,
        );
    }

    fn parallel_copy_and_sort_links(
        columns: &[Column<ShapeCountBoolean>],
        max_columns_count: usize,
        overlay_rule: OverlayRule,
        links_slice: &mut [OverlayLink],
        ends_slice: &mut [End],
    ) {
        if columns.len() <= max_columns_count {
            let mut ls = links_slice;
            let mut es = ends_slice;

            for column in columns {
                let (sub_links, link_rest) = ls.split_at_mut(column.links_count);
                let (sub_ends, end_rest) = es.split_at_mut(column.links_count);

                column.copy_links_into(overlay_rule, sub_links);
                column.prepare_links_and_ends(sub_links, sub_ends);

                ls = link_rest;
                es = end_rest;
            }
            return;
        }

        let mid = columns.len() / 2;

        let (left_columns, right_columns) = columns.split_at(mid);

        let start = columns.first().map_or(0, |c| c.links_start);
        let middle = columns[mid].links_start;

        let left_len = middle - start;

        let (left_links, right_links) = links_slice.split_at_mut(left_len);
        let (left_ends, right_ends) = ends_slice.split_at_mut(left_len);

        rayon::join(
            || {
                Self::parallel_copy_and_sort_links(
                    left_columns,
                    max_columns_count,
                    overlay_rule,
                    left_links,
                    left_ends,
                )
            },
            || {
                Self::parallel_copy_and_sort_links(
                    right_columns,
                    max_columns_count,
                    overlay_rule,
                    right_links,
                    right_ends,
                )
            },
        );
    }

    #[inline]
    fn validate_links_range_and_allocate_space(&mut self, graph: &mut OverlayGraph) {
        let mut total_count = 0;
        for column in self.columns.iter_mut() {
            column.links_start = total_count;
            total_count += column.links_count;
        }

        graph.links.resize(total_count, Default::default());
        graph.ends.resize(total_count, Default::default());
    }
}

impl Column<ShapeCountBoolean> {
    #[inline]
    fn prepare_links(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) {
        self.split();
        self.fill_boolean(fill_rule);
        self.links_count = self.count_links(overlay_rule);
    }
}
