use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use i_shape::int::shape::IntContour;
use crate::gear::s_layout::SLayout;

pub(super) struct SPart {
    pub(super) count_hz: usize,
    pub(super) count_vr: usize,
    pub(super) count_dp: usize,
    pub(super) count_dn: usize,
}

pub(super) struct SMapper {
    pub(super) layout: SLayout,
    pub(super) hz_parts: Vec<usize>,
    pub(super) vr_parts: Vec<usize>,
    pub(super) dp_parts: Vec<usize>,
    pub(super) dn_parts: Vec<usize>,
}

impl SMapper {
    #[inline]
    pub(super) fn new(layout: SLayout) -> Self {
        let n = layout.count();
        Self {
            layout,
            hz_parts: vec![0; n],
            vr_parts: vec![0; n],
            dp_parts: vec![0; n],
            dn_parts: vec![0; n],
        }
    }

    pub(super) fn add_contours(&mut self, contours: &[IntContour]) {
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

    pub(crate) fn iter_by_parts(&self) -> impl Iterator<Item =SPart> {
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
            SPart {
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
    use i_float::int::point::IntPoint;
    use crate::gear::s_mapper::SMapper;
    use crate::gear::s_layout::SLayout;

    #[test]
    fn test_0() {
        let subj = [[
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]
            .to_vec()];

        let mut mapper = SMapper::new(SLayout::with_subj_and_clip(&subj, &[], 2));

        mapper.add_contours(&subj);

        assert_eq!(mapper.hz_parts[0], 2);
        assert_eq!(mapper.vr_parts[0], 2);
    }
}
