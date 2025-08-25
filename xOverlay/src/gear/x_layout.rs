use i_float::int::rect::IntRect;
use i_shape::int::shape::IntContour;

#[derive(Clone)]
pub(crate) struct XLayout {
    rect: IntRect,
    part_log_width: u32,
    parts_count: usize,
}

impl XLayout {

    #[inline(always)]
    pub(crate) fn boundary(&self) -> &IntRect {
        &self.rect
    }

    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        self.parts_count
    }

    #[inline(always)]
    pub(super) fn log_width(&self) -> u32 {
        self.part_log_width
    }

    #[inline(always)]
    pub(crate) fn index(&self, pos: i32) -> usize {
        (pos - self.rect.min_x) as usize >> self.part_log_width
    }

    #[inline(always)]
    pub(crate) fn index_inner_border_check(&self, pos: i32) -> (usize, bool) {
        let dx = (pos - self.rect.min_x) as usize;
        let i = dx >> self.part_log_width;
        let xi = i << self.part_log_width;
        let border = dx == xi;

        if border {
            (i - 1, i < self.parts_count)
        } else {
            (i, false)
        }
    }

    #[inline(always)]
    pub(crate) fn left_border(&self, index: usize) -> i32 {
        self.rect.min_x + (index << self.part_log_width) as i32
    }

    #[inline(always)]
    pub(crate) fn borders(&self, index: usize) -> (i32, i32) {
        let min = self.rect.min_x + (index << self.part_log_width) as i32;
        let max = (min + (1i32 << self.part_log_width)).min(self.rect.max_x);
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

    pub(crate) fn with_rect(rect: IntRect, elements_count: usize, avg_count_per_column: usize, max_parts_count: usize) -> Self {
        let width = 1 + rect.width() as usize;

        let approximate_width = avg_count_per_column * width / elements_count;
        let part_log_width = approximate_width.ilog2();

        let part_width = 1 << part_log_width;
        let parts_count = ((rect.width() + part_width) >> part_log_width) as usize;

        if parts_count <= max_parts_count {
            Self {
                parts_count,
                rect,
                part_log_width,
            }
        } else {
            let exact_part_width = width / max_parts_count;
            let mut part_log_width = exact_part_width.ilog2();
            if 1 << part_log_width < width {
                part_log_width += 1;
            }

            let part_width = 1 << part_log_width;
            let parts_count = ((rect.width() + part_width) >> part_log_width) as usize;
            Self {
                parts_count,
                rect,
                part_log_width,
            }
        }
    }

    pub(crate) fn with_subj_and_clip(subj: &[IntContour], clip: &[IntContour], cpu_count: usize) -> Option<Self> {
        let subj_rect = IntRect::with_iter(subj.iter().flatten());
        let clip_rect = IntRect::with_iter(clip.iter().flatten());
        let rect = match (subj_rect, clip_rect) {
            (Some(r0), Some(r1)) => IntRect::with_rects(&r0, &r1),
            (Some(r0), None) => r0,
            (None, Some(r1)) => r1,
            _ => return None,
        };

        let part_log_width = if cpu_count == 1 {
            (2 * rect.width() - 1).ilog2()
        } else {
            let width = 1 + rect.width() as u32;
            let log_width_cpu = Self::part_log_width_by_cpu(width, cpu_count);
            let log_width_max = (width / 2).ilog2();

            log_width_cpu.min(log_width_max)
        };

        let part_width = 1 << part_log_width;
        let parts_count = ((rect.width() + part_width) >> part_log_width) as usize;

        Some(Self {
            parts_count,
            rect,
            part_log_width,
        })
    }

    fn part_log_width_by_cpu(width: u32, cpu_count: usize) -> u32 {
        let optimal_count = 3 * cpu_count;
        let optimal_count_log = optimal_count.ilog2();

        (width >> optimal_count_log).ilog2()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use i_float::int::point::IntPoint;
    use i_float::int::rect::IntRect;
    use crate::gear::x_layout::XLayout;

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

        let layout = XLayout::with_subj_and_clip(&subj, &[], 1).unwrap();

        assert_eq!(layout.parts_count, 1);
    }

    #[test]
    fn test_1() {
        let subj = [
            vec![
                IntPoint::new(0, 0),
                IntPoint::new(3, 0),
            ],
            vec![
                IntPoint::new(4, 0),
                IntPoint::new(7, 0),
            ]
        ];

        let layout = XLayout::with_subj_and_clip(&subj, &[], 1).unwrap();

        assert_eq!(layout.parts_count, 1);
    }

    #[test]
    fn test_2() {
        let rect = IntRect::new(0, 10, 0, 10);
        let layout = XLayout::with_rect(rect, 100, 80, 4);

        assert_eq!(layout.parts_count, 2);
    }

    #[test]
    fn test_3() {
        let rect = IntRect::new(0, 10, 0, 10);
        let layout = XLayout::with_rect(rect, 100, 80, 1);

        assert_eq!(layout.parts_count, 1);
    }

    #[test]
    fn test_4() {
        let rect = IntRect::new(0, 100, 0, 10);
        let layout = XLayout::with_rect(rect, 100, 20, 10);

        assert_eq!(layout.parts_count, 7);
    }
}