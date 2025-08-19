use alloc::vec;
use alloc::vec::Vec;
use crate::core::layout::Layout;
use i_shape::int::shape::IntContour;

#[derive(Default, Clone)]
pub(crate) struct Counter {
    pub(crate) hz: usize,
    pub(crate) vr: usize,
    pub(crate) border_points: usize,
}

pub(crate) struct Mapper {
    layout: Layout,
    pub(crate) columns: Vec<Counter>,
}

impl Mapper {
    #[inline]
    pub(crate) fn new(layout: Layout) -> Self {
        let n = layout.count();
        Self {
            layout,
            columns: vec![Counter::default(); n],
        }
    }

    pub(crate) fn add_ortho_contours(&mut self, contours: &[IntContour]) {
        for contour in contours {
            if contour.len() >= 4 {
                self.add_ortho_contour(contour);
            }
        }
    }

    #[inline(always)]
    fn add_ortho_contour(&mut self, contour: &IntContour) {
        let mut p0 = contour[0];
        for &pi in contour.iter() {
            if pi.x == p0.x {
                // vertical
                let index = self.layout.index(pi.x);
                unsafe {
                    self.columns.get_unchecked_mut(index).vr += 1;
                }
            } else {
                // horizontal
                let (i0, i1, border) = self.layout.indices(p0.x, pi.x);
                for index in i0..=i1 {
                    unsafe {
                        self.columns.get_unchecked_mut(index).hz += 1;
                    }
                }
                if border {
                    unsafe {
                        self.columns.get_unchecked_mut(i1 + 1).border_points += 1;
                    }
                }
            }
            p0 = pi;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::layout::Layout;
    use crate::ortho::mapper::Mapper;
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

        let mut mapper = Mapper::new(Layout::with_subj_and_clip(&subj, &[], 2).unwrap());

        mapper.add_ortho_contours(&subj);

        assert_eq!(mapper.columns[0].hz, 2);
        assert_eq!(mapper.columns[0].vr, 2);
    }
}
