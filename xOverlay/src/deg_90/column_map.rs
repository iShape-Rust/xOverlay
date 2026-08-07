use crate::core::cpu_count::CPUCount;
use crate::core::integer::OverlayInt;
use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::config::ColumnConfig90;
use crate::deg_90::layout::ColumnLayout;
use crate::geom::range::LineRange;
use crate::geom::segment::Segment;
use crate::util::x_range::XRangeAndCount;
use alloc::vec;
use alloc::vec::Vec;
use i_shape::int::shape::IntContour;

struct Mapper<I: OverlayInt> {
    layout: ColumnLayout<I>,
    parts: Vec<usize>,
}

pub(crate) struct Column<I: OverlayInt, W: WindingCount = i16> {
    pub(super) range: LineRange<I>,
    pub(super) segments: Vec<Segment<I, W>>,
}

pub(crate) struct ColumnMap<I: OverlayInt, W: WindingCount = i16> {
    pub(crate) columns: Vec<Column<I, W>>,
}

impl<I: OverlayInt, W: WindingCount> ColumnMap<I, W> {
    pub(crate) fn with_subj_and_clip(
        subj: &[IntContour<I>],
        clip: &[IntContour<I>],
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
        subj: &[IntContour<I>],
        clip: &[IntContour<I>],
        cpu_count: CPUCount,
        config: ColumnConfig90,
    ) -> ColumnLayout<I> {
        let (subj_range, subj_count) = subj.x_range_and_count(cpu_count);
        let (clip_range, clip_count) = clip.x_range_and_count(cpu_count);

        let segments_count = subj_count + clip_count;
        if segments_count == 0 {
            return ColumnLayout::with_segments_count(
                0,
                LineRange::with_min_max(I::ZERO, I::ZERO),
                cpu_count,
                config,
            );
        }

        let range = LineRange::with_min_max(
            subj_range.min.min(clip_range.min),
            subj_range.max.max(clip_range.max),
        );
        ColumnLayout::with_segments_count(segments_count, range, cpu_count, config)
    }

    pub(super) fn with_segments(
        segments: &[Segment<I, W>],
        max_segments_in_line: usize,
        range: LineRange<I>,
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
    fn pre_init_columns(&mut self, mapper: &Mapper<I>) {
        let mut x0 = mapper.layout.range.min;

        self.columns.reserve(mapper.parts.len());

        for (index, &hz) in mapper.parts.iter().enumerate() {
            let x1 = if index + 1 == mapper.parts.len() {
                mapper.layout.capped_left_border(index + 1)
            } else {
                mapper.layout.left_border(index + 1)
            };
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
        contours: &[IntContour<I>],
        shape_type: ShapeType,
        layout: &ColumnLayout<I>,
    ) {
        let (direct, invert) = ShapeCountBoolean::<W>::with_shape_type(shape_type);

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

    fn add_segments(&mut self, segments: &[Segment<I, W>], layout: &ColumnLayout<I>) {
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

    #[cfg(test)]
    pub(crate) fn with_columns_count(
        subj: &[IntContour<I>],
        clip: &[IntContour<I>],
        count: usize,
    ) -> Self {
        assert!(count > 0, "column count must be greater than zero");

        let (subj_range, _) = subj.x_range_and_count(CPUCount::Single);
        let (clip_range, _) = clip.x_range_and_count(CPUCount::Single);

        let range = LineRange::with_min_max(
            subj_range.min.min(clip_range.min),
            subj_range.max.max(clip_range.max),
        );

        let layout = ColumnLayout::with_range_and_count(range, count);

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
}

impl<I: OverlayInt> Mapper<I> {
    #[inline]
    pub(super) fn new(layout: ColumnLayout<I>) -> Self {
        let n = layout.count;
        Self {
            layout,
            parts: vec![0; n],
        }
    }

    fn add_contours(&mut self, contours: &[IntContour<I>]) {
        for contour in contours {
            if contour.len() >= 4 {
                self.add_contour(contour);
            }
        }
    }

    #[inline]
    fn add_contour(&mut self, contour: &IntContour<I>) {
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

    pub(super) fn add_segments<W: WindingCount>(&mut self, segments: &[Segment<I, W>]) {
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
    use crate::core::cpu_count::CPUCount;
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;
    use i_shape::int_shape;

    #[test]
    fn test_0() {
        let subj = int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[4, 0], [8, 0], [8, 4], [4, 4]],
        ];
        let config = ColumnConfig90::dev(2);
        let map = ColumnMap::<i32>::with_subj_and_clip(&subj, &[], CPUCount::Single, config);
        assert_eq!(map.columns.len(), 2);
    }
}
