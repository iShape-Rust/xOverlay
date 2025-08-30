use crate::geom::range::LineRange;
use i_float::int::rect::IntRect;
use i_shape::int::shape::IntContour;
use crate::util::log2::Log2;

#[derive(Clone)]
pub(crate) struct SLayout {
    full_rect: IntRect,
    section_log_width: u32,
    sections_count: usize,
}

impl SLayout {
    #[inline(always)]
    pub(crate) fn boundary(&self) -> &IntRect {
        &self.full_rect
    }

    #[inline(always)]
    pub(crate) fn count(&self) -> usize {
        self.sections_count
    }


    #[inline(always)]
    pub(crate) fn index(&self, pos: i32) -> usize {
        (pos - self.full_rect.min_x) as usize >> self.section_log_width
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

    pub(crate) fn with_subj_and_clip(
        subj: &[IntContour],
        clip: &[IntContour],
        optimal_sections_count: usize,
    ) -> Self {
        let (subj_rect, subj_count) = subj.rect_and_count();
        let (clip_rect, clip_count) = clip.rect_and_count();

        let rect = IntRect::with_rects(&subj_rect, &clip_rect);
        let items_count = subj_count + clip_count;

        let width = 1 + rect.width() as u32;
        let max_log_section_width = width.ilog2_ceil();

        if items_count <= 4 || width <= 4 {
            // degenerate case
            return Self {
                full_rect: rect,
                section_log_width: max_log_section_width,
                sections_count: 1,
            };
        }

        if optimal_sections_count == 1 {
            return Self {
                section_log_width: max_log_section_width,
                full_rect: rect,
                sections_count: 1,
            };
        }

        let optimal_sections_log_count = (optimal_sections_count as u32).ilog2_ceil();
        let optimal_section_width = width >> optimal_sections_log_count;
        let log_optimal_section_width = optimal_section_width.ilog2();
        let section_log_width = log_optimal_section_width.min(max_log_section_width);

        let count = width >> section_log_width;
        let sections_count = if (count << section_log_width) < width {
            count + 1
        } else {
            count
        } as usize;

        Self {
            sections_count,
            full_rect: rect,
            section_log_width,
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
    use alloc::vec;
    use i_float::int::point::IntPoint;
    use crate::gear::s_layout::SLayout;

    #[test]
    fn test_0() {
        let subj = [vec![
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]];

        let layout = SLayout::with_subj_and_clip(&subj, &[], 1);

        assert_eq!(layout.count(), 1);
    }

    #[test]
    fn test_1() {
        let subj = [
            vec![IntPoint::new(0, 0), IntPoint::new(3, 0)],
            vec![IntPoint::new(4, 0), IntPoint::new(7, 0)],
        ];
        let layout = SLayout::with_subj_and_clip(&subj, &[], 1);

        assert_eq!(layout.count(), 1);
    }

    #[test]
    fn test_2() {
        let subj = [
            vec![IntPoint::new(0, 0), IntPoint::new(0, 1)],
            vec![IntPoint::new(1, 0), IntPoint::new(1, 1)],
            vec![IntPoint::new(2, 0), IntPoint::new(2, 1)],
            vec![IntPoint::new(3, 0), IntPoint::new(3, 1)],
        ];
        let layout = SLayout::with_subj_and_clip(&subj, &[], 2);

        assert_eq!(layout.count(), 1);
    }

    #[test]
    fn test_3() {
        let subj = [
            vec![IntPoint::new(0, 0), IntPoint::new(0, 1)],
            vec![IntPoint::new(1, 0), IntPoint::new(1, 1)],
            vec![IntPoint::new(2, 0), IntPoint::new(2, 1)],
            vec![IntPoint::new(3, 0), IntPoint::new(3, 1)],
            vec![IntPoint::new(4, 0), IntPoint::new(4, 1)],
        ];
        let layout = SLayout::with_subj_and_clip(&subj, &[], 2);

        assert_eq!(layout.count(), 3);
    }

    #[test]
    fn test_4() {
        let subj = [
            vec![IntPoint::new(0, 0), IntPoint::new(0, 1)],
            vec![IntPoint::new(1, 0), IntPoint::new(1, 1)],
            vec![IntPoint::new(2, 0), IntPoint::new(2, 1)],
            vec![IntPoint::new(3, 0), IntPoint::new(3, 1)],
            vec![IntPoint::new(4, 0), IntPoint::new(4, 1)],
            vec![IntPoint::new(5, 0), IntPoint::new(5, 1)],
            vec![IntPoint::new(6, 0), IntPoint::new(6, 1)],
            vec![IntPoint::new(7, 0), IntPoint::new(7, 1)],
        ];
        let layout = SLayout::with_subj_and_clip(&subj, &[], 2);

        assert_eq!(layout.count(), 2);
    }

    #[test]
    fn test_5() {
        let subj = [
            vec![IntPoint::new(0, 0), IntPoint::new(0, 1)],
            vec![IntPoint::new(1, 0), IntPoint::new(1, 1)],
            vec![IntPoint::new(2, 0), IntPoint::new(2, 1)],
            vec![IntPoint::new(3, 0), IntPoint::new(3, 1)],
            vec![IntPoint::new(4, 0), IntPoint::new(4, 1)],
            vec![IntPoint::new(5, 0), IntPoint::new(5, 1)],
            vec![IntPoint::new(6, 0), IntPoint::new(6, 1)],
            vec![IntPoint::new(7, 0), IntPoint::new(7, 1)],
            vec![IntPoint::new(7, 0), IntPoint::new(8, 1)],
        ];
        let layout = SLayout::with_subj_and_clip(&subj, &[], 2);

        assert_eq!(layout.count(), 3);
    }
}
