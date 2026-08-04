use i_shape::int::shape::IntContour;
use crate::geom::range::LineRange;

pub(crate) trait YRangeAndCount {
    fn y_range_and_count(&self, parallel: bool) -> (LineRange, usize);
}

impl YRangeAndCount for [IntContour<i32>] {
    fn y_range_and_count(&self, parallel: bool) -> (LineRange, usize) {
        #[cfg(feature = "allow_multithreading")]
        {
            use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

            if parallel {
                let (min_y, max_y, count) = self
                    .par_iter()
                    .map(|contour| {
                        let mut min_y = i32::MAX;
                        let mut max_y = i32::MIN;
                        for p in contour {
                            min_y = min_y.min(p.y);
                            max_y = max_y.max(p.y);
                        }
                        (min_y, max_y, contour.len())
                    })
                    .reduce(
                        || (i32::MAX, i32::MIN, 0usize),
                        |a, b| (a.0.min(b.0), a.1.max(b.1), a.2 + b.2),
                    );
                return (LineRange::with_min_max(min_y, max_y), count);
            }
        }

        #[cfg(not(feature = "allow_multithreading"))]
        let _ = parallel;

        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        let mut count = 0;

        for contour in self.iter() {
            for p in contour {
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
            count += contour.len();
        }

        (LineRange::with_min_max(min_y, max_y), count)
    }
}
