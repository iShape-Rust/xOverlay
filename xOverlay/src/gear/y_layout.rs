use i_float::int::rect::IntRect;

#[derive(Clone)]
pub(super) struct YLayout {
    min_y: i32,
    max_y: i32,
    part_log_height: u32,
    parts_count: usize,
    single_shift_height: i32,
    double_shift_height: i32,
}

impl YLayout {

    #[inline(always)]
    pub(super) fn count(&self) -> usize {
        self.parts_count
    }

    #[inline(always)]
    pub(super) fn bottom_index(&self, y: i32) -> usize {
        let y0 = y - self.min_y;
        y0 as usize >> self.part_log_height
    }

    #[inline(always)]
    pub(super) fn top_unit_index(&self, y: i32) -> usize {
        let y0 = y - self.min_y;
        let y1 = (y0 + 1).min(self.max_y);
        y1 as usize >> self.part_log_height
    }

    #[inline(always)]
    pub(super) fn top_single_index(&self, y: i32) -> usize {
        let y0 = y - self.min_y;
        let y1 = (y0 + self.single_shift_height).min(self.max_y);
        y1 as usize >> self.part_log_height
    }

    #[inline(always)]
    pub(super) fn top_double_index(&self, y: i32) -> usize {
        let y0 = y - self.min_y;
        let y1 = (y0 + self.double_shift_height).min(self.max_y);
        y1 as usize >> self.part_log_height
    }

    pub(super) fn new(rect: IntRect, part_log_height: u32) -> Self {
        let parts_count = (rect.height() as usize >> part_log_height) + 1;
        let part_height = 1 << part_log_height;
        Self {
            min_y: rect.min_y,
            max_y: rect.max_y,
            part_log_height,
            parts_count,
            single_shift_height: part_height + 1,
            double_shift_height: 2 * part_height + 1
        }

    }
}

#[cfg(test)]
mod tests {
    use i_float::int::rect::IntRect;
    use crate::gear::y_layout::YLayout;

    #[test]
    fn test_0() {
        let rect = IntRect::new(0, 8, 0, 16);
        let layout = YLayout::new(rect, 4);

        assert_eq!(layout.parts_count, 2);
    }

}