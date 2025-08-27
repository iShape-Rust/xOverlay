use core::ops::Range;
use i_float::int::rect::IntRect;
use crate::geom::range::LineRange;

#[derive(Clone)]
pub(super) struct YLayout {
    min_y: i32,
    max_y: i32,
    part_log_height: u32,
    parts_count: usize,
    max_height: i32
}

impl YLayout {

    #[inline(always)]
    pub(super) fn count(&self) -> usize {
        self.parts_count
    }

    #[inline(always)]
    pub(super) fn index(&self, y: i32) -> usize {
        let dy = (y - self.min_y) as usize;
        dy >> self.part_log_height
    }

    #[inline(always)]
    pub(super) fn indices_bottom_offset(&self, y: i32) -> Range<usize> {
        let start = self.index_clamp_min(y - self.max_height);
        let end = self.index_clamp_max(y);
        start..end
    }

    #[inline(always)]
    pub(super) fn indices_by_range(&self, range: LineRange) -> Range<usize> {
        let start = self.index(range.min);
        let end = self.index_clamp_max(range.max);
        start..end
    }

    #[inline(always)]
    pub(super) fn indices_by_range_bottom_offset(&self, range: LineRange) -> Range<usize> {
        let start = self.index_clamp_min(range.min - self.max_height);
        let end = self.index_clamp_max(range.max);
        start..end
    }

    #[inline(always)]
    fn index_clamp_min(&self, y: i32) -> usize {
        let dy = (y.max(self.min_y) - self.min_y) as usize;
        dy >> self.part_log_height
    }

    #[inline(always)]
    fn index_clamp_max(&self, y: i32) -> usize {
        let dy = (y.min(self.max_y) - self.min_y) as usize;
        dy >> self.part_log_height
    }

    pub(super) fn new(y_range: LineRange, part_log_height: u32) -> Self {
        let height = y_range.max.abs_diff(y_range.min) as usize;
        let parts_count = (height >> part_log_height) + 1;
        let part_height = 1 << part_log_height;
        Self {
            min_y: y_range.min,
            max_y: y_range.max,
            part_log_height,
            parts_count,
            max_height: part_height,
        }

    }
}

#[cfg(test)]
mod tests {
    use crate::gear::y_layout::YLayout;
    use crate::geom::range::LineRange;

    #[test]
    fn test_0() {
        let layout = YLayout::new(LineRange::with_min_max(0, 16), 4);

        assert_eq!(layout.parts_count, 2);
    }

}