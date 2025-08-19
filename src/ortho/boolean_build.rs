use crate::build::boolean::ShapeCountBoolean;
use crate::core::fill_rule::FillRule;
use crate::core::graph::BooleanGraph;
use crate::core::link::OverlayLink;
use crate::core::overlay_rule::OverlayRule;
use crate::ortho::column::Column;
use crate::ortho::overlay::OrthoOverlay;
use alloc::vec::Vec;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

impl OrthoOverlay<ShapeCountBoolean> {
    #[inline]
    pub(super) fn build_graph(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) {
        let mut graph = self.graph.take().unwrap_or_default();
        if self.solver.multithreading && self.columns.len() > 4 {
            self.parallel_build_graph(&mut graph, fill_rule, overlay_rule);
        } else {
            self.serial_build_graph(&mut graph, fill_rule, overlay_rule);
        }
        self.graph = Some(graph)
    }

    fn serial_build_graph(
        &mut self,
        graph: &mut BooleanGraph,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        let counts: Vec<usize> = self
            .columns
            .iter_mut()
            .map(|column| column.prepare_links(fill_rule, overlay_rule))
            .collect();

        let total_count = counts.iter().sum();

        graph.links.resize(total_count, Default::default());

        let mut start = 0;
        for (column, count) in self.columns.iter().zip(counts) {
            let sub_links = &mut graph.links[start..];
            column.copy_and_sort_links_into(overlay_rule, sub_links);
            start += count;
        }
    }

    fn parallel_build_graph(
        &mut self,
        graph: &mut BooleanGraph,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        let mut chunks: Vec<Chunk> = self
            .columns
            .par_iter_mut()
            .map(|column| {
                let count = column.prepare_links(fill_rule, overlay_rule);
                Chunk { end: count, column }
            })
            .collect();

        let mut count = 0;
        for chunk in chunks.iter_mut() {
            count += chunk.end;
            chunk.end = count;
        }
        graph.links.resize(count, Default::default());

        Self::parallel_copy_and_sort_links(&mut graph.links, &chunks, 0, overlay_rule);
    }

    fn parallel_copy_and_sort_links(
        slice: &mut [OverlayLink],
        chunks: &[Chunk],
        base: usize,
        overlay_rule: OverlayRule,
    ) {
        const THRESHOLD: usize = 16;
        if chunks.len() <= THRESHOLD {
            let mut prev = base;
            let mut s = slice;
            for chunk in chunks.iter() {
                let len = chunk.end - prev;
                let (sub_links, rest) = s.split_at_mut(len);
                chunk.column.copy_and_sort_links_into(overlay_rule, sub_links);
                // self.columns[column_index].copy_links_into_with_overlay_rule(overlay_rule, 0, slice);

                s = rest;
                prev = chunk.end;
            }
            return;
        }

        let mid = chunks.len() / 2;

        // Split point in *global* coords, then convert to local offset.
        let split_global = chunks[mid - 1].end;
        let left_len = split_global - base;

        let (left, right) = slice.split_at_mut(left_len);
        let (ends_left, ends_right) = chunks.split_at(mid);

        rayon::join(
            || Self::parallel_copy_and_sort_links(left, ends_left, base, overlay_rule),
            || Self::parallel_copy_and_sort_links(right, ends_right, split_global, overlay_rule),
        );
    }
}

impl Column<ShapeCountBoolean> {
    fn prepare_links(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> usize {
        self.split();
        self.fill_boolean(fill_rule);
        self.count_included_links_with_overlay_rule(overlay_rule)
    }
}

struct Chunk<'a> {
    end: usize,
    column: &'a Column<ShapeCountBoolean>,
}
