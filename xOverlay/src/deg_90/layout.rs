use crate::core::cpu_count::CPUCount;
use crate::deg_90::config::ColumnConfig90;
use crate::geom::range::LineRange;

#[derive(Clone)]
pub(super) struct ColumnLayout {
    // X-range [min, max] covered by all columns
    pub(super) range: LineRange,

    // number of columns produced for that range
    pub(super) count: usize,

    // log2(column_width)
    // column_width = 1 << power
    power: usize,
}

impl ColumnLayout {
    #[inline(always)]
    fn distance(&self, pos: i32) -> usize {
        (pos - self.range.min) as usize
    }

    #[inline(always)]
    pub(super) fn index(&self, pos: i32) -> usize {
        self.distance(pos) >> self.power
    }

    #[inline(always)]
    pub(super) fn index_round_down(&self, pos: i32) -> usize {
        self.distance(pos).saturating_sub(1) >> self.power
    }

    #[inline(always)]
    pub(super) fn left_border(&self, index: usize) -> i32 {
        let dx = (index << self.power) as i32;
        self.range.min + dx
    }

    #[inline(always)]
    pub(super) fn step(&self) -> usize {
        1 << self.power
    }

    pub(super) fn with_segments_count(
        segments_count: usize, // total segments we plan to drop here
        range: LineRange,      // x: [min, max]
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
        //    if the minimal column width is 2^min_column_width_power,
        //    then width / min_step is the max number of columns we can place
        let width = (range.max - range.min) as usize;
        let min_step = 1usize << config.min_column_width_power;
        let max_possible_by_geometry = width.div_ceil(min_step);

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

        // This is the "ideal" column width for that hint.
        let column_width = width.div_ceil(count_hint);

        // We store width as a power-of-two shift.
        // Here we ROUND DOWN to nearest power of two, but never below the config floor.
        let power = (column_width.ilog2() as usize).max(config.min_column_width_power);

        // Final column count:
        // step = 1 << power
        // count = ceil(width / step)
        let count = (width.saturating_sub(1) >> power) + 1;

        Self {
            range,
            count,
            power,
        }
    }

    pub(super) fn with_max_segments_in_line(
        max_segments_in_line: usize,
        range: LineRange,
        config: ColumnConfig90,
    ) -> Option<Self> {
        if max_segments_in_line <= config.max_allowed_segments_per_line {
            return None;
        }

        let required = max_segments_in_line.div_ceil(config.max_allowed_segments_per_line);

        let width = (range.max - range.min) as usize;
        let min_step = 1usize << config.min_column_width_power;
        let max_possible_by_geometry = width.div_ceil(min_step);

        let count_hint = required.min(max_possible_by_geometry);

        if count_hint <= 1 {
            return None;
        }

        // This is the "ideal" column width for that hint.
        let column_width = width.div_ceil(count_hint);

        // We store width as a power-of-two shift.
        // Here we ROUND DOWN to nearest power of two, but never below the config floor.
        let power = (column_width.ilog2() as usize).max(config.min_column_width_power);

        // Final column count:
        // step = 1 << power
        // count = ceil(width / step)
        let count = (width.saturating_sub(1) >> power) + 1;

        if count <= 1 {
            return None;
        }

        Some(Self {
            range,
            count,
            power,
        })
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
}
