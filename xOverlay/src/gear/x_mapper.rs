use crate::gear::x_layout::XLayout;
use crate::gear::segment::Segment;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use i_shape::int::shape::IntContour;

pub(crate) struct XPart {
    pub(crate) count_hz: usize,
    pub(crate) count_vr: usize,
    pub(crate) count_dp: usize,
    pub(crate) count_dn: usize,
}

pub(crate) struct XMapper {
    layout: XLayout,
    pub(crate) hz_parts: Vec<usize>,
    pub(crate) vr_parts: Vec<usize>,
    pub(crate) dp_parts: Vec<usize>,
    pub(crate) dn_parts: Vec<usize>,
}

impl XMapper {
    #[inline]
    pub(crate) fn new(layout: XLayout) -> Self {
        let n = layout.count();
        Self {
            layout,
            hz_parts: vec![0; n],
            vr_parts: vec![0; n],
            dp_parts: vec![0; n],
            dn_parts: vec![0; n],
        }
    }

    pub(crate) fn add_contours(&mut self, contours: &[IntContour]) {
        for contour in contours {
            if contour.len() >= 4 {
                self.add_contour(contour);
            }
        }
    }

    #[inline(always)]
    fn add_contour(&mut self, contour: &IntContour) {
        let mut p0 = contour[0];
        for &pi in contour.iter() {
            if pi.x == p0.x {
                // vertical
                let index = self.layout.index(pi.x);
                unsafe {
                    *self.vr_parts.get_unchecked_mut(index) += 1;
                }
            } else {
                let (i0, i1) = self.layout.indices_by_xx(p0.x, pi.x);

                match pi.y.cmp(&p0.y) {
                    Ordering::Equal => {
                        // horizontal
                        for index in i0..=i1 {
                            unsafe {
                                *self.hz_parts.get_unchecked_mut(index) += 1;
                            }
                        }
                    }
                    Ordering::Less => {
                        // positive diagonal
                        for index in i0..=i1 {
                            unsafe {
                                *self.dp_parts.get_unchecked_mut(index) += 1;
                            }
                        }
                    }
                    Ordering::Greater => {
                        // negative diagonal
                        for index in i0..=i1 {
                            unsafe {
                                *self.dn_parts.get_unchecked_mut(index) += 1;
                            }
                        }
                    }
                }
            }
            p0 = pi;
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

#[cfg(test)]
mod tests {
    use crate::gear::x_layout::XLayout;
    use crate::gear::x_mapper::XMapper;
    use i_float::int::point::IntPoint;

    #[test]
    fn test_0() {
        let subj = [[
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]
        .to_vec()];

        let mut mapper = XMapper::new(XLayout::with_subj_and_clip(&subj, &[], 2).unwrap());

        mapper.add_contours(&subj);

        assert_eq!(mapper.hz_parts[0], 2);
        assert_eq!(mapper.vr_parts[0], 2);
    }
}
