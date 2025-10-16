use crate::geom::range::LineRange;
use crate::util::log2::Log2;

#[derive(Clone)]
pub(super) struct RowLayout {
    pub(super) range: LineRange,
    pub(super) count: usize,
    power: usize,
}

impl RowLayout {
    #[inline(always)]
    pub(super) fn index(&self, pos: i32) -> usize {
        debug_assert!(
            pos >= self.range.min && pos <= self.range.max,
            "pos out of TileLayout range"
        );
        (pos - self.range.min) as usize >> self.power
    }

    #[inline(always)]
    pub(super) fn step(&self) -> usize {
        1 << self.power
    }

    #[inline]
    pub(super) fn new(range: LineRange, opt_chunks_count: usize) -> Self {
        Self::with_constraints(range, opt_chunks_count, Default::default())
    }

    fn with_range_power_and_count(range: LineRange, power: usize, count: usize) -> Self {
        Self {
            range,
            power,
            count,
        }
    }

    fn with_constraints(
        range: LineRange,
        opt_chunks_count: usize,
        min_chunk_width_power: u32,
    ) -> Self {
        let length = (range.max - range.min) as usize;
        let min_chunk_width = 1 << min_chunk_width_power;
        if length <= min_chunk_width {
            return Self::with_range_power_and_count(range, min_chunk_width_power as usize, 1);
        }

        let opt_count = opt_chunks_count.max(1);
        let chunk_width = length.div_ceil(opt_count);
        let chunk_width_power_by_length = length.ilog2_ceil().saturating_sub(min_chunk_width_power);
        let chunk_width_power_by_count = chunk_width.ilog2_ceil();

        let power = chunk_width_power_by_count
            .min(chunk_width_power_by_length)
            .max(min_chunk_width_power) as usize;

        let count = ((length - 1) >> power) + 1;

        Self::with_range_power_and_count(range, power, count)
    }
}

#[cfg(test)]
mod tests {
    use crate::deg_90::layout::RowLayout;
    use crate::geom::range::LineRange;

    #[test]
    fn test_0() {
        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 3), 2, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 4), 2, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 5), 2, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 6), 2, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 7), 2, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 8), 2, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 9), 2, 1);
        assert_eq!(layout.count, 2);
    }

    #[test]
    fn test_1() {
        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 2), 3, 1);
        assert_eq!(layout.count, 1);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 4), 3, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 4), 3, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 5), 3, 1);
        assert_eq!(layout.count, 3);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 6), 3, 1);
        assert_eq!(layout.count, 3);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 7), 3, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 8), 3, 1);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 9), 3, 1);
        assert_eq!(layout.count, 3);
    }

    #[test]
    fn test_2() {
        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 4), 4, 2);
        assert_eq!(layout.count, 1);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 5), 4, 2);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 7), 4, 2);
        assert_eq!(layout.count, 2);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 16), 4, 2);
        assert_eq!(layout.count, 4);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 15), 4, 2);
        assert_eq!(layout.count, 4);

        let layout = RowLayout::with_constraints(LineRange::with_min_max(0, 17), 4, 2);
        assert_eq!(layout.count, 3);
    }
}
