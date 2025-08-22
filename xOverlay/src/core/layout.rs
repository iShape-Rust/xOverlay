use i_float::int::rect::IntRect;
use i_shape::int::count::PointsCount;
use i_shape::int::shape::IntContour;

#[derive(Clone)]
pub(crate) struct Layout {
    rect: IntRect,
    column_width_power: u32,
    columns_count: usize,
}

impl Layout {

    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        self.columns_count
    }

    #[inline(always)]
    pub(crate) fn index(&self, pos: i32) -> usize {
        (pos - self.rect.min_x) as usize >> self.column_width_power
    }

    #[inline(always)]
    pub(crate) fn index_inner_border_check(&self, pos: i32) -> (usize, bool) {
        let dx = (pos - self.rect.min_x) as usize;
        let i = dx >> self.column_width_power;
        let xi = i << self.column_width_power;
        let border = dx == xi;

        if border {
            (i - 1, i < self.columns_count)
        } else {
            (i, false)
        }
    }

    #[inline(always)]
    pub(crate) fn left_border(&self, index: usize) -> i32 {
        self.rect.min_x + (index << self.column_width_power) as i32
    }

    #[inline(always)]
    pub(crate) fn borders(&self, index: usize) -> (i32, i32) {
        let min = self.rect.min_x + (index << self.column_width_power) as i32;
        let max = (min + (1i32 << self.column_width_power)).min(self.rect.max_x);
        (min, max)
    }

    #[inline(always)]
    pub(crate) fn indices(&self, x0: i32, x1: i32) -> (usize, usize, bool) {
        let (min_x, max_x) = if x0 < x1 {
            (x0, x1)
        } else {
            (x1, x0)
        };

        let i0 = self.index(min_x);
        let (i1, border) = self.index_inner_border_check(max_x);
        (i0, i1, border)
    }

    #[inline]
    pub(crate) fn with_subj_and_clip(subj: &[IntContour], clip: &[IntContour], min_count_per_column_power: u32) -> Option<Self> {
        let subj_rect = IntRect::with_iter(subj.iter().flatten());
        let clip_rect = IntRect::with_iter(clip.iter().flatten());
        let rect = match (subj_rect, clip_rect) {
            (Some(r0), Some(r1)) => IntRect::with_rects(&r0, &r1),
            (Some(r0), None) => r0,
            (None, Some(r1)) => r1,
            _ => return None,
        };

        let width = 1 + rect.width() as u32;
        let count = (subj.points_count() + clip.points_count()) as u32;
        let count_per_column_power = (4 * count.isqrt()).ilog2().max(min_count_per_column_power);

        let columns_count_approx = (count >> count_per_column_power).max(1);
        let columns_count_max = width / 2;
        let columns_count_log = columns_count_approx.min(columns_count_max).ilog2();

        let column_width_power = width.ilog2_round_up() - columns_count_log;
        let column_width = 1 << column_width_power;

        let columns_count = ((width + column_width - 1) >> column_width_power) as usize;

        Some(Self {
            columns_count,
            rect,
            column_width_power,
        })
    }
}

trait Log {
    fn ilog2_round_up(&self) -> u32;
}

impl Log for u32 {
    #[inline(always)]
    fn ilog2_round_up(&self) -> u32 {
        (self - 1).ilog2() + 1
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            rect: IntRect::new(0, 0, 0, 0),
            column_width_power: 0,
            columns_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use i_float::int::point::IntPoint;
    use crate::core::layout::Layout;

    #[test]
    fn test_0() {
        let subj = [
            [
                IntPoint::new(0, 0),
                IntPoint::new(10, 0),
                IntPoint::new(10, 10),
                IntPoint::new(0, 10),
            ].to_vec()
        ];

        let layout = Layout::with_subj_and_clip(&subj, &[], 2).unwrap();

        assert_eq!(layout.columns_count, 1);
    }

    #[test]
    fn test_1() {
        let subj = [
            [
                IntPoint::new(0, 0),
                IntPoint::new(10, 0),
                IntPoint::new(10, 10),
                IntPoint::new(0, 10),
            ].to_vec()
        ];

        let layout = Layout::with_subj_and_clip(&subj, &[], 1).unwrap();

        assert_eq!(layout.columns_count, 2);
    }
}