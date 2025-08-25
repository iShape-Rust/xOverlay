use alloc::vec;
use alloc::vec::Vec;
use crate::gear::x_layout::XLayout;
use crate::gear::x_mapper::XMapper;
use crate::gear::segment::Segment;

pub(crate) struct GeometrySource {
    pub(crate) vr_list: Vec<Segment>,
    pub(crate) hz_list: Vec<Segment>,
    pub(crate) dg_pos_list: Vec<Segment>,
    pub(crate) dg_neg_list: Vec<Segment>,
}

impl GeometrySource {
    pub(super) fn map_by_columns(&self, layout: &XLayout) -> (Self, XMapper) {
        let map = self.mapper(layout.clone());

        let mut vr_by_columns = vec![Segment::default(); self.vr_list.len()];
        let mut hz_by_columns = vec![Segment::default(); self.hz_list.len()];
        let mut dg_pos_by_columns = vec![Segment::default(); self.dg_pos_list.len()];
        let mut dg_neg_by_columns = vec![Segment::default(); self.dg_neg_list.len()];

        // copy by columns

        let mut indices = vec![0usize; layout.count()];

        layout.copy_by_pos(&map.vr_parts, &mut indices, &self.vr_list, &mut vr_by_columns);
        layout.copy_by_min(&map.hz_parts, &mut indices, &self.hz_list, &mut hz_by_columns);
        layout.copy_by_min(&map.dg_pos_parts, &mut indices, &self.dg_pos_list, &mut dg_pos_by_columns);
        layout.copy_by_min(&map.dg_neg_parts, &mut indices, &self.dg_neg_list, &mut dg_neg_by_columns);

        (Self {
            vr_list: vr_by_columns,
            hz_list: hz_by_columns,
            dg_pos_list: dg_pos_by_columns,
            dg_neg_list: dg_neg_by_columns,
        }, map)
    }

    fn mapper(&self, layout: XLayout) -> XMapper {
        let mut mapper = XMapper::new(layout);
        mapper.add_vr_list(&self.vr_list);
        mapper.add_hz_list(&self.hz_list);
        mapper.add_dg_pos_list(&self.dg_pos_list);
        mapper.add_dg_neg_list(&self.dg_neg_list);
        mapper
    }
}

impl XLayout {
    fn copy_by_pos(&self, parts: &[usize], indices: &mut[usize], source: &[Segment], target: &mut [Segment]) {
        let mut offset = 0;
        for (&n, inx) in parts.iter().zip(indices.iter_mut()) {
            *inx = offset;
            offset += n;
        }

        for &vr in source.iter() {
            let index = self.index(vr.pos);
            unsafe {
                *target.get_unchecked_mut(index) = vr;
            }
        }
    }

    fn copy_by_min(&self, parts: &[usize], indices: &mut[usize], source: &[Segment], target: &mut [Segment]) {
        let mut offset = 0;
        for (&n, inx) in parts.iter().zip(indices.iter_mut()) {
            *inx = offset;
            offset += n;
        }

        for &hz in source.iter() {
            let index = self.index(hz.min);
            unsafe {
                *target.get_unchecked_mut(index) = hz;
            }
        }
    }
}
