use crate::core::fill_rule::FillRule;
use crate::core::winding::WindingCount;
use crate::deg_90::column_map::Column;
use crate::definition::segment::SegmentFill;
use crate::definition::fill::{
    EvenOddStrategy, FillStrategy, NegativeStrategy, NonZeroStrategy, PositiveStrategy,
};
use crate::definition::winding_count::ShapeCountBoolean;
use crate::gear::segment::Segment;
use alloc::vec::Vec;
use core::mem::swap;

impl Column {
    pub(super) fn fill(
        &self,
        fill_rule: FillRule,
        fill_buffer: &mut FillBuffer,
    ) -> Vec<SegmentFill> {
        match fill_rule {
            FillRule::EvenOdd => self
                .segments
                .fill_with_strategy::<EvenOddStrategy>(fill_buffer),
            FillRule::NonZero => self
                .segments
                .fill_with_strategy::<NonZeroStrategy>(fill_buffer),
            FillRule::Positive => self
                .segments
                .fill_with_strategy::<PositiveStrategy>(fill_buffer),
            FillRule::Negative => self
                .segments
                .fill_with_strategy::<NegativeStrategy>(fill_buffer),
        }
    }
}
trait Fill {
    fn fill_with_strategy<F: FillStrategy<ShapeCountBoolean>>(
        &self,
        buffer: &mut FillBuffer,
    ) -> Vec<SegmentFill>;
}

impl Fill for [Segment] {
    fn fill_with_strategy<F: FillStrategy<ShapeCountBoolean>>(
        &self,
        buffer: &mut FillBuffer,
    ) -> Vec<SegmentFill> {
        let n = self.len();
        buffer.clear();

        let mut result: Vec<SegmentFill> = Vec::with_capacity(n);

        let mut i = 0;
        while i < n {
            let start = i;
            let pos = self[i].pos;
            i += 1;

            while i < n && self[i].pos == pos {
                i += 1;
            }
            buffer.add_segments::<F>(&self[start..i], &mut result);
        }

        debug_assert!(result.len() == n);

        result
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Anchor {
    x: i32,
    count: ShapeCountBoolean,
}

impl Anchor {
    #[inline(always)]
    fn add(&self, count: ShapeCountBoolean) -> Self {
        Self {
            x: self.x,
            count: self.count + count,
        }
    }
}

struct FillBuffer {
    active: Vec<Anchor>,
    buffer: Vec<Anchor>,
}

impl FillBuffer {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            active: Vec::with_capacity(capacity),
            buffer: Vec::with_capacity(capacity),
        }
    }

    fn add_segments<F: FillStrategy<ShapeCountBoolean>>(
        &mut self,
        segments: &[Segment],
        output: &mut Vec<SegmentFill>,
    ) {
        // segments are always sorted by range.min and not overlap each other
        // s(i).range.max <= s(i+1).range.min
        // segment.count.is_not_empty() === true
        // the segments not more < 100..200 elements
        // the buffers in average much less than 1000 and close to segments.len

        let mut anchor_iter = self.active.iter();
        let mut anchor = if let Some(first) = anchor_iter.next() {
            *first
        } else {
            self.active.init::<F>(segments, output);
            return;
        };

        self.buffer.clear();


        for (i, s) in segments.iter().enumerate() {
            while anchor.x <= s.range.min {
                self.buffer.add_or_merge(anchor);
                if let Some(next) = anchor_iter.next() {
                    anchor = *next;
                } else {
                    self.buffer.join::<F>(&segments[i..], output);
                    self.buffer.remove_last_if_empty();
                    swap(&mut self.active, &mut self.buffer);
                    return;
                }
            }

            // s.range.min < anchor.x

            if !self.buffer.last().is_some_and(|last| last.x == s.range.min) {
                self.buffer.add_or_merge(Anchor { x: s.range.min, count: anchor.count });
            }

            let (top, fill) = F::add_and_fill(s.count, anchor.count);
            output.push(fill);

            if anchor.x >= s.range.max {
                self.buffer.add_or_merge(Anchor {
                    x: s.range.max,
                    count: top,
                });

                if anchor.x == s.range.max {
                    if let Some(next) = anchor_iter.next() {
                        anchor = *next;
                    } else {
                        self.buffer.remove_last_if_empty();
                        swap(&mut self.active, &mut self.buffer);
                        return;
                    }
                }

                continue;
            }

            while anchor.x < s.range.max {
                self.buffer.add_or_merge(anchor.add(s.count));
                if let Some(next) = anchor_iter.next() {
                    anchor = *next;
                } else {
                    let (top, fill) = F::add_and_fill(s.count, ShapeCountBoolean::empty());
                    self.buffer.add_or_merge(Anchor {
                        x: s.range.max,
                        count: top,
                    });

                    output.push(fill);
                    let i1 = i + 1;
                    if i1 < segments.len() {
                        self.buffer.join::<F>(&segments[i1..], output);
                    }

                    self.buffer.remove_last_if_empty();
                    swap(&mut self.active, &mut self.buffer);
                    return;
                }
            }

            // s.range.max <= anchor.x

            // s.range.max < anchor.x
            let (top, fill) = F::add_and_fill(s.count, anchor.count);
            self.buffer.add_or_merge(Anchor {
                x: s.range.max,
                count: top,
            });
            // output.push(definition);
        }

        self.buffer.add_or_merge(anchor);
        self.buffer.extend(anchor_iter);
        self.buffer.remove_last_if_empty();
        swap(&mut self.active, &mut self.buffer);
    }

    #[inline]
    fn clear(&mut self) {
        self.active.clear();
        self.buffer.clear();
    }
}

trait AnchorBuffer {
    fn add_or_merge(&mut self, anchor: Anchor);
    fn remove_last_if_empty(&mut self);
    fn join<F: FillStrategy<ShapeCountBoolean>>(
        &mut self,
        segments: &[Segment],
        output: &mut Vec<SegmentFill>,
    );
    fn init<F: FillStrategy<ShapeCountBoolean>>(
        &mut self,
        segments: &[Segment],
        output: &mut Vec<SegmentFill>,
    );
}

impl AnchorBuffer for Vec<Anchor> {
    #[inline(always)]
    fn add_or_merge(&mut self, anchor: Anchor) {
        if let Some(last) = self.last_mut() {
            if last.count == anchor.count {
                last.x = anchor.x;
                return;
            }
        }
        self.push(anchor);
    }

    #[inline(always)]
    fn remove_last_if_empty(&mut self) {
        if let Some(last) = self.last() {
            if last.count.is_empty() {
                self.pop();
            }
        }
    }

    #[inline]
    fn join<F: FillStrategy<ShapeCountBoolean>>(
        &mut self,
        segments: &[Segment],
        output: &mut Vec<SegmentFill>,
    ) {
        for s in segments {
            self.push(Anchor {
                x: s.range.min,
                count: ShapeCountBoolean::empty(),
            });
            self.push(Anchor {
                x: s.range.max,
                count: s.count,
            });
            output.push(F::fill(s.count, ShapeCountBoolean::empty()));
        }
    }
    #[inline]
    fn init<F: FillStrategy<ShapeCountBoolean>>(
        &mut self,
        segments: &[Segment],
        output: &mut Vec<SegmentFill>,
    ) {
        self.clear();
        let capacity = 2 * segments.len() + 1;
        self.reserve(capacity);
        let mut x0 = i32::MAX;
        for s in segments {
            debug_assert!(s.count.is_not_empty());
            let a1 = Anchor {
                x: s.range.max,
                count: s.count,
            };

            if s.range.min != x0 {
                let a0 = Anchor {
                    x: s.range.min,
                    count: ShapeCountBoolean::empty(),
                };
                self.push(a0);
                self.push(a1);
            } else {
                self.add_or_merge(a1);
            }

            output.push(F::fill(s.count, ShapeCountBoolean::empty()));

            x0 = s.range.max;
        }

        debug_assert!(self.len() <= capacity);
        self.remove_last_if_empty();
    }
}

#[cfg(test)]
mod tests {
    use crate::core::cpu_count::CPUCount;
    use crate::core::winding::WindingCount;
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;
    use crate::deg_90::fill::{Anchor, Fill, FillBuffer};
    use crate::definition::segment::{SUBJ_BOTH, SUBJ_BOTTOM, SUBJ_TOP};
    use crate::definition::fill::NonZeroStrategy;
    use crate::definition::winding_count::ShapeCountBoolean;
    use crate::gear::segment::Segment;
    use crate::geom::range::LineRange;
    use crate::partition::solver::Partition;
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_overlay::vector::edge::Reverse;
    use i_shape::int::path::IntPath;
    use i_shape::int::shape::{IntContour, IntShape};
    use i_shape::{int_path, int_shape};
    use rand::Rng;

    #[test]
    fn test_buffer_0() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 4),
            count: ShapeCountBoolean::new(1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line, &mut output);

        assert_eq!(line.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
    }

    #[test]
    fn test_buffer_1() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line_0 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 4),
            count: ShapeCountBoolean::new(1, 0),
        }];
        let line_1 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 4),
            count: ShapeCountBoolean::new(-1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line_0, &mut output);
        buffer.add_segments::<NonZeroStrategy>(&line_1, &mut output);

        assert!(buffer.active.is_empty());
        assert_eq!(line_0.len() + line_1.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_BOTTOM);
    }

    #[test]
    fn test_buffer_2() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line_0 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(1, 7),
            count: ShapeCountBoolean::new(1, 0),
        }];
        let line_1 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 6),
            count: ShapeCountBoolean::new(1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line_0, &mut output);
        buffer.add_segments::<NonZeroStrategy>(&line_1, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 1,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 2,
                    count: ShapeCountBoolean::new(1, 0)
                },
                Anchor {
                    x: 6,
                    count: ShapeCountBoolean::new(2, 0)
                },
                Anchor {
                    x: 7,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line_0.len() + line_1.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_BOTH);
    }

    #[test]
    fn test_buffer_3() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line_0 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(1, 7),
            count: ShapeCountBoolean::new(1, 0),
        }];
        let line_1 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(2, 6),
            count: ShapeCountBoolean::new(-1, 0),
        }];
        buffer.add_segments::<NonZeroStrategy>(&line_0, &mut output);
        buffer.add_segments::<NonZeroStrategy>(&line_1, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 1,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 2,
                    count: ShapeCountBoolean::new(1, 0)
                },
                Anchor {
                    x: 6,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 7,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line_0.len() + line_1.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_BOTTOM);
    }

    #[test]
    fn test_buffer_4() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line = [
            Segment {
                pos: 0,
                range: LineRange::with_min_max(1, 2),
                count: ShapeCountBoolean::new(1, 0),
            },
            Segment {
                pos: 0,
                range: LineRange::with_min_max(3, 4),
                count: ShapeCountBoolean::new(1, 0),
            },
        ];

        buffer.add_segments::<NonZeroStrategy>(&line, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 1,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 2,
                    count: ShapeCountBoolean::new(1, 0)
                },
                Anchor {
                    x: 3,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 4,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_TOP);
    }

    #[test]
    fn test_buffer_5() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();
        let line = [
            Segment {
                pos: 0,
                range: LineRange::with_min_max(0, 4),
                count: ShapeCountBoolean::new(1, 0),
            },
            Segment {
                pos: 0,
                range: LineRange::with_min_max(4, 8),
                count: ShapeCountBoolean::new(1, 0),
            },
        ];

        buffer.add_segments::<NonZeroStrategy>(&line, &mut output);

        assert_eq!(
            &buffer.active,
            &[
                Anchor {
                    x: 0,
                    count: ShapeCountBoolean::new(0, 0)
                },
                Anchor {
                    x: 8,
                    count: ShapeCountBoolean::new(1, 0)
                },
            ]
        );
        assert_eq!(line.len(), output.len());
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_TOP);
    }

    #[test]
    fn test_buffer_6() {
        let mut buffer = FillBuffer::with_capacity(0);
        let mut output = Vec::new();

        let line_0 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(1, 4),
            count: ShapeCountBoolean::new(1, 0),
        }];

        let line_1 = [Segment {
            pos: 0,
            range: LineRange::with_min_max(3, 7),
            count: ShapeCountBoolean::new(1, 0),
        }];

        buffer.add_segments::<NonZeroStrategy>(&line_0, &mut output);
        assert_eq!(line_0.len(), output.len(), "sanity check");

        buffer.add_segments::<NonZeroStrategy>(&line_1, &mut output);

        assert_eq!(output.len(), 3);
        assert_eq!(output[0], SUBJ_TOP);
        assert_eq!(output[1], SUBJ_BOTH);
        assert_eq!(output[2], SUBJ_TOP);
    }

    #[test]
    fn test_composite_0() {
        test_contours(&int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[0, 5], [4, 5], [4, 9], [0, 9]],
        ]);
    }

    #[test]
    fn test_composite_1() {
        test_contours(&int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[4, 0], [8, 0], [8, 4], [4, 4]],
        ]);
    }

    #[test]
    fn test_composite_2() {
        test_contours(&int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[5, 0], [9, 0], [9, 4], [5, 4]],
        ]);
    }

    #[test]
    fn test_composite_3() {
        test_contours(&int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[5, 0], [9, 0], [9, 4], [5, 4]],
            [[10, 0], [14, 0], [14, 4], [10, 4]],
        ]);
    }

    #[test]
    fn test_composite_4() {
        test_contours(&int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[4, 0], [4, 4], [8, 4], [8, 0]],
        ]);
    }

    #[test]
    fn test_composite_5() {
        test_contours(&int_shape![
            [[0, 0], [4, 0], [4, 4], [0, 4]],
            [[0, 4], [0, 8], [4, 8], [4, 4]],
        ]);
    }

    #[test]
    fn test_composite_6() {
        test_contours(&int_shape![
            [[0, 0], [-3, 0], [-3, 2], [-6, 2], [-6, 1], [0, 1]],
        ]);
    }

    #[test]
    fn test_random_0() {
        for _ in 0..1000 {
            let contour = random_90_deg_contour(4, 4);
            test_contours(&vec![contour]);
        }
    }

    fn random_90_deg_contour(n: usize, radius: usize) -> Vec<IntPoint> {
        let mut x = 0;
        let mut y = 0;

        let mut contour = IntPath::new();
        let mut rng = rand::rng();

        contour.push(IntPoint::new(x, y));
        for i in 0..n {
            let rnd = rng.random_range(1..2 * radius) as i32;
            let ds = rnd - radius as i32;
            if i % 2 == 0 {
                x += ds;
            } else {
                y += ds;
            }
            contour.push(IntPoint::new(x, y));
        }

        if x != 0 {
            contour.push(IntPoint::new(0, y));
        }
        contour
    }

    #[derive(Debug, PartialEq, Eq)]
    struct SegFill {
        a: IntPoint,
        b: IntPoint,
        f: u8,
    }

    fn contour_to_template_s_fills(contours: &[IntContour]) -> Option<Vec<SegFill>> {
        let mut overlay = i_overlay::core::overlay::Overlay::with_contours(&contours, &[]);
        let graph = overlay
            .build_graph_view(i_overlay::core::fill_rule::FillRule::NonZero)?;

        let edges = graph.extract_separate_vectors();

        let mut s_fills: Vec<_> = edges
            .iter()
            .filter(|e| e.a.y == e.b.y)
            .map(|e| {
                let (a, b, f) = if e.a < e.b {
                    (e.a, e.b, e.fill)
                } else {
                    (e.b, e.a, e.fill.reverse())
                };

                SegFill { a, b, f }
            })
            .collect();

        s_fills.sort_by_key(|s| s.a);

        Some(s_fills)
    }

    fn contour_to_subject_s_fills(
        contours: &IntShape,
        config: ColumnConfig90,
        buffer: &mut FillBuffer,
    ) -> Vec<SegFill> {
        let mut column = ColumnMap::with_subj_and_clip(contours, &[], CPUCount::Single, config);
        buffer.clear();
        debug_assert!(column.columns.len() == 1);
        let segments = &mut column.columns[0].segments;
        _ = segments.partition();

        let fills = segments.fill_with_strategy::<NonZeroStrategy>(buffer);
        let mut s_fills: Vec<_> = segments
            .iter()
            .zip(fills.iter())
            .map(|(s, &f)| SegFill {
                a: IntPoint::new(s.range.min, s.pos),
                b: IntPoint::new(s.range.max, s.pos),
                f,
            })
            .collect();
        s_fills.sort_by_key(|s| s.a);
        s_fills
    }

    fn test_contours(contours: &IntShape) {
        let config = ColumnConfig90 {
            min_columns_count: 1,
            min_column_width_power: 20,
            max_allow_segments_per_column: 1000_000_000,
            min_allowed_segments_per_column: 1_000_000,
            max_allowed_segments_per_line: 1000_000,
        };

        let mut buffer = FillBuffer::with_capacity(0);

        let template = if let Some(segments) = contour_to_template_s_fills(contours) {
            segments
        } else {
            return;
        };
        let subject = contour_to_subject_s_fills(contours, config, &mut buffer);

        if subject != template {
            assert_eq!(subject, template);
        }
    }
}
