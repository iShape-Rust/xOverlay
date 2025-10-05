use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use i_shape::int::shape::IntContour;
use crate::geom::range::LineRange;
use crate::tile::layout::TileLayout;

pub(super) struct TilePart {
    pub(super) range: LineRange,
    pub(super) count_hz: usize,
    pub(super) count_vr: usize,
    pub(super) count_dp: usize,
    pub(super) count_dn: usize,
}

pub(super) struct TileMapper {
    pub(super) layout: TileLayout,
    pub(super) hz_parts: Vec<usize>,
    pub(super) vr_parts: Vec<usize>,
    pub(super) dp_parts: Vec<usize>,
    pub(super) dn_parts: Vec<usize>,
}

impl TileMapper {
    #[inline]
    pub(super) fn new(layout: TileLayout) -> Self {
        let n = layout.count;
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
                if let Some((i0, i1)) = self.layout.indices_by_xx(p0.x, pi.x) {
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
            }
            p0 = pi;
        }
    }

    pub(crate) fn iter_by_parts(&self) -> impl Iterator<Item = TilePart> {
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
        let mut x0 = self.layout.range.min;
        let s = self.layout.step() as i32;
        (0..n).map(move |i| {
            let x1 = x0 + s;
            let range = LineRange::with_min_max(x0, x1);
            x0 = x1;
            unsafe {
                TilePart {
                    range,
                    count_hz: *hz.get_unchecked(i),
                    count_vr: *vr.get_unchecked(i),
                    count_dp: *dp.get_unchecked(i),
                    count_dn: *dn.get_unchecked(i),
                }
            }
        })
    }
}