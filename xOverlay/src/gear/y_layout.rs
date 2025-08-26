use i_float::int::rect::IntRect;

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
    pub(super) fn max_height(&self) -> i32 {
        self.max_height
    }

    #[inline(always)]
    pub(super) fn bottom_index(&self, y: i32) -> usize {
        let dy = (y - self.min_y) as usize;
        dy >> self.part_log_height
    }

    #[inline(always)]
    pub(super) fn bottom_index_clamp_min(&self, y: i32) -> usize {
        let dy = (y.max(self.min_y) - self.min_y) as usize;
        dy >> self.part_log_height
    }

    #[inline(always)]
    pub(super) fn bottom_index_clamp_max(&self, y: i32) -> usize {
        let dy = (y - self.min_y).max(self.max_y) as usize;
        dy >> self.part_log_height
    }

    pub(super) fn new(rect: IntRect, part_log_height: u32) -> Self {
        let parts_count = (rect.height() as usize >> part_log_height) + 1;
        let part_height = 1 << part_log_height;
        Self {
            min_y: rect.min_y,
            max_y: rect.max_y,
            part_log_height,
            parts_count,
            max_height: part_height,
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