use i_float::int::point::IntPoint;
use i_shape::int::shape::IntContour;
use crate::core::layout::Layout;
use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;
use crate::graph::boolean::winding_count::ShapeCountBoolean;
use crate::ortho::error::OrthoError;
use crate::ortho::mapper::Mapper;
use crate::ortho::orientation::Orientation;
use crate::ortho::overlay::OrthoOverlay;
use crate::ortho::segment::OrthoSegment;
use crate::sub::seg_iter::{DropCollinear, SegmentIterable};

const MIN_COUNT_PER_COLUMN_POWER: u32 = 6;

impl OrthoOverlay<ShapeCountBoolean> {
    pub fn init_with_ortho_contours(
        &mut self,
        subj: &[IntContour],
        clip: &[IntContour],
    ) -> Result<(), OrthoError> {
        let layout = if let Some(layout) =
            Layout::with_subj_and_clip(subj, clip, MIN_COUNT_PER_COLUMN_POWER)
        {
            layout
        } else {
            return Ok(());
        };

        self.layout = layout;
        let mut mapper = Mapper::new(self.layout.clone());

        mapper.add_ortho_contours(subj);
        mapper.add_ortho_contours(clip);

        self.columns.resize(self.layout.count(), Default::default());
        for (i, (column, counter)) in self.columns.iter_mut().zip(mapper.columns).enumerate() {
            let (min, max) = self.layout.borders(i);
            column.init_with_counter(min, max, counter);
        }

        self.add_ortho_contours(subj, ShapeType::Subject)?;
        self.add_ortho_contours(clip, ShapeType::Clip)?;

        Ok(())
    }

    fn add_ortho_contours(
        &mut self,
        contours: &[IntContour],
        shape_type: ShapeType,
    ) -> Result<(), OrthoError> {
        let (direct, invert) = ShapeCountBoolean::with_shape_type(shape_type);

        for contour in contours.iter() {
            self.add_ortho_contour(contour, direct, invert)?;
        }

        Ok(())
    }

    #[inline]
    fn add_ortho_contour(
        &mut self,
        contour: &[IntPoint],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> Result<(), OrthoError> {
        let iter = if let Some(result) = contour.segment_iter::<DropCollinear>() {
            result
        } else {
            return Ok(());
        };

        for s in iter {
            _ = self.add_segment(s, direct, invert);
        }

        Ok(())
    }

    #[inline]
    fn add_segment(
        &mut self,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> Result<(), OrthoError> {
        match Orientation::new(segment)? {
            Orientation::Vertical => {
                let index = self.layout.index(segment[0].x);
                let (min_y, max_y) = segment.y_range();
                unsafe {
                    self.columns.get_unchecked_mut(index).vr_segments.push(
                        OrthoSegment::new_boolean(min_y, max_y, segment[0].x, direct, invert),
                    );
                }
            }
            Orientation::Horizontal => {
                let (min_x, max_x) = segment.x_range();

                let i0 = self.layout.index(min_x);
                let (i1, inner_border) = self.layout.index_inner_border_check(max_x);

                let mut x0 = min_x;
                for index in i0..i1 {
                    let xi = self.layout.left_border(index + 1);
                    unsafe {
                        self.columns.get_unchecked_mut(index).hz_segments.push(
                            OrthoSegment::new_boolean(x0, xi, segment[0].y, direct, invert),
                        );
                    }
                    x0 = xi
                }

                // add last
                unsafe {
                    self.columns
                        .get_unchecked_mut(i1)
                        .hz_segments
                        .push(OrthoSegment::new_boolean(
                            x0,
                            max_x,
                            segment[0].y,
                            direct,
                            invert,
                        ));
                }

                if inner_border {
                    let i2 = i1 + 1;
                    unsafe {
                        self.columns
                            .get_unchecked_mut(i2)
                            .border_points
                            .push(segment[0].y);
                    }
                }
            }
        }
        Ok(())
    }
}

trait XYMinMaxRange {
    fn x_range(&self) -> (i32, i32);
    fn y_range(&self) -> (i32, i32);
}

impl XYMinMaxRange for [IntPoint; 2] {
    #[inline(always)]
    fn x_range(&self) -> (i32, i32) {
        if self[0].x < self[1].x {
            (self[0].x, self[1].x)
        } else {
            (self[1].x, self[0].x)
        }
    }

    #[inline(always)]
    fn y_range(&self) -> (i32, i32) {
        if self[0].y < self[1].y {
            (self[0].y, self[1].y)
        } else {
            (self[1].y, self[0].y)
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::core::shape_type::ShapeType::Subject;
    use crate::ortho::overlay::OrthoOverlay;
    use crate::ortho::segment::OrthoSegment;
    use i_float::int::point::IntPoint;
    use std::collections::HashSet;
    use crate::core::winding::WindingCount;
    use crate::graph::boolean::winding_count::ShapeCountBoolean;

    #[test]
    fn test_0() {
        let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();

        let subj = [[
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]
            .to_vec()];

        overlay.init_with_ortho_contours(&subj, &[]);
        let (direct, invert) = ShapeCountBoolean::with_shape_type(Subject);

        assert_eq!(overlay.columns.len(), 1);
        let column = &overlay.columns[0];

        let must_be_hz_set: HashSet<_> = [
            OrthoSegment::new_boolean(0, 10, 0, direct, invert),
            OrthoSegment::new_boolean(0, 10, 10, direct, invert),
        ]
            .iter()
            .copied()
            .collect();

        let must_be_vr_set: HashSet<_> = [
            OrthoSegment::new_boolean(0, 10, 0, direct, invert),
            OrthoSegment::new_boolean(0, 10, 10, direct, invert),
        ]
            .iter()
            .copied()
            .collect();

        let value_hz_set: HashSet<_> = column.hz_segments.iter().copied().collect();
        let value_vr_set: HashSet<_> = column.vr_segments.iter().copied().collect();

        assert_eq!(must_be_hz_set, value_hz_set);
        assert_eq!(must_be_vr_set, value_vr_set);
        assert_eq!(column.border_points.len(), 0);
    }

    #[test]
    fn test_1() {
        let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();

        let subj = [[
            IntPoint::new(0, 0),
            IntPoint::new(5, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 5),
            IntPoint::new(10, 10),
            IntPoint::new(5, 10),
            IntPoint::new(0, 10),
            IntPoint::new(0, 5),
        ]
            .to_vec()];

        overlay.init_with_ortho_contours(&subj, &[]);
        let (direct, invert) = ShapeCountBoolean::with_shape_type(Subject);

        assert_eq!(overlay.columns.len(), 1);
        let column = &overlay.columns[0];

        let must_be_hz_set: HashSet<_> = [
            OrthoSegment::new_boolean(0, 10, 0, direct, invert),
            OrthoSegment::new_boolean(0, 10, 10, direct, invert),
        ]
            .iter()
            .copied()
            .collect();

        let must_be_vr_set: HashSet<_> = [
            OrthoSegment::new_boolean(0, 10, 0, direct, invert),
            OrthoSegment::new_boolean(0, 10, 10, direct, invert),
        ]
            .iter()
            .copied()
            .collect();

        let value_hz_set: HashSet<_> = column.hz_segments.iter().copied().collect();
        let value_vr_set: HashSet<_> = column.vr_segments.iter().copied().collect();

        assert_eq!(must_be_hz_set, value_hz_set);
        assert_eq!(must_be_vr_set, value_vr_set);
        assert_eq!(column.border_points.len(), 0);
    }
}