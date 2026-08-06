use crate::core::cpu_count::CPUCount;
use crate::geom::range::LineRange;
use i_shape::int::shape::IntContour;

pub(crate) trait XRangeAndCount {
    fn x_range_and_count(&self, cpu_count: CPUCount) -> (LineRange, usize);
}

impl XRangeAndCount for [IntContour<i32>] {
    fn x_range_and_count(&self, _cpu: CPUCount) -> (LineRange, usize) {
        #[cfg(feature = "allow_multithreading")]
        {
            use rayon::iter::IntoParallelRefIterator;
            use rayon::iter::ParallelIterator;

            if _cpu.count() > 1 {
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
                return (LineRange::with_min_max(min_x, max_x), count);
            }
        }

        // serial

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut count = 0;

        for contour in self.iter() {
            for p in contour {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
            }
            count += contour.len();
        }

        (LineRange::with_min_max(min_x, max_x), count)
    }
}
