use crate::core::fill_rule::FillRule;
use crate::core::overlay::Overlay;
use crate::core::overlay_rule::OverlayRule;
use crate::gear::fill_buffer::FillBuffer;
use crate::gear::section::Section;
use crate::gear::sub_graph::FilledSegment;
use crate::graph::OverlayGraph;
use alloc::vec::Vec;
use core::mem::swap;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

impl Overlay {
    pub(crate) fn process_overlay(
        &mut self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> OverlayGraph {
        if self.solver.cpu_count() == 1 {
            self.serial_process(fill_rule, overlay_rule)
        } else {
            self.parallel_process(fill_rule, overlay_rule)
        }
    }

    fn serial_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let mut groups = Vec::with_capacity(self.sections.len());
        for s in self.sections.iter_mut() {
            groups.push(s.process(fill_rule, overlay_rule));
        }
        OverlayGraph::new(false, groups, self.options)
    }

    fn parallel_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let groups: Vec<_> = self
            .sections
            .par_iter_mut()
            .map(|s| s.process(fill_rule, overlay_rule))
            .collect();
        OverlayGraph::new(true, groups, self.options)
    }
}

impl Section {
    fn process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> Vec<FilledSegment> {
        // split by columns

        let mut source_by_columns = self.source.new_same_size();
        let mut map_by_columns = self
            .source
            .map_by_columns(&self.layout, &mut source_by_columns);

        // intersect

        let mut split_buffer = self.intersect(&mut source_by_columns, &map_by_columns);

        let any_split = !split_buffer.is_empty();

        if any_split {
            self.split_by_marks(&mut source_by_columns, &mut split_buffer, &mut Vec::new(), &mut Vec::new());
            map_by_columns = source_by_columns.map_by_columns(&self.layout, &mut self.source);
        } else {
            swap(&mut self.source, &mut source_by_columns)
        }

        let any_merge = self.sort_and_merge(&map_by_columns, &mut Vec::new());

        if any_split || any_merge {
            self.source.init_map(&mut map_by_columns);
        }

        let fill_source = self.fill(fill_rule, FillBuffer::new(split_buffer), map_by_columns);

        self.filled_segments(overlay_rule, fill_source)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use i_float::int::point::IntPoint;
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay::Overlay;
    use crate::core::overlay_rule::OverlayRule;
    use crate::core::solver::Solver;

    #[test]
    fn test_0() {
        let subj = [
            vec![
                IntPoint::new(1, 1),
                IntPoint::new(7, 1),
                IntPoint::new(7, 2),
                IntPoint::new(1, 2),
            ],
            vec![
                IntPoint::new(5, 3),
                IntPoint::new(7, 3),
                IntPoint::new(7, 4),
                IntPoint::new(5, 4),
            ],
            vec![
                IntPoint::new(1, 5),
                IntPoint::new(5, 5),
                IntPoint::new(5, 6),
                IntPoint::new(1, 6),
            ],
            vec![
                IntPoint::new(1, 7),
                IntPoint::new(7, 7),
                IntPoint::new(7, 8),
                IntPoint::new(1, 8),
            ],
        ];

        let solver = Solver::fixed(2);
        let mut overlay = Overlay::with_contours_custom(&subj, &[], Default::default(), solver).expect("create");
        let result = overlay.overlay(FillRule::EvenOdd, OverlayRule::Subject);

        assert_eq!(result.len(), 4);
    }

}
