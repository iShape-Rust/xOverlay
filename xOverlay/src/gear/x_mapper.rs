use crate::gear::x_layout::XLayout;
use crate::gear::segment::Segment;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use i_shape::int::shape::IntContour;

pub(crate) struct XPart {
    pub(crate) hz: usize,
    pub(crate) vr: usize,
    pub(crate) dg_pos: usize,
    pub(crate) dg_neg: usize,
    pub(crate) border: usize,
}

pub(crate) struct XMapper {
    layout: XLayout,
    pub(crate) hz_parts: Vec<usize>,
    pub(crate) vr_parts: Vec<usize>,
    pub(crate) dg_pos_parts: Vec<usize>,
    pub(crate) dg_neg_parts: Vec<usize>,
    pub(crate) borders: Vec<usize>,
}

impl XMapper {
    #[inline]
    pub(crate) fn new(layout: XLayout) -> Self {
        let n = layout.count();
        Self {
            layout,
            hz_parts: vec![0; n],
            vr_parts: vec![0; n],
            dg_pos_parts: vec![0; n],
            dg_neg_parts: vec![0; n],
            borders: vec![0; n],
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
                let (i0, i1, border) = self.layout.indices(p0.x, pi.x);

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
                                *self.dg_pos_parts.get_unchecked_mut(index) += 1;
                            }
                        }
                    }
                    Ordering::Greater => {
                        // negative diagonal
                        for index in i0..=i1 {
                            unsafe {
                                *self.dg_neg_parts.get_unchecked_mut(index) += 1;
                            }
                        }
                    }
                }
                if border {
                    unsafe {
                        *self.borders.get_unchecked_mut(i1 + 1) += 1;
                    }
                }
            }
            p0 = pi;
        }
    }

    pub(crate) fn add_vr_list(&mut self, list: &[Segment]) {
        for vr in list.iter() {
            let index = self.layout.index(vr.pos);
            unsafe {
                *self.vr_parts.get_unchecked_mut(index) += 1;
            }
        }
    }

    pub(crate) fn add_hz_list(&mut self, list: &[Segment]) {
        for hz in list.iter() {
            let (i0, i1, border) = self.layout.indices(hz.min, hz.max);
            for index in i0..=i1 {
                unsafe {
                    *self.hz_parts.get_unchecked_mut(index) += 1;
                }
            }
            if border {
                unsafe {
                    *self.borders.get_unchecked_mut(i1 + 1) += 1;
                }
            }
        }
    }

    pub(crate) fn add_dg_pos_list(&mut self, list: &[Segment]) {
        for dg in list.iter() {
            let (i0, i1, border) = self.layout.indices(dg.min, dg.max);
            for index in i0..=i1 {
                unsafe {
                    *self.dg_pos_parts.get_unchecked_mut(index) += 1;
                }
            }
            if border {
                unsafe {
                    *self.borders.get_unchecked_mut(i1 + 1) += 1;
                }
            }
        }
    }

    pub(crate) fn add_dg_neg_list(&mut self, list: &[Segment]) {
        for dg in list.iter() {
            let (i0, i1, border) = self.layout.indices(dg.min, dg.max);
            for index in i0..=i1 {
                unsafe {
                    *self.dg_neg_parts.get_unchecked_mut(index) += 1;
                }
            }
            if border {
                unsafe {
                    *self.borders.get_unchecked_mut(i1 + 1) += 1;
                }
            }
        }
    }

    pub(crate) fn iter_by_parts(&self) -> impl Iterator<Item =XPart> {
        let (hz, vr, dp, dn, br) = (
            &self.hz_parts[..],
            &self.vr_parts[..],
            &self.dg_pos_parts[..],
            &self.dg_neg_parts[..],
            &self.borders[..],
        );
        debug_assert!(hz.len() == vr.len()
            && hz.len() == dp.len()
            && hz.len() == dn.len()
            && hz.len() == br.len());

        let n = hz.len();
        (0..n).map(move |i| unsafe {
            XPart {
                hz: *hz.get_unchecked(i),
                vr: *vr.get_unchecked(i),
                dg_pos: *dp.get_unchecked(i),
                dg_neg: *dn.get_unchecked(i),
                border: *br.get_unchecked(i),
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
