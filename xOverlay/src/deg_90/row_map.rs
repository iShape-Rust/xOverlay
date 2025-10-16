use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;
use crate::deg_90::layout::RowLayout;
use crate::gear::segment::Segment;
use crate::gear::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;
use crate::util::y_range::YRangeAndCount;
use alloc::vec;
use alloc::vec::Vec;
use i_shape::int::shape::IntContour;
use crate::partition::row::Row;

struct Mapper {
    layout: RowLayout,
    parts: Vec<usize>,
}

pub(crate) struct RowMap {
    pub(super) rows: Vec<Row>,
}

impl RowMap {
    fn with_subj_and_clip(
        subj: &[IntContour],
        clip: &[IntContour],
        parallel: bool,
        row_max_height_count: usize,
    ) -> Self {
        let layout = Self::calculate_layout(subj, clip, parallel, row_max_height_count);

        let mut mapper = Mapper::new(layout);
        mapper.add_contours(subj);
        mapper.add_contours(clip);

        let mut row_map = Self {
            rows: Vec::with_capacity(mapper.layout.count),
        };

        row_map.pre_init_rows(&mapper);
        row_map.add_contours(subj, ShapeType::Subject, &mapper.layout);
        row_map.add_contours(clip, ShapeType::Clip, &mapper.layout);

        row_map
    }

    fn calculate_layout(
        subj: &[IntContour],
        clip: &[IntContour],
        parallel: bool,
        row_max_height_count: usize,
    ) -> RowLayout {
        debug_assert!(row_max_height_count.is_power_of_two());

        let (subj_range, subj_count) = subj.y_range_and_count(parallel);
        let (clip_range, clip_count) = clip.y_range_and_count(parallel);

        let range = LineRange::with_min_max(
            subj_range.min.min(clip_range.min),
            subj_range.max.max(clip_range.max),
        );
        let items_count = subj_count + clip_count;

        let avg_count_per_dimension = items_count.isqrt();
        let opt_chunks_count = avg_count_per_dimension >> row_max_height_count.ilog2();

        RowLayout::new(range, opt_chunks_count)
    }
}

impl Mapper {
    #[inline]
    pub(super) fn new(layout: RowLayout) -> Self {
        let n = layout.count;
        Self {
            layout,
            parts: vec![0; n],
        }
    }

    pub(super) fn add_contours(&mut self, contours: &[IntContour]) {
        for contour in contours {
            if contour.len() >= 4 {
                self.add_contour(contour);
            }
        }
    }

    #[inline]
    fn add_contour(&mut self, contour: &IntContour) {
        let mut p0 = contour[0];
        for &pi in contour.iter() {
            if pi.y == p0.y {
                // horizontal
                let index = self.layout.index(pi.y);
                unsafe {
                    *self.parts.get_unchecked_mut(index) += 1;
                }
            }
            p0 = pi;
        }
    }
}

impl RowMap {
    #[inline]
    pub(super) fn pre_init_rows(&mut self, mapper: &Mapper) {
        let mut y0 = mapper.layout.range.min;
        let s = mapper.layout.step() as i32;

        self.rows.reserve(mapper.parts.len());

        for &hz in mapper.parts.iter() {
            let y1 = y0 + s;
            let range = LineRange::with_min_max(y0, y1);
            y0 = y1;
            self.rows.push(Row {
                range,
                segments: Vec::with_capacity(hz),
            });
        }
    }

    pub(super) fn add_contours(
        &mut self,
        contours: &[IntContour],
        shape_type: ShapeType,
        layout: &RowLayout,
    ) {
        let (direct, invert) = ShapeCountBoolean::with_shape_type(shape_type);

        for contour in contours.iter() {
            if let Some(mut a) = contour.last() {
                for b in contour {
                    if a.y == b.y {
                        let (range, dir) = if a.x < b.x {
                            (LineRange::with_min_max(a.x, b.x), direct)
                        } else {
                            (LineRange::with_min_max(b.x, a.x), invert)
                        };

                        let row_index = layout.index(a.y);

                        let row = unsafe { self.rows.get_unchecked_mut(row_index) };

                        row.segments.push(Segment {
                            pos: a.y,
                            range,
                            count: dir,
                        });
                    }
                    a = b;
                }
            }
        }
    }
}
