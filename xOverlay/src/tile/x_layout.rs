use crate::geom::range::LineRange;
use crate::util::log2::Log2;
use i_float::int::rect::IntRect;
use i_shape::int::shape::IntContour;

const MIN_CHUNK_WIDTH_POWER: u32 = 4;
const MIN_CHUNK_WIDTH: usize = 1 << MIN_CHUNK_WIDTH_POWER;
const MAX_SPLIT_COUNT_POWER: u32 = 6;
const MAX_SPLIT_COUNT: usize = 1 << MAX_SPLIT_COUNT_POWER;

#[derive(Clone)]
pub(crate) struct XLayout {
    range: LineRange,
    power: usize,
    count: usize,
}

#[derive(Clone, Copy)]
struct LayoutConstraints {
    min_chunk_width: usize,
    min_chunk_width_power: u32,
    max_split_count: usize,
    max_split_count_power: u32,
}

impl LayoutConstraints {
    #[inline]
    fn new(min_chunk_width_power: u32, max_split_count_power: u32) -> Self {
        Self {
            min_chunk_width: 1 << min_chunk_width_power,
            min_chunk_width_power,
            max_split_count: 1 << max_split_count_power,
            max_split_count_power,
        }
    }
}

impl Default for LayoutConstraints {
    #[inline]
    fn default() -> Self {
        Self::new(MIN_CHUNK_WIDTH_POWER, MAX_SPLIT_COUNT_POWER)
    }
}

impl XLayout {
    /*
        #[inline(always)]
        pub(crate) fn index(&self, pos: i32) -> usize {
            (pos - self.range.min) as usize >> self.section_log_width
        }

        #[inline(always)]
        pub(crate) fn indices_by_range(&self, range: LineRange) -> (usize, usize) {
            let i0 = self.index(range.min);
            let i1 = self.index(range.max);
            (i0, i1)
        }

        #[inline(always)]
        pub(crate) fn left_border(&self, index: usize) -> i32 {
            self.full_rect.min_x + (index << self.section_log_width) as i32
        }

        #[inline(always)]
        pub(crate) fn borders(&self, index: usize) -> (i32, i32) {
            let left = self.left_border(index);
            let width = 1i32 << self.section_log_width;
            let max = left + width - 1;
            let right = max.min(self.full_rect.max_x);
            (left, right)
        }

        #[inline(always)]
        pub(crate) fn indices_by_xx(&self, x0: i32, x1: i32) -> (usize, usize) {
            let (min_x, max_x) = if x0 < x1 { (x0, x1) } else { (x1, x0) };

            let i0 = self.index(min_x);
            let i1 = self.index(max_x);
            (i0, i1)
        }
    */

    #[inline]
    pub(crate) fn new(
        range: LineRange,
        opt_chunks_count: usize
    ) -> Option<Self> {
        Self::with_constraints(range, opt_chunks_count, Default::default())
    }

    fn with_constraints(
        range: LineRange,
        opt_chunks_count: usize,
        constraints: LayoutConstraints,
    ) -> Option<Self> {
        let length = (range.max - range.min) as usize;
        if length <= constraints.min_chunk_width {
            return None;
        }

        let opt_count = opt_chunks_count.max(1).min(constraints.max_split_count);
        let chunk_width = length.div_ceil(opt_count);
        let chunk_width_power_by_length = length.ilog2_ceil().saturating_sub(constraints.min_chunk_width_power);
        let chunk_width_power_by_count = chunk_width.ilog2_ceil();

        let power = chunk_width_power_by_count
            .min(chunk_width_power_by_length)
            .max(constraints.min_chunk_width_power) as usize;

        let count = ((length - 1) >> power) + 1;
        if count <= 1 {
            None
        } else {
            Some(Self {
                range,
                power,
                count,
            })
        }
    }

    fn with_max_count(range: LineRange, constraints: LayoutConstraints) -> Self {
        let length = (range.max - range.min) as usize;
        let chunk_width = length >> constraints.max_split_count_power;
        let power = chunk_width.ilog2_ceil() as usize;
        let count = ((length - 1) >> power) + 1;

        Self {
            range,
            power,
            count,
        }
    }
}

trait RectAndCount {
    fn rect_and_count(&self) -> (IntRect, usize);
}

impl RectAndCount for [IntContour] {
    fn rect_and_count(&self) -> (IntRect, usize) {
        let mut rect = IntRect::new(i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        let mut count = 0;

        for contour in self.iter() {
            for p in contour {
                rect.add_point(p);
            }
            count += contour.len();
        }

        (rect, count)
    }
}

#[cfg(test)]
mod tests {
    use crate::geom::range::LineRange;
    use crate::tile::x_layout::{LayoutConstraints, XLayout};

    #[test]
    fn test_0() {
        let constraints = LayoutConstraints::new(1, 3);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 3), 2, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 4), 2, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 5), 2, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 6), 2, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 7), 2, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 8), 2, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 9), 2, constraints).unwrap();
        assert_eq!(layout.count, 2);
    }

    #[test]
    fn test_1() {
        let constraints = LayoutConstraints::new(1, 3);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 2), 3, constraints);
        assert!(layout.is_none());

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 4), 3, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 4), 3, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 5), 3, constraints).unwrap();
        assert_eq!(layout.count, 3);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 6), 3, constraints).unwrap();
        assert_eq!(layout.count, 3);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 7), 3, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 8), 3, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 9), 3, constraints).unwrap();
        assert_eq!(layout.count, 3);
    }

    #[test]
    fn test_2() {
        let constraints = LayoutConstraints::new(2, 4);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 4), 4, constraints);
        assert!(layout.is_none());

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 5), 4, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 7), 4, constraints).unwrap();
        assert_eq!(layout.count, 2);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 16), 4, constraints).unwrap();
        assert_eq!(layout.count, 4);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 15), 4, constraints).unwrap();
        assert_eq!(layout.count, 4);

        let layout = XLayout::with_constraints(LineRange::with_min_max(0, 17), 4, constraints).unwrap();
        assert_eq!(layout.count, 3);
    }

}
