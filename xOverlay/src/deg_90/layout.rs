use crate::core::cpu_count::CPUCount;
use crate::core::integer::OverlayInt;
use crate::deg_90::config::ColumnConfig90;
use crate::geom::range::LineRange;
use i_float::int::number::wide_int::WideIntNumber;

#[derive(Clone)]
pub(super) struct ColumnLayout<I: OverlayInt> {
    // X-range [min, max] covered by all columns
    pub(super) range: LineRange<I>,

    // number of columns produced for that range
    pub(super) count: usize,

    // log2(column_width)
    // column_width = 1 << power
    power: u32,
}

impl<I: OverlayInt> ColumnLayout<I> {
    #[inline(always)]
    fn count_for_power(range: LineRange<I>, power: u32) -> usize {
        let width = range.max.to_wide() - range.min.to_wide();
        if width <= I::Wide::ZERO {
            return 1;
        }

        let count = range.max.shifted_distance(range.min, power as usize);
        if power == 0 {
            return count.max(1);
        }

        let truncated = (width >> power) << power;
        count.saturating_add((truncated != width) as usize).max(1)
    }

    #[inline(always)]
    pub(super) fn index(&self, pos: I) -> usize {
        pos.shifted_distance(self.range.min, self.power as usize)
    }

    #[inline(always)]
    pub(super) fn index_round_down(&self, pos: I) -> usize {
        if pos <= self.range.min {
            0
        } else {
            (pos - I::ONE).shifted_distance(self.range.min, self.power as usize)
        }
    }

    #[inline(always)]
    pub(super) fn left_border(&self, index: usize) -> I {
        let dx = I::Wide::from_usize(index) << self.power;
        I::from_wide(self.range.min.to_wide() + dx)
    }

    #[inline]
    pub(super) fn capped_left_border(&self, index: usize) -> I {
        let dx = I::Wide::from_usize(index) << self.power;
        let value = self.range.min.to_wide() + dx;
        if value > I::MAX.to_wide() {
            I::MAX
        } else {
            I::from_wide(value)
        }
    }

    #[cfg(test)]
    #[inline(always)]
    pub(super) fn step(&self) -> usize {
        1 << self.power
    }

    pub(super) fn with_segments_count(
        segments_count: usize, // total segments we plan to drop here
        range: LineRange<I>,   // x: [min, max]
        cpu_count: CPUCount,
        config: ColumnConfig90,
    ) -> Self {
        // 1) how many columns we *could* run in parallel at most
        let cpus = cpu_count.count();

        // 2) maximum columns we can have by density constraints
        let max_possible_by_density = if config.min_allowed_segments_per_column > 0 {
            segments_count.div_ceil(config.min_allowed_segments_per_column)
        } else {
            1
        };

        // 3) minimum columns we must have by density constraints
        let min_required_by_density = if config.max_allow_segments_per_column > 0 {
            segments_count.div_ceil(config.max_allow_segments_per_column)
        } else {
            1
        };

        // 4) geometry bound:
        //    we cannot have more columns than we can physically fit
        //    if the minimal column_graph width is 2^min_column_width_power,
        //    then width / min_step is the max number of columns we can place
        let width = range.max.to_wide() - range.min.to_wide();
        let min_power = config.min_column_width_power as u32;
        if width <= I::Wide::ZERO {
            return Self {
                range,
                count: 1,
                power: min_power,
            };
        }
        let max_possible_by_geometry = Self::count_for_power(range, min_power);

        // now combine:
        // - we want max_possible_by_density
        // - but not more than CPUs
        // - but not less min_required_by_density
        // - but not more than geometry allows
        // - and not less than at least columns required by config
        let count_hint = max_possible_by_density
            .min(cpus)
            .max(min_required_by_density)
            .min(max_possible_by_geometry)
            .max(config.min_columns_count)
            .max(1);

        // This is the "ideal" column_graph width for that hint.
        let divisor = I::Wide::from_usize(count_hint);
        let column_width = (width + divisor - I::Wide::ONE) / divisor;

        // We store width as a power-of-two shift.
        // Here we ROUND DOWN to nearest power of two, but never below the config floor.
        let power = column_width.ilog2().max(min_power);

        // Final column_graph count:
        // step = 1 << power
        // count = ceil(width / step)
        let count = Self::count_for_power(range, power);

        Self {
            range,
            count,
            power,
        }
    }

    pub(super) fn with_max_segments_in_line(
        max_segments_in_line: usize,
        range: LineRange<I>,
        config: ColumnConfig90,
    ) -> Option<Self> {
        if max_segments_in_line <= config.max_allowed_segments_per_line {
            return None;
        }

        let required = max_segments_in_line.div_ceil(config.max_allowed_segments_per_line);

        let width = range.max.to_wide() - range.min.to_wide();
        if width <= I::Wide::ZERO {
            return None;
        }
        let min_power = config.min_column_width_power as u32;
        let max_possible_by_geometry = Self::count_for_power(range, min_power);

        let count_hint = required.min(max_possible_by_geometry);

        if count_hint <= 1 {
            return None;
        }

        // This is the "ideal" column_graph width for that hint.
        let divisor = I::Wide::from_usize(count_hint);
        let column_width = (width + divisor - I::Wide::ONE) / divisor;

        // We store width as a power-of-two shift.
        // Here we ROUND DOWN to nearest power of two, but never below the config floor.
        let power = column_width.ilog2().max(min_power);

        // Final column_graph count:
        // step = 1 << power
        // count = ceil(width / step)
        let count = Self::count_for_power(range, power);

        if count <= 1 {
            return None;
        }

        Some(Self {
            range,
            count,
            power,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_range_and_count(range: LineRange<I>, count: usize) -> Self {
        assert!(count > 0, "column count must be greater than zero");

        let width = range.max.to_wide() - range.min.to_wide();
        assert!(width > I::Wide::ZERO, "range must have positive width");

        for power in 0..I::BITS {
            let calculated_count = Self::count_for_power(range, power);
            if calculated_count == count {
                return Self {
                    range,
                    count,
                    power,
                };
            }
        }

        panic!(
            "column count {} is not representable for range width {}",
            count, width
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cpu_count::CPUCount;
    use crate::deg_90::config::ColumnConfig90;
    use crate::geom::range::LineRange;

    #[test]
    fn with_config_respects_min_column_hint() {
        let range = LineRange::with_min_max(0, 512);
        let config = ColumnConfig90::dev(4);

        let layout = ColumnLayout::with_segments_count(10_000, range, CPUCount::Single, config);

        assert_eq!(layout.count, 4);
        assert_eq!(layout.step(), 128);
        assert_eq!(layout.left_border(0), 0);
        assert_eq!(layout.left_border(layout.count - 1), 384);

        let step = layout.step() as i32;
        assert_eq!(layout.index(range.min), 0);
        assert_eq!(layout.index(range.min + step - 1), 0);
        assert_eq!(layout.index(range.min + step), 1);
        assert_eq!(layout.index_round_down(range.max), layout.count - 1);
        assert_eq!(layout.index_round_down(range.max - 1), layout.count - 1);
    }

    #[test]
    fn indexing_handles_negative_coordinates() {
        let range = LineRange::with_min_max(-256, 256);
        let config = ColumnConfig90::dev(8);

        let layout = ColumnLayout::with_segments_count(0, range, CPUCount::Single, config);
        let step = layout.step() as i32;

        assert_eq!(layout.count, 8);
        assert_eq!(step, 64);

        for idx in 0..layout.count {
            let expected_left = range.min + (idx as i32) * step;
            assert_eq!(layout.left_border(idx), expected_left);
        }

        assert_eq!(layout.left_border(layout.count - 1) + step, range.max);
        assert_eq!(layout.index(range.min), 0);
        assert_eq!(layout.index(range.min + step - 1), 0);
        assert_eq!(layout.index(range.min + step), 1);
        assert_eq!(layout.index(range.max - 1), layout.count - 1);
        assert_eq!(layout.index_round_down(range.min), 0);
        assert_eq!(layout.index_round_down(range.max), layout.count - 1);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn i64_layout_handles_range_wider_than_usize_on_32_bit_targets() {
        let range = LineRange::with_min_max(i64::MIN + 1, i64::MAX);
        let layout =
            ColumnLayout::with_segments_count(0, range, CPUCount::Single, ColumnConfig90::dev(8));

        assert_eq!(layout.count, 8);
        assert_eq!(layout.index(range.min), 0);
        assert_eq!(layout.index(range.max - 1), 7);
        assert_eq!(layout.index_round_down(range.max), 7);
        for index in 1..layout.count {
            assert!(layout.left_border(index - 1) < layout.left_border(index));
        }
    }

    #[test]
    fn test_constructor_with_range_and_count() {
        let range = LineRange::with_min_max(0, 512);
        let layout = ColumnLayout::with_range_and_count(range, 8);

        assert_eq!(layout.count, 8);
        assert_eq!(layout.step(), 64);
        assert_eq!(layout.left_border(layout.count - 1), 448);
        assert_eq!(layout.index(range.max - 1), layout.count - 1);
    }

    #[test]
    #[should_panic(expected = "is not representable")]
    fn test_constructor_with_range_and_count_panics_for_unrepresentable_count() {
        let range = LineRange::with_min_max(0, 512);
        let _ = ColumnLayout::with_range_and_count(range, 3);
    }
}
