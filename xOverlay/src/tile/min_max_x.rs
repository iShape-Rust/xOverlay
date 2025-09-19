use i_shape::int::shape::IntContour;
use crate::geom::range::LineRange;

pub(crate) trait XRangeAndCount {
    fn x_range_and_count(&self) -> (LineRange, usize);
}

impl XRangeAndCount for [IntContour] {
    fn x_range_and_count(&self) -> (LineRange, usize) {
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