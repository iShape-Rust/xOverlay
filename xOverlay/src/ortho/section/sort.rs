use alloc::vec;
use alloc::vec::Vec;
use crate::ortho::column::Column;
use crate::ortho::mapper::Mapper;
use crate::ortho::section::Section;
use crate::ortho::section::segment::Segment;

impl Section {

    pub(crate) fn sort_by_columns(self) -> Self {
        let map = self.map();
        let mut columns = Vec::with_capacity(map.parts.len());

        let mut section = Section {
            vr_list: vec![Segment::default(); self.vr_list.len()],
            hz_list: vec![Segment::default(); self.hz_list.len()],
            dg_pos_list: vec![Segment::default(); self.dg_pos_list.len()],
            dg_neg_list: vec![Segment::default(); self.dg_neg_list.len()],
            border_points: vec![0; self.border_points.len()],
            layout: self.layout.clone(),
        };

        for s in self.vr_list.into_iter() {

        }

    }

    fn map(&self) -> Mapper {
        let mut mapper = Mapper::new(self.layout.clone());
        mapper.add_vr_list(&self.vr_list);
        mapper.add_hz_list(&self.hz_list);
        mapper.add_dg_pos_list(&self.dg_pos_list);
        mapper.add_dg_neg_list(&self.dg_neg_list);
    }

}