use crate::definition::segment::SegmentFill;
use crate::core::fill_rule::FillRule;
use crate::core::overlay::Overlay;
use crate::core::overlay_rule::OverlayRule;
use crate::gear::fill_buffer::FillBuffer;
use crate::gear::section::Section;
use crate::geom::x_segment::XSegment;
use alloc::vec::Vec;
use core::mem::swap;
#[cfg(feature = "allow_multithreading")]
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use crate::graph::data::OverlayGraph;

impl Overlay {
    pub(crate) fn process_overlay(
        &mut self,
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

    fn serial_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let packs: Vec<_> = self
            .sections
            .iter_mut()
            .map(|s| s.process(fill_rule, overlay_rule)).collect();

        OverlayGraph::new(1, SegmentsPack::with_packs(packs), self.options)
    }

    #[cfg(feature = "allow_multithreading")]
    fn parallel_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> OverlayGraph {
        let packs: Vec<_> = self
            .sections
            .par_iter_mut()
            .map(|s| s.process(fill_rule, overlay_rule)).collect();

        OverlayGraph::new(self.cpu_count.count(), SegmentsPack::with_packs(packs), self.options)
    }
}

pub(super) struct SegmentsPack {
    pub(super) segments: Vec<XSegment>,
    pub(super) fills: Vec<SegmentFill>,
}

impl SegmentsPack {
    #[inline]
    fn with_packs(packs: Vec<SegmentsPack>) -> Self {
        let capacity = packs.iter().fold(0, |s, p| s + p.fills.len());
        let mut segments = Vec::with_capacity(capacity);
        let mut fills = Vec::with_capacity(capacity);
        for mut pack in packs {
            segments.append(&mut pack.segments);
            fills.append(&mut pack.fills);
        }
        Self {
            segments,
            fills,
        }
    }
}

impl Section {
    fn process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> SegmentsPack {
        // split by columns

        let mut source_by_columns = self.source.new_same_size();
        let mut map_by_columns = self
            .source
            .map_by_columns(&self.layout, &mut source_by_columns);

        // intersect

        let mut split_buffer = self.intersect(&mut source_by_columns, &map_by_columns);

        let any_split = !split_buffer.is_empty();

        if any_split {
            self.split_by_marks(
                &mut source_by_columns,
                &mut split_buffer,
                &mut Vec::new(),
                &mut Vec::new(),
            );
            map_by_columns = source_by_columns.map_by_columns(&self.layout, &mut self.source);
        } else {
            swap(&mut self.source, &mut source_by_columns)
        }

        let any_merge = self.sort_and_merge(&map_by_columns, &mut Vec::new());

        if any_split || any_merge {
            self.source.init_map(&mut map_by_columns);
        }

        let fill_source = self.fill(fill_rule, FillBuffer::new(split_buffer), map_by_columns);

        self.prepare_segments(overlay_rule, fill_source)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay::Overlay;
    use crate::core::overlay_rule::OverlayRule;
    use alloc::vec;
    use i_float::int::point::IntPoint;
    use crate::core::cpu_count::CPUCount;

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

        let cpu_count = CPUCount::Fixed(2);
        let mut overlay =
            Overlay::with_contours_custom(&subj, &[], Default::default(), cpu_count).expect("create");
        let result = overlay.overlay(FillRule::EvenOdd, OverlayRule::Subject);

        assert_eq!(result.len(), 4);
    }
}
