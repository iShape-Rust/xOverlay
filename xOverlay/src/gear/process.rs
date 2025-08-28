use core::mem::swap;
use crate::core::fill_rule::FillRule;
use crate::core::overlay::Overlay;
use crate::core::overlay_rule::OverlayRule;
use crate::gear::fill_buffer::FillBuffer;
use crate::gear::section::Section;
use crate::gear::split_buffer::SplitBuffer;

impl Overlay {

    pub(crate) fn process_overlay(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) {
        if self.solver.cpu_count() == 1 {
            self.serial_process(fill_rule, overlay_rule);

        } else {
            self.parallel_process(fill_rule, overlay_rule);
        }
    }

    fn serial_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) {
        for s in self.sections.iter_mut() {
            s.process();
        }

    }

    fn parallel_process(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) {
        self.sections.iter_mut().for_each(|s|{
            s.process();
        })
    }
}

impl Section {

    fn process(&mut self) {

        // split by columns

        let mut source_by_columns = self.source.new_same_size();
        let mut map_by_columns = self
            .source
            .map_by_columns(&self.layout, &mut source_by_columns);

        // intersect

        let split_buffer = self.intersect(&mut source_by_columns, &map_by_columns);

        if split_buffer.is_empty() {
            swap(&mut self.source, &mut source_by_columns);
        } else {
            self.split_by_marks(&mut source_by_columns, &split_buffer);
            map_by_columns = source_by_columns.map_by_columns(&self.layout, &mut self.source);
        }

        // merge same

        self.sort_and_merge(&map_by_columns);

        // fill

        self.fill(FillBuffer::new(split_buffer), map_by_columns);
    }
}