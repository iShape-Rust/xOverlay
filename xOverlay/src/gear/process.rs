use crate::core::fill_rule::FillRule;
use crate::core::overlay::Overlay;
use crate::core::overlay_rule::OverlayRule;
use crate::gear::section::Section;

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
        
    }
}