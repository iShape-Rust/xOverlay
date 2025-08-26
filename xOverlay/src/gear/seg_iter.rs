use core::iter::{Chain, Copied};
use core::marker::PhantomData;
use core::slice::Iter;
use i_float::int::point::IntPoint;

pub(crate) struct SegmentIterator<'a, F> {
    iter: Chain<Copied<Iter<'a, IntPoint>>, core::array::IntoIter<IntPoint, 2>>,
    p0: IntPoint,
    p1: IntPoint,
    phantom_data: PhantomData<fn() -> F>,
}

impl<'a, F: PointFilter> SegmentIterator<'a, F> {
    #[inline]
    fn new(contour: &'a [IntPoint]) -> Option<Self> {
        if contour.len() < 3 {
            return None;
        }

        let mut iter = contour.iter().copied();

        let mut p0 = if let Some(p) = iter.next() {
            p
        } else {
            return None;
        };
        let mut p1 = if let Some(p) = iter.find(|p| p0.ne(p)) {
            p
        } else {
            return None;
        };

        let q0 = p0;

        for p2 in &mut iter {
            if F::keep_vertex(p0, p1, p2) {
                p0 = p1;
                p1 = p2;
                break;
            }
            p1 = p2;
        }

        let q1 = p0;

        let chain_iter = iter.chain([q0, q1]);

        Some(Self {
            iter: chain_iter,
            p0,
            p1,
            phantom_data: PhantomData,
        })
    }
}

impl<'a, F: PointFilter> Iterator for SegmentIterator<'a, F> {
    type Item = [IntPoint; 2];

    #[inline]
    fn next(&mut self) -> Option<[IntPoint; 2]> {
        for p2 in &mut self.iter {
            if !F::keep_vertex(self.p0, self.p1, p2) {
                self.p1 = p2;
                continue;
            }

            let item = [self.p0, self.p1];

            self.p0 = self.p1;
            self.p1 = p2;

            return Some(item);
        }

        if self.p1 == self.p0 {
            None
        } else {
            let item = [self.p0, self.p1];
            self.p0 = self.p1;
            Some(item)
        }
    }
}

pub(crate) trait SegmentIterable {
    #[must_use]
    fn segment_iter<F: PointFilter>(&self) -> Option<SegmentIterator<F>>;
}

impl SegmentIterable for [IntPoint] {
    #[inline]
    fn segment_iter<F: PointFilter>(&self) -> Option<SegmentIterator<F>> {
        SegmentIterator::new(self)
    }
}

pub(crate) trait PointFilter {
    fn keep_vertex(a: IntPoint, b: IntPoint, c: IntPoint) -> bool;
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct DropOppositeCollinear;
#[derive(Copy, Clone, Debug)]
pub(crate) struct DropCollinear;

impl PointFilter for DropOppositeCollinear {
    #[inline(always)]
    fn keep_vertex(p0: IntPoint, p1: IntPoint, p2: IntPoint) -> bool {
        let a = p1.subtract(p0);
        let b = p1.subtract(p2);

        if a.cross_product(b) != 0 {
            // not collinear
            return true;
        }

        // collinear – keep only if we keep going same direction
        a.dot_product(b) < 0
    }
}

impl PointFilter for DropCollinear {
    #[inline(always)]
    fn keep_vertex(p0: IntPoint, p1: IntPoint, p2: IntPoint) -> bool {
        let a = p1.subtract(p0);
        let b = p1.subtract(p2);
        a.cross_product(b) != 0
    }
}

#[cfg(test)]
mod tests {
    use crate::gear::seg_iter::{DropCollinear, SegmentIterable};
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;

    #[test]
    fn test_0() {
        let contour = [
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ];

        let result: Vec<_> = contour.segment_iter::<DropCollinear>().unwrap().collect();

        let template = vec![
            [contour[1], contour[2]],
            [contour[2], contour[3]],
            [contour[3], contour[0]],
            [contour[0], contour[1]],
        ];

        assert_eq!(template, result);
    }

    #[test]
    fn test_1() {
        let contour = [
            IntPoint::new(0, 0),
            IntPoint::new(5, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 5),
            IntPoint::new(10, 10),
            IntPoint::new(5, 10),
            IntPoint::new(0, 10),
            IntPoint::new(0, 5),
        ];

        let result: Vec<_> = contour.segment_iter::<DropCollinear>().unwrap().collect();

        let template = vec![
            [contour[2], contour[4]],
            [contour[4], contour[6]],
            [contour[6], contour[0]],
            [contour[0], contour[2]],
        ];

        assert_eq!(template, result);
    }
}
