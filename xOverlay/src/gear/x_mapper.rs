use crate::gear::x_layout::XLayout;
use crate::gear::segment::Segment;
use alloc::vec;
use alloc::vec::Vec;

pub(super) struct XPart {
    pub(super) count_hz: usize,
    pub(super) count_vr: usize,
    pub(super) count_dp: usize,
    pub(super) count_dn: usize,
}

pub(super) struct XMapper {
    pub(super) layout: XLayout,
    pub(super) hz_parts: Vec<usize>,
    pub(super) vr_parts: Vec<usize>,
    pub(super) dp_parts: Vec<usize>,
    pub(super) dn_parts: Vec<usize>,
}

impl XMapper {
    #[inline]
    pub(super) fn new(layout: XLayout) -> Self {
        let n = layout.count();
        Self {
            layout,
            hz_parts: vec![0; n],
            vr_parts: vec![0; n],
            dp_parts: vec![0; n],
            dn_parts: vec![0; n],
        }
    }

    #[inline]
    pub(super) fn add_vr_segments(&mut self, segments: &[Segment]) {
        Self::add_segments_by_pos(&self.layout, segments, &mut self.vr_parts)
    }

    #[inline]
    pub(super) fn add_hz_segments(&mut self, segments: &[Segment]) {
        Self::add_segments_by_min(&self.layout, segments, &mut self.hz_parts)
    }

    #[inline]
    pub(super) fn add_dp_segments(&mut self, segments: &[Segment]) {
        Self::add_segments_by_min(&self.layout, segments, &mut self.dp_parts)
    }

    #[inline]
    pub(super) fn add_dn_segments(&mut self, segments: &[Segment]) {
        Self::add_segments_by_min(&self.layout, segments, &mut self.dn_parts)
    }

    fn add_segments_by_pos(layout: &XLayout, segments: &[Segment], counter: &mut [usize]) {
        for s in segments.iter() {
            let index = layout.index(s.pos);
            unsafe {
                *counter.get_unchecked_mut(index) += 1;
            }
        }
    }

    fn add_segments_by_min(layout: &XLayout, segments: &[Segment], counter: &mut [usize]) {
        for s in segments.iter() {
            let index = layout.index(s.range.min);
            unsafe {
                *counter.get_unchecked_mut(index) += 1;
            }
        }
    }

    pub(crate) fn iter_by_parts(&self) -> impl Iterator<Item =XPart> {
        let (hz, vr, dp, dn) = (
            &self.hz_parts[..],
            &self.vr_parts[..],
            &self.dp_parts[..],
            &self.dn_parts[..],
        );
        debug_assert!(hz.len() == vr.len()
            && hz.len() == dp.len()
            && hz.len() == dn.len());

        let n = hz.len();
        (0..n).map(move |i| unsafe {
            XPart {
                count_hz: *hz.get_unchecked(i),
                count_vr: *vr.get_unchecked(i),
                count_dp: *dp.get_unchecked(i),
                count_dn: *dn.get_unchecked(i),
            }
        })
    }
}