use crate::geom::range::LineRange;
use i_float::int::rect::IntRect;
use crate::util::log2::Log2;

#[derive(Clone)]
pub(crate) struct XLayout {
    section_rect: IntRect,
    column_log_width: u32,
    columns_count: usize,
}

impl XLayout {
    #[inline(always)]
    pub(crate) fn x_range(&self) -> LineRange {
        LineRange::with_min_max(self.section_rect.min_x, self.section_rect.max_x)
    }

    #[inline(always)]
    pub(crate) fn y_range(&self) -> LineRange {
        LineRange::with_min_max(self.section_rect.min_y, self.section_rect.max_y)
    }

    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        self.columns_count
    }

    #[inline(always)]
    pub(super) fn log_width(&self) -> u32 {
        self.column_log_width
    }

    #[inline(always)]
    pub(crate) fn index(&self, pos: i32) -> usize {
        (pos - self.section_rect.min_x) as usize >> self.column_log_width
    }

    #[inline(always)]
    pub(crate) fn left_border(&self, index: usize) -> i32 {
        self.section_rect.min_x + (index << self.column_log_width) as i32
    }

    #[inline(always)]
    pub(crate) fn borders(&self, index: usize) -> (i32, i32) {
        let left = self.left_border(index);
        let width = 1i32 << self.column_log_width;
        let max = left + width - 1;
        let right = max.min(self.section_rect.max_x);
        (left, right)
    }

    pub(crate) fn with_rect(
        section_rect: IntRect,
        elements_count: usize,
        min_count_per_column: usize,
        max_sections_count: usize,
    ) -> Self {

        let width = 1 + section_rect.width() as u32;

        if width == 1 {
            return Self {
                section_rect,
                column_log_width: 0,
                columns_count: 1,
            }
        }

        let max_log_column_width = width.ilog2_ceil();

        let optimal_columns_count = (elements_count >> min_count_per_column.ilog2()).min(max_sections_count).max(1);
        let optimal_column_width = width >> optimal_columns_count.ilog2();
        let column_log_width = optimal_column_width.ilog2().min(max_log_column_width);

        let count = width >> column_log_width;
        let columns_count = if (count << column_log_width) < width {
            count + 1
        } else {
            count
        } as usize;

        debug_assert!(width as usize <= (columns_count << column_log_width));

        Self {
            section_rect,
            column_log_width,
            columns_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gear::x_layout::XLayout;
    use i_float::int::rect::IntRect;

    #[test]
    fn test_0() {
        let rect = IntRect::new(0, 10, 0, 10);
        let layout = XLayout::with_rect(rect, 100, 80, 4);

        assert_eq!(layout.columns_count, 2);
    }

    #[test]
    fn test_1() {
        let rect = IntRect::new(0, 10, 0, 10);
        let layout = XLayout::with_rect(rect, 100, 80, 1);

        assert_eq!(layout.columns_count, 1);
    }

    #[test]
    fn test_2() {
        let rect = IntRect::new(0, 100, 0, 10);
        let layout = XLayout::with_rect(rect, 100, 20, 10);

        assert_eq!(layout.columns_count, 7);
    }
}
