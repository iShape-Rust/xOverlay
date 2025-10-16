use crate::core::overlay::OverlayError;
use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;
use crate::gear::init::XYMinMaxRange;
use crate::gear::seg_iter::{DropCollinear, SegmentIterable};
use crate::gear::segment::Segment;
use crate::gear::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;
use crate::tile::column::TileColumn;
use crate::tile::layout::TileLayout;
use crate::tile::source::GeometrySource;
use crate::tile::tilemap::TileMap;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use i_float::int::point::IntPoint;
use i_shape::int::shape::IntContour;

pub(super) struct TileMapper {
    pub(super) layout: TileLayout,
    pub(super) hz_parts: Vec<usize>,
    pub(super) vr_parts: Vec<usize>,
    pub(super) dp_parts: Vec<usize>,
    pub(super) dn_parts: Vec<usize>,
}

impl TileMapper {
    #[inline]
    pub(super) fn new(layout: TileLayout) -> Self {
        let n = layout.count;
        Self {
            layout,
            hz_parts: vec![0; n],
            vr_parts: vec![0; n],
            dp_parts: vec![0; n],
            dn_parts: vec![0; n],
        }
    }

    pub(super) fn add_contours(&mut self, contours: &[IntContour]) {
        for contour in contours {
            if contour.len() >= 4 {
                self.add_contour(contour);
            }
        }
    }

    #[inline(always)]
    fn add_contour(&mut self, contour: &IntContour) {
        let mut p0 = contour[0];
        for &pi in contour.iter() {
            if pi.x == p0.x {
                // vertical
                if let Some(index) = self.layout.index_exclude_border(pi.x) {
                    unsafe {
                        *self.vr_parts.get_unchecked_mut(index) += 1;
                    }
                }
            } else {
                if let Some((i0, i1)) = self.layout.indices_by_xx(p0.x, pi.x) {
                    match pi.y.cmp(&p0.y) {
                        Ordering::Equal => {
                            // horizontal
                            for index in i0..=i1 {
                                unsafe {
                                    *self.hz_parts.get_unchecked_mut(index) += 1;
                                }
                            }
                        }
                        Ordering::Less => {
                            // positive diagonal
                            for index in i0..=i1 {
                                unsafe {
                                    *self.dp_parts.get_unchecked_mut(index) += 1;
                                }
                            }
                        }
                        Ordering::Greater => {
                            // negative diagonal
                            for index in i0..=i1 {
                                unsafe {
                                    *self.dn_parts.get_unchecked_mut(index) += 1;
                                }
                            }
                        }
                    }
                }
            }
            p0 = pi;
        }
    }
}

impl TileMap {

    #[inline]
    pub(super) fn pre_init_columns(&mut self, mapper: &TileMapper) {
        let mut x0 = mapper.layout.range.min;
        let s = mapper.layout.step() as i32;

        self.columns.reserve(mapper.vr_parts.len());

        for (((&vr, &hz), &dp), &dn) in mapper
            .vr_parts
            .iter()
            .zip(mapper.hz_parts.iter())
            .zip(mapper.dp_parts.iter())
            .zip(mapper.dn_parts.iter())
        {
            let x1 = x0 + s;
            let range = LineRange::with_min_max(x0, x1);
            x0 = x1;

            let source = GeometrySource {
                vr_list: Vec::with_capacity(vr),
                hz_list: Vec::with_capacity(hz),
                dp_list: Vec::with_capacity(dp),
                dn_list: Vec::with_capacity(dn),
            };

            self.columns.push(TileColumn { range, source });
        }
    }

    pub(super) fn add_contours(
        &mut self,
        contours: &[IntContour],
        shape_type: ShapeType,
        layout: &TileLayout,
    ) -> Result<(), OverlayError> {
        let (direct, invert) = ShapeCountBoolean::with_shape_type(shape_type);

        for contour in contours.iter() {
            self.add_contour(layout, contour, direct, invert)?;
        }

        Ok(())
    }

    fn add_contour(
        &mut self,
        layout: &TileLayout,
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
        layout: &TileLayout,
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
        layout: &TileLayout,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) {
        let x0 = segment[0].x;

        if layout.is_border(x0) {
            return;
        }

        // vertical
        let (range, dir) = segment.y_range(direct, invert);

        let index = layout.index(x0);
        unsafe {
            self.columns
                .get_unchecked_mut(index)
                .source
                .vr_list
                .push(Segment {
                    pos: x0,
                    range,
                    count: dir,
                });
        }
    }

    #[inline]
    fn add_horizontal(
        &mut self,
        layout: &TileLayout,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) {
        let y0 = segment[0].y;

        let (range, dir) = segment.x_range(direct, invert);
        let (i0, i1) = layout.indices_by_range(range);

        let mut x0 = range.min;

        for index in i0..=i1 {
            let xi = layout.left_border(index + 1);
            unsafe {
                self.columns
                    .get_unchecked_mut(index)
                    .source
                    .hz_list
                    .push(Segment {
                        pos: y0,
                        range: LineRange::with_min_max(x0, xi),
                        count: dir,
                    });
            }
            x0 = xi
        }

        // add last
        unsafe {
            self.columns
                .get_unchecked_mut(i1)
                .source
                .hz_list
                .push(Segment {
                    pos: y0,
                    range: LineRange::with_min_max(x0, range.max),
                    count: dir,
                });
        }
    }

    #[inline]
    fn add_diagonal(
        &mut self,
        layout: &TileLayout,
        segment: [IntPoint; 2],
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) {
        let (a, b, dir) = segment.xy_range(direct, invert);
        let (i0, i1) = layout.indices_by_range(LineRange::with_min_max(a.x, b.x));

        let mut x0 = a.x;

        if a.y < b.y {
            // positive diagonal

            let y0 = a.y;
            let mut yi = y0;

            for index in i0..i1 {
                let xi = layout.left_border(index + 1);
                let dx = xi.wrapping_sub(a.x);
                unsafe {
                    self.columns
                        .get_unchecked_mut(index)
                        .source
                        .dp_list
                        .push(Segment {
                            pos: yi,
                            range: LineRange::with_min_max(x0, xi),
                            count: dir,
                        });
                }
                yi = y0.wrapping_add(dx);
                x0 = xi
            }

            // add last
            unsafe {
                self.columns
                    .get_unchecked_mut(i1)
                    .source
                    .dp_list
                    .push(Segment {
                        pos: yi,
                        range: LineRange::with_min_max(x0, b.x),
                        count: dir,
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
                    self.columns
                        .get_unchecked_mut(index)
                        .source
                        .dn_list
                        .push(Segment {
                            pos: yi,
                            range: LineRange::with_min_max(x0, xi),
                            count: dir,
                        });
                }
                yi = y0.wrapping_sub(dx);
                x0 = xi
            }

            // add last
            unsafe {
                self.columns
                    .get_unchecked_mut(i1)
                    .source
                    .dn_list
                    .push(Segment {
                        pos: yi,
                        range: LineRange::with_min_max(x0, b.x),
                        count: dir,
                    });
            }
        }
    }
}
