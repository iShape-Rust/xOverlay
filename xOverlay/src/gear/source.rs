use crate::gear::segment::Segment;
use crate::gear::x_layout::XLayout;
use crate::gear::x_mapper::XMapper;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone)]
pub(crate) struct GeometrySource {
    pub(crate) vr_list: Vec<Segment>,
    pub(crate) hz_list: Vec<Segment>,
    pub(crate) dp_list: Vec<Segment>,
    pub(crate) dn_list: Vec<Segment>,
}

impl GeometrySource {
    pub(crate) fn count(&self) -> usize {
        self.vr_list.len() + self.hz_list.len() + self.dp_list.len() + self.dn_list.len()
    }

    pub(super) fn map_by_columns(&self, layout: &XLayout, output: &mut Self) -> XMapper {
        let mut map = XMapper::new(layout.clone());
        self.fill_map(&mut map);

        // copy by columns
        let mut indices = vec![0usize; layout.count()];

        layout.copy_by_pos(
            &map.vr_parts,
            &mut indices,
            &self.vr_list,
            &mut output.vr_list,
        );
        layout.copy_by_min(
            &map.hz_parts,
            &mut indices,
            &self.hz_list,
            &mut output.hz_list,
        );
        layout.copy_by_min(
            &map.dp_parts,
            &mut indices,
            &self.dp_list,
            &mut output.dp_list,
        );
        layout.copy_by_min(
            &map.dn_parts,
            &mut indices,
            &self.dn_list,
            &mut output.dn_list,
        );

        map
    }

    pub(super) fn fill_map(&self, map: &mut XMapper) {
        map.add_vr_segments(&self.vr_list);
        map.add_hz_segments(&self.hz_list);
        map.add_dp_segments(&self.dp_list);
        map.add_dn_segments(&self.dn_list);
    }

    pub(super) fn init_map(&mut self, map: &mut XMapper) {
        map.vr_parts.fill(0);
        map.hz_parts.fill(0);
        map.dp_parts.fill(0);
        map.dn_parts.fill(0);
        self.fill_map(map);
    }

    pub(super) fn new_same_size(&self) -> Self {
        Self {
            vr_list: vec![Default::default(); self.vr_list.len()],
            hz_list: vec![Default::default(); self.hz_list.len()],
            dp_list: vec![Default::default(); self.dp_list.len()],
            dn_list: vec![Default::default(); self.dn_list.len()],
        }
    }
}

impl XLayout {
    fn copy_by_pos(
        &self,
        parts: &[usize],
        indices: &mut [usize],
        source: &[Segment],
        target: &mut [Segment],
    ) {
        let mut offset = 0;
        for (&n, inx) in parts.iter().zip(indices.iter_mut()) {
            *inx = offset;
            offset += n;
        }

        for &s in source.iter() {
            let column = self.index(s.pos);
            unsafe {
                let index = indices.get_unchecked_mut(column);
                let i = *index;
                *index += 1;
                *target.get_unchecked_mut(i) = s;
            }
        }
    }

    fn copy_by_min(
        &self,
        parts: &[usize],
        indices: &mut [usize],
        source: &[Segment],
        target: &mut [Segment],
    ) {
        let mut offset = 0;
        for (&n, inx) in parts.iter().zip(indices.iter_mut()) {
            *inx = offset;
            offset += n;
        }

        for &s in source.iter() {
            let column = self.index(s.range.min);
            unsafe {
                let index = indices.get_unchecked_mut(column);
                let i = *index;
                *index += 1;
                *target.get_unchecked_mut(i) = s;
            }
        }
    }
}
