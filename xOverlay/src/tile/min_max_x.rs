use rayon::iter::ParallelIterator;
use i_shape::int::shape::IntContour;
use rayon::iter::IntoParallelRefIterator;
use crate::geom::range::LineRange;

pub(crate) trait XRangeAndCount {
    fn x_range_and_count(&self, parallel: bool) -> (LineRange, usize);
}

impl XRangeAndCount for [IntContour] {
    fn x_range_and_count(&self, parallel: bool) -> (LineRange, usize) {
        if parallel {
            let (min_x, max_x, count) = self
                .par_iter()
                .map(|contour| {
                    let mut min_x = i32::MAX;
                    let mut max_x = i32::MIN;
                    for p in contour {
                        min_x = min_x.min(p.x);
                        max_x = max_x.max(p.x);
                    }
                    (min_x, max_x, contour.len())
                })
                .reduce(
                    || (i32::MAX, i32::MIN, 0usize),
                    |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2),
                );
            (LineRange::with_min_max(min_x, max_x), count)
        } else {
            let mut min_x = i32::MAX;
            let mut max_x = i32::MIN;
            let mut count = 0;

            for contour in self.iter() {
                for p in contour {
                    min_x = min_x.min(p.x);
                    max_x = max_x.min(p.x);
                }
                count += contour.len();
            }

            (LineRange::with_min_max(min_x, max_x), count)
        }
    }
}