use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;
use crate::deg_90::layout::ColumnLayout;
use crate::gear::segment::Segment;
use crate::fill::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;
use crate::util::x_range::XRangeAndCount;
use alloc::vec;
use alloc::vec::Vec;
use i_shape::int::shape::IntContour;
use crate::core::cpu_count::CPUCount;
use crate::deg_90::config::ColumnConfig90;

struct Mapper {
    layout: ColumnLayout,
    parts: Vec<usize>,
}

pub(super) struct Column {
    pub(super) range: LineRange,
    pub(super) segments: Vec<Segment>,
}

pub(crate) struct ColumnMap {
    pub(super) columns: Vec<Column>,
}

impl ColumnMap {
    pub(super) fn with_subj_and_clip(
        subj: &[IntContour],
        clip: &[IntContour],
        cpu_count: CPUCount,
        config: ColumnConfig90,
    ) -> Self {
        let layout = Self::calculate_layout(subj, clip, cpu_count, config);

        let mut mapper = Mapper::new(layout);
        mapper.add_contours(subj);
        mapper.add_contours(clip);

        let mut column_map = Self {
            columns: Vec::with_capacity(mapper.layout.count),
        };

        column_map.pre_init_columns(&mapper);
        column_map.add_contours(subj, ShapeType::Subject, &mapper.layout);
        column_map.add_contours(clip, ShapeType::Clip, &mapper.layout);

        column_map
    }

    fn calculate_layout(
        subj: &[IntContour],
        clip: &[IntContour],
        cpu_count: CPUCount,
        config: ColumnConfig90,
    ) -> ColumnLayout {
        let (subj_range, subj_count) = subj.x_range_and_count(cpu_count);
        let (clip_range, clip_count) = clip.x_range_and_count(cpu_count);

        let range = LineRange::with_min_max(
            subj_range.min.min(clip_range.min),
            subj_range.max.max(clip_range.max),
        );
        let segments_count = subj_count + clip_count;

        ColumnLayout::with_segments_count(segments_count, range, cpu_count, config)
    }

    pub(super) fn with_segments(
        segments: &[Segment],
        max_segments_in_line: usize,
        range: LineRange,
        config: ColumnConfig90,
    ) -> Option<Self> {
        let layout = ColumnLayout::with_max_segments_in_line(max_segments_in_line, range, config)?;

        let mut mapper = Mapper::new(layout);
        mapper.add_segments(segments);

        let mut column_map = Self {
            columns: Vec::with_capacity(mapper.layout.count),
        };

        column_map.pre_init_columns(&mapper);
        column_map.add_segments(segments, &mapper.layout);

        Some(column_map)
    }

    #[inline]
    fn pre_init_columns(&mut self, mapper: &Mapper) {
        let mut x0 = mapper.layout.range.min;
        let s = mapper.layout.step() as i32;

        self.columns.reserve(mapper.parts.len());

        for &hz in mapper.parts.iter() {
            let x1 = x0 + s;
            let range = LineRange::with_min_max(x0, x1);
            x0 = x1;
            self.columns.push(Column {
                range,
                segments: Vec::with_capacity(hz),
            });
        }
    }

    fn add_contours(
        &mut self,
        contours: &[IntContour],
        shape_type: ShapeType,
        layout: &ColumnLayout,
    ) {
        let (direct, invert) = ShapeCountBoolean::with_shape_type(shape_type);

        for contour in contours.iter() {
            let mut a = if let Some(last) = contour.last()
                && contour.len() >= 4
            {
                last
            } else {
                continue;
            };

            for b in contour {
                if a.x == b.x {
                    // skip vertical
                    a = b;
                    continue;
                }

                // horizontal

                let (range, dir) = if a.x < b.x {
                    (LineRange::with_min_max(a.x, b.x), direct)
                } else {
                    (LineRange::with_min_max(b.x, a.x), invert)
                };

                let i0 = layout.index(range.min);
                let i1 = layout.index_round_down(range.max);

                let y = a.y;
                let mut x0 = range.min;

                for i in i0..i1 {
                    let xi = layout.left_border(i + 1);
                    unsafe {
                        self.columns.get_unchecked_mut(i).segments.push(Segment {
                            pos: y,
                            range: LineRange::with_min_max(x0, xi),
                            count: dir,
                        });
                    }
                    x0 = xi
                }

                // add last
                unsafe {
                    self.columns.get_unchecked_mut(i1).segments.push(Segment {
                        pos: y,
                        range: LineRange::with_min_max(x0, range.max),
                        count: dir,
                    });
                }

                a = b;
            }
        }
    }

    fn add_segments(
        &mut self,
        segments: &[Segment],
        layout: &ColumnLayout,
    ) {
        for s in segments.iter() {
            let i0 = layout.index(s.range.min);
            let i1 = layout.index_round_down(s.range.max);

            let y = s.pos;
            let mut x0 = s.range.min;

            for i in i0..i1 {
                let xi = layout.left_border(i + 1);
                unsafe {
                    self.columns.get_unchecked_mut(i).segments.push(Segment {
                        pos: y,
                        range: LineRange::with_min_max(x0, xi),
                        count: s.count,
                    });
                }
                x0 = xi
            }

            // add last
            unsafe {
                self.columns.get_unchecked_mut(i1).segments.push(Segment {
                    pos: y,
                    range: LineRange::with_min_max(x0, s.range.max),
                    count: s.count,
                });
            }
        }
    }
}

impl Mapper {
    #[inline]
    pub(super) fn new(layout: ColumnLayout) -> Self {
        let n = layout.count;
        Self {
            layout,
            parts: vec![0; n],
        }
    }

    fn add_contours(&mut self, contours: &[IntContour]) {
        for contour in contours {
            if contour.len() >= 4 {
                self.add_contour(contour);
            }
        }
    }

    #[inline]
    fn add_contour(&mut self, contour: &IntContour) {
        let mut p0 = *contour.last().unwrap();
        for &pi in contour.iter() {
            if pi.x == p0.x {
                // skip vertical
                p0 = pi;
                continue;
            }

            // horizontal
            let (x0, x1) = if pi.x < p0.x {
                (pi.x, p0.x)
            } else {
                (p0.x, pi.x)
            };
            let i0 = self.layout.index(x0);
            let i1 = self.layout.index_round_down(x1);
            for index in i0..=i1 {
                unsafe {
                    *self.parts.get_unchecked_mut(index) += 1;
                }
            }
        }
    }

    pub(super) fn add_segments(&mut self, segments: &[Segment]) {
        for s in segments.iter() {
            let i0 = self.layout.index(s.range.min);
            let i1 = self.layout.index_round_down(s.range.max);
            for index in i0..=i1 {
                unsafe {
                    *self.parts.get_unchecked_mut(index) += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::deg_90::column_map::ColumnMap;
    use i_shape::int_shape;
    use crate::core::cpu_count::CPUCount;
    use crate::deg_90::config::ColumnConfig90;

    #[test]
    fn test_0() {
        let subj = int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[4, 0], [8, 0], [8, 4], [4, 4]],
        ];
        let config = ColumnConfig90::dev(2);
        let map = ColumnMap::with_subj_and_clip(&subj, &[], CPUCount::Single, config);
        assert_eq!(map.columns.len(), 2);
    }
}
