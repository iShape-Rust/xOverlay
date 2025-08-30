use crate::core::options::IntOverlayOptions;
use crate::core::overlay::{Overlay, OverlayError};
use crate::core::shape_type::ShapeType;
use crate::core::solver::Solver;
use crate::core::winding::WindingCount;
use crate::gear::s_mapper::SMapper;
use crate::gear::section::Section;
use crate::gear::s_layout::SLayout;
use crate::gear::seg_iter::{DropCollinear, SegmentIterable};
use crate::gear::segment::Segment;
use crate::gear::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_float::int::rect::IntRect;
use i_shape::int::shape::IntContour;

impl Overlay {
    pub(crate) fn init_contours_custom(
        subj: &[IntContour],
        clip: &[IntContour],
        options: IntOverlayOptions,
        solver: Solver,
    ) -> Result<Self, OverlayError> {
        let layout = SLayout::with_subj_and_clip(subj, clip, solver.cpu_count());

        let mut mapper = SMapper::new(layout);

        mapper.add_contours(subj);
        mapper.add_contours(clip);

        let count = mapper.layout.count();
        let mut sections = Vec::with_capacity(count);

        for (i, part) in mapper.iter_by_parts().enumerate() {
            let (min_x, max_x) = mapper.layout.borders(i);
            let rect = IntRect {
                min_x,
                max_x,
                min_y: mapper.layout.boundary().min_y,
                max_y: mapper.layout.boundary().max_y,
            };
            sections.push(Section::new(
                rect,
                part,
                options.avg_count_per_column,
                options.max_parts_count,
            ))
        }

        let mut overlay = Self {
            options,
            solver,
            sections,
        };

        overlay.add_contours(&mapper.layout, subj, ShapeType::Subject)?;
        overlay.add_contours(&mapper.layout, clip, ShapeType::Clip)?;

        Ok(overlay)
    }

    fn add_contours(
        &mut self,
        layout: &SLayout,
        contours: &[IntContour],
        shape_type: ShapeType,
    ) -> Result<(), OverlayError> {
        let (direct, invert) = ShapeCountBoolean::with_shape_type(shape_type);

        for contour in contours.iter() {
            self.add_contour(layout, contour, direct, invert)?;
        }

        Ok(())
    }

    #[inline]
    fn add_contour(
        &mut self,
        layout: &SLayout,
        contour: &[IntPoint],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> Result<(), OverlayError> {
        let iter = if let Some(result) = contour.segment_iter::<DropCollinear>() {
            result
        } else {
            return Ok(());
        };

        for s in iter {
            _ = self.add_segment(layout, s, direct, invert);
        }

        Ok(())
    }

    #[inline]
    fn add_segment(
        &mut self,
        layout: &SLayout,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) {
        if segment[0].x == segment[1].x {
            self.add_vertical(layout, segment, direct, invert);
        } else if segment[0].y == segment[1].y {
            self.add_horizontal(layout, segment, direct, invert);
        } else {
            self.add_diagonal(layout, segment, direct, invert);
        }
    }

    #[inline]
    fn add_vertical(
        &mut self,
        layout: &SLayout,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) {
        let x0 = segment[0].x;

        // vertical
        let (range, dir) = segment.y_range(direct, invert);
        let index = layout.index(x0);
        unsafe {
            self.sections
                .get_unchecked_mut(index)
                .source
                .vr_list
                .push(Segment {
                    pos: x0,
                    range,
                    dir,
                });
        }
    }

    #[inline]
    fn add_horizontal(
        &mut self,
        layout: &SLayout,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) {
        let y0 = segment[0].y;

        let (range, dir) = segment.x_range(direct, invert);
        let (i0, i1) = layout.indices_by_range(range);

        let mut x0 = range.min;

        for index in i0..i1 {
            let xi = layout.left_border(index + 1);
            unsafe {
                self.sections
                    .get_unchecked_mut(index)
                    .source
                    .hz_list
                    .push(Segment {
                        pos: y0,
                        range: LineRange::with_min_max(x0, xi),
                        dir,
                    });
            }
            x0 = xi
        }

        // add last
        unsafe {
            self.sections
                .get_unchecked_mut(i1)
                .source
                .hz_list
                .push(Segment {
                    pos: y0,
                    range: LineRange::with_min_max(x0, range.max),
                    dir,
                });
        }
    }

    #[inline]
    fn add_diagonal(
        &mut self,
        layout: &SLayout,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) {
        let (a, b, dir) = segment.xy_range(direct, invert);
        let i0 = layout.index(a.x);
        let i1 = layout.index(b.x);

        let mut x0 = a.x;

        if a.y < b.y {
            // positive diagonal

            let y0 = a.y;
            let mut yi = y0;

            for index in i0..i1 {
                let xi = layout.left_border(index + 1);
                let dx = xi.wrapping_sub(a.x);
                unsafe {
                    self.sections
                        .get_unchecked_mut(index)
                        .source
                        .dp_list
                        .push(Segment {
                            pos: yi,
                            range: LineRange::with_min_max(x0, xi),
                            dir,
                        });
                }
                yi = y0.wrapping_add(dx);
                x0 = xi
            }

            // add last
            unsafe {
                self.sections
                    .get_unchecked_mut(i1)
                    .source
                    .dp_list
                    .push(Segment {
                        pos: yi,
                        range: LineRange::with_min_max(x0, b.x),
                        dir,
                    });
            }
        } else {
            // negative diagonal

            let y0 = b.y;
            let mut yi = y0;

            for index in i0..i1 {
                let xi = layout.left_border(index + 1);
                let dx = xi.wrapping_sub(a.x);
                unsafe {
                    self.sections
                        .get_unchecked_mut(index)
                        .source
                        .dn_list
                        .push(Segment {
                            pos: yi,
                            range: LineRange::with_min_max(x0, xi),
                            dir,
                        });
                }
                yi = y0.wrapping_sub(dx);
                x0 = xi
            }

            // add last
            unsafe {
                self.sections
                    .get_unchecked_mut(i1)
                    .source
                    .dn_list
                    .push(Segment {
                        pos: yi,
                        range: LineRange::with_min_max(x0, b.x),
                        dir,
                    });
            }
        }
    }
}

pub(crate) trait XYMinMaxRange {
    fn x_range(
        &self,
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> (LineRange, ShapeCountBoolean);
    fn y_range(
        &self,
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> (LineRange, ShapeCountBoolean);

    fn xy_range(
        &self,
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> (IntPoint, IntPoint, ShapeCountBoolean);
}

impl XYMinMaxRange for [IntPoint; 2] {
    #[inline(always)]
    fn x_range(
        &self,
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> (LineRange, ShapeCountBoolean) {
        if self[0].x < self[1].x {
            (LineRange::with_min_max(self[0].x, self[1].x), direct)
        } else {
            (LineRange::with_min_max(self[1].x, self[0].x), invert)
        }
    }

    #[inline(always)]
    fn y_range(
        &self,
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> (LineRange, ShapeCountBoolean) {
        if self[0].y < self[1].y {
            (LineRange::with_min_max(self[0].y, self[1].y), direct)
        } else {
            (LineRange::with_min_max(self[1].y, self[0].y), invert)
        }
    }

    #[inline(always)]
    fn xy_range(
        &self,
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> (IntPoint, IntPoint, ShapeCountBoolean) {
        if self[0].x < self[1].x {
            (self[0], self[1], direct)
        } else {
            (self[1], self[0], invert)
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::core::overlay::Overlay;
    use crate::core::shape_type::ShapeType;
    use crate::core::shape_type::ShapeType::Subject;
    use crate::core::solver::Solver;
    use crate::core::winding::WindingCount;
    use crate::gear::segment::Segment;
    use crate::gear::winding_count::ShapeCountBoolean;
    use crate::geom::range::LineRange;
    use alloc::vec;
    use i_float::int::point::IntPoint;
    use std::collections::HashSet;

    impl Segment {
        pub(crate) fn test_with_shape(z0: i32, z1: i32, pos: i32, shape: ShapeType) -> Self {
            let (direct, invert) = ShapeCountBoolean::with_shape_type(shape);
            let (range, dir) = if z0 < z1 {
                (LineRange::with_min_max(z0, z1), direct)
            } else {
                (LineRange::with_min_max(z1, z0), invert)
            };
            Self { pos, range, dir }
        }
    }

    #[test]
    fn test_0() {
        let subj = [vec![
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]];

        let overlay =
            Overlay::with_contours_custom(&subj, &[], Default::default(), Solver::single())
                .expect("valid path");

        assert_eq!(overlay.sections.len(), 1);
        let section = &overlay.sections[0];

        let must_be_hz_set: HashSet<_> = [
            Segment::test_with_shape(0, 10, 0, Subject),
            Segment::test_with_shape(10, 0, 10, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let must_be_vr_set: HashSet<_> = [
            Segment::test_with_shape(10, 0, 0, Subject),
            Segment::test_with_shape(0, 10, 10, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let value_hz_set: HashSet<_> = section.source.hz_list.iter().copied().collect();
        let value_vr_set: HashSet<_> = section.source.vr_list.iter().copied().collect();

        assert_eq!(must_be_hz_set, value_hz_set);
        assert_eq!(must_be_vr_set, value_vr_set);
    }

    #[test]
    fn test_1() {
        let subj = [vec![
            IntPoint::new(0, 0),
            IntPoint::new(5, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 5),
            IntPoint::new(10, 10),
            IntPoint::new(5, 10),
            IntPoint::new(0, 10),
            IntPoint::new(0, 5),
        ]];

        let overlay =
            Overlay::with_contours_custom(&subj, &[], Default::default(), Solver::single())
                .expect("valid path");

        assert_eq!(overlay.sections.len(), 1);
        let column = &overlay.sections[0];

        let must_be_hz_set: HashSet<_> = [
            Segment::test_with_shape(0, 10, 0, Subject),
            Segment::test_with_shape(10, 0, 10, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let must_be_vr_set: HashSet<_> = [
            Segment::test_with_shape(10, 0, 0, Subject),
            Segment::test_with_shape(0, 10, 10, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let value_hz_set: HashSet<_> = column.source.hz_list.iter().copied().collect();
        let value_vr_set: HashSet<_> = column.source.vr_list.iter().copied().collect();

        assert_eq!(must_be_hz_set, value_hz_set);
        assert_eq!(must_be_vr_set, value_vr_set);
    }

    #[test]
    fn test_2() {
        let subj = [vec![
            IntPoint::new(0, 1),
            IntPoint::new(1, 0),
            IntPoint::new(2, 0),
            IntPoint::new(3, 1),
            IntPoint::new(3, 2),
            IntPoint::new(2, 3),
            IntPoint::new(1, 3),
            IntPoint::new(0, 2),
        ]];

        let overlay =
            Overlay::with_contours_custom(&subj, &[], Default::default(), Solver::single())
                .expect("valid path");

        assert_eq!(overlay.sections.len(), 1);
        let section = &overlay.sections[0];

        let must_be_hz_set: HashSet<_> = [
            Segment::test_with_shape(2, 1, 3, Subject),
            Segment::test_with_shape(1, 2, 0, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let must_be_vr_set: HashSet<_> = [
            Segment::test_with_shape(2, 1, 0, Subject),
            Segment::test_with_shape(1, 2, 3, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let must_be_dg_pos_set: HashSet<_> = [
            Segment::test_with_shape(2, 3, 0, Subject),
            Segment::test_with_shape(1, 0, 2, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let must_be_dg_neg_set: HashSet<_> = [
            Segment::test_with_shape(0, 1, 0, Subject),
            Segment::test_with_shape(3, 2, 2, Subject),
        ]
        .iter()
        .copied()
        .collect();

        let value_hz_set: HashSet<_> = section.source.hz_list.iter().copied().collect();
        let value_vr_set: HashSet<_> = section.source.vr_list.iter().copied().collect();
        let value_dg_pos_set: HashSet<_> = section.source.dp_list.iter().copied().collect();
        let value_dg_neg_set: HashSet<_> = section.source.dn_list.iter().copied().collect();

        assert_eq!(must_be_hz_set, value_hz_set);
        assert_eq!(must_be_vr_set, value_vr_set);
        assert_eq!(must_be_dg_pos_set, value_dg_pos_set);
        assert_eq!(must_be_dg_neg_set, value_dg_neg_set);
    }
}
