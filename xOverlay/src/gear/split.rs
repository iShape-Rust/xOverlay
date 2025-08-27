use crate::gear::merge::Merge;
use crate::gear::section::Section;
use crate::gear::segment::Segment;
use crate::gear::source::GeometrySource;
use crate::gear::split_buffer::{
    Intersection, SplitBuffer, SplitDn, SplitDp, SplitHz, XMark, YMark,
};
use crate::gear::x_mapper::XMapper;
use crate::geom::diagonal::{Diagonal, NegativeDiagonal, PositiveDiagonal};
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use core::mem::swap;

impl Section {
    pub(super) fn split(&mut self) {
        let mut source_by_columns = self.source.new_same_size();
        let mut map_by_columns = self
            .source
            .map_by_columns(&self.layout, &mut source_by_columns);

        let intersection = self.intersect(&mut source_by_columns, &map_by_columns);

        if intersection.is_empty() {
            swap(&mut self.source, &mut source_by_columns);
        } else {
            self.split_by_marks(&mut source_by_columns, intersection);
            map_by_columns = source_by_columns.map_by_columns(&self.layout, &mut self.source);
        }

        self.sort_and_merge(&map_by_columns);
    }

    fn intersect(
        &mut self,
        source_by_columns: &mut GeometrySource,
        map_by_columns: &XMapper,
    ) -> Intersection {
        let mut start_vr = 0;
        let mut start_hz = 0;
        let mut start_dp = 0;
        let mut start_dn = 0;

        let scale = self.layout.count().ilog2();

        let mut hz_buffer = Vec::with_capacity(source_by_columns.hz_list.len() >> scale);
        let mut dp_buffer = Vec::with_capacity(source_by_columns.dp_list.len() >> scale);
        let mut dn_buffer = Vec::with_capacity(source_by_columns.dn_list.len() >> scale);

        let mut split_buffer = SplitBuffer::new(self.layout.y_range(), self.layout.log_width());

        for (column_index, part) in map_by_columns.iter_by_parts().enumerate() {
            let (min_x, max_x) = self.layout.borders(column_index);

            // get slices to new column data

            let vr_slice = &source_by_columns.vr_list[start_vr..start_vr + part.count_vr];
            let hz_slice = &source_by_columns.hz_list[start_hz..start_hz + part.count_hz];
            let dp_slice = &source_by_columns.dp_list[start_dp..start_dp + part.count_dp];
            let dn_slice = &source_by_columns.dn_list[start_dn..start_dn + part.count_dn];

            // prepare column data

            // hz
            self.clean_by_min_x_hz(min_x, &mut hz_buffer);
            self.add_hz(start_hz, hz_slice, &mut hz_buffer);

            // dn
            self.clean_by_min_x_dn(min_x, &mut dn_buffer);
            self.add_dn(start_dn, dn_slice, &mut dn_buffer);

            // dp
            self.clean_by_min_x_dp(min_x, &mut dp_buffer);
            self.add_dp(start_dp, dp_slice, &mut dp_buffer);

            // fill buffer
            split_buffer.add_hz_edges(max_x, &hz_buffer);
            split_buffer.add_dp_edges(max_x, &dp_buffer);
            split_buffer.add_dn_edges(max_x, &dn_buffer);

            // split

            if !vr_slice.is_empty() {
                // vr x hz
                if split_buffer.is_not_empty_hz() {
                    for (offset, vr) in vr_slice.iter().enumerate() {
                        let index = start_vr + offset;
                        split_buffer.intersect_vr_and_hz(IndexEdge::new_vr(index, vr));
                    }
                }

                // vr x dp
                if split_buffer.is_not_empty_dp() {
                    for (offset, vr) in vr_slice.iter().enumerate() {
                        let index = start_vr + offset;
                        split_buffer.intersect_vr_and_dp(IndexEdge::new_vr(index, vr));
                    }
                }

                // vr x dn
                if split_buffer.is_not_empty_dn() {
                    for (offset, vr) in vr_slice.iter().enumerate() {
                        let index = start_vr + offset;
                        split_buffer.intersect_vr_and_dn(IndexEdge::new_vr(index, vr));
                    }
                }
            }

            // all rest in the buffer
            split_buffer.intersect();

            start_vr += part.count_vr;
            start_hz += part.count_hz;
            start_dp += part.count_dp;
            start_dn += part.count_dn;
        }

        split_buffer.into_marks()
    }

    fn split_by_marks(&mut self, source_by_columns: &mut GeometrySource, result: Intersection) {
        source_by_columns.vr_list.split_as_vr(&result.vr_marks);
        source_by_columns.hz_list.split_as_hz(&result.hz_marks);
        source_by_columns.dp_list.split_as_dp(&result.dp_marks);
        source_by_columns.dn_list.split_as_dn(&result.dn_marks);

        self.source
            .vr_list
            .resize(source_by_columns.vr_list.len(), Default::default());
        self.source
            .hz_list
            .resize(source_by_columns.hz_list.len(), Default::default());
        self.source
            .dp_list
            .resize(source_by_columns.dp_list.len(), Default::default());
        self.source
            .dn_list
            .resize(source_by_columns.dn_list.len(), Default::default());
    }

    fn sort_and_merge(&mut self, map_by_columns: &XMapper) {
        Self::sort_vertically_by_min(&mut self.source.vr_list, &map_by_columns.vr_parts);
        self.source.vr_list.merge_if_needed();

        Self::sort_vertically_by_pos(&mut self.source.hz_list, &map_by_columns.hz_parts);
        self.source.hz_list.merge_if_needed();

        Self::sort_vertically_by_pos(&mut self.source.dp_list, &map_by_columns.dp_parts);
        self.source.dp_list.merge_if_needed();

        Self::sort_vertically_by_pos(&mut self.source.dn_list, &map_by_columns.dn_parts);
        self.source.dn_list.merge_if_needed();
    }

    #[inline]
    fn clean_by_min_x_dp(&mut self, min_x: i32, buffer: &mut Vec<SplitDp>) {
        buffer.retain_mut(|dp| {
            if dp.x_range.max < min_x {
                false
            } else {
                let new_min_y = dp.find_y(min_x);
                dp.x_range.min = min_x;
                dp.y_range.min = new_min_y;
                true
            }
        });
    }

    #[inline]
    fn clean_by_min_x_dn(&mut self, min_x: i32, buffer: &mut Vec<SplitDn>) {
        buffer.retain_mut(|dn| {
            if dn.x_range.max < min_x {
                false
            } else {
                let new_max_y = dn.find_y(min_x);
                dn.x_range.min = min_x;
                dn.y_range.max = new_max_y;

                true
            }
        });
    }

    #[inline]
    fn clean_by_min_x_hz(&mut self, min_x: i32, buffer: &mut Vec<SplitHz>) {
        buffer.retain_mut(|e| {
            if e.x_range.max < min_x {
                false
            } else {
                e.x_range.min = min_x;
                true
            }
        });
    }

    fn add_hz(&mut self, offset: usize, new_segments: &[Segment], buffer: &mut Vec<SplitHz>) {
        for (i, s) in new_segments.iter().enumerate() {
            let index = offset + i;
            buffer.push(SplitHz::with_segment(index, s));
        }
    }

    fn add_dp(&mut self, offset: usize, new_segments: &[Segment], buffer: &mut Vec<SplitDp>) {
        for (i, s) in new_segments.iter().enumerate() {
            let index = offset + i;
            buffer.push(SplitDp::with_segment(index, s));
        }
    }

    fn add_dn(&mut self, offset: usize, new_segments: &[Segment], buffer: &mut Vec<SplitDn>) {
        for (i, s) in new_segments.iter().enumerate() {
            let index = offset + i;
            buffer.push(SplitDn::with_segment(index, s));
        }
    }

    fn sort_vertically_by_min(segments: &mut [Segment], counts: &[usize]) {
        let mut start = 0;
        for &count in counts.iter() {
            let slice = &mut segments[start..start + count];
            slice.sort_unstable_by(|s0, s1| {
                s0.range
                    .min
                    .cmp(&s1.range.min)
                    .then(s0.pos.cmp(&s1.pos))
                    .then(s0.range.max.cmp(&s1.range.max))
            });
            start += count;
        }
    }

    fn sort_vertically_by_pos(segments: &mut [Segment], counts: &[usize]) {
        let mut start = 0;
        for &count in counts.iter() {
            let slice = &mut segments[start..start + count];
            slice.sort_unstable_by(|s0, s1| {
                s0.pos
                    .cmp(&s1.pos)
                    .then(s0.range.min.cmp(&s1.range.min))
                    .then(s0.range.max.cmp(&s1.range.max))
            });
            start += count;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct IndexEdge {
    pub(super) index: u32,
    pub(super) pos: i32,
    pub(super) range: LineRange,
}

impl IndexEdge {
    #[inline(always)]
    pub(super) fn new_vr(index: usize, segment: &Segment) -> Self {
        Self {
            index: index as u32,
            pos: segment.pos,
            range: segment.range,
        }
    }
}

trait SplitSegments {
    fn split_as_vr(&mut self, marks: &[YMark]);
    fn split_as_hz(&mut self, marks: &[XMark]);
    fn split_as_dp(&mut self, marks: &[XMark]);
    fn split_as_dn(&mut self, marks: &[XMark]);
}

impl SplitSegments for Vec<Segment> {
    #[inline]
    fn split_as_vr(&mut self, marks: &[YMark]) {
        let mut m0 = if let Some(&m) = marks.first() {
            m
        } else {
            return;
        };
        self.reserve(marks.len());

        let mut tail = unsafe { self.get_unchecked_mut(m0.index as usize).cut_tail(m0.y) };

        for &m in marks.iter().skip(1) {
            if m.index == m0.index {
                if m.y == m0.y {
                    continue;
                }

                let head = tail.cut_head(m.y);
                self.push(head);
            } else {
                self.push(tail);
                tail = unsafe { self.get_unchecked_mut(m.index as usize).cut_tail(m.y) };
            }
            m0 = m;
        }

        self.push(tail);
    }

    #[inline]
    fn split_as_hz(&mut self, marks: &[XMark]) {
        let mut m0 = if let Some(&m) = marks.first() {
            m
        } else {
            return;
        };
        self.reserve(marks.len());

        let mut tail = unsafe { self.get_unchecked_mut(m0.index as usize).cut_tail(m0.x) };

        for &m in marks.iter().skip(1) {
            if m.index == m0.index {
                if m.x == m0.x {
                    continue;
                }
                let head = tail.cut_head(m.x);
                self.push(head);
            } else {
                self.push(tail);
                tail = unsafe { self.get_unchecked_mut(m.index as usize).cut_tail(m.x) };
            }

            m0 = m;
        }

        self.push(tail);
    }

    #[inline]
    fn split_as_dp(&mut self, marks: &[XMark]) {
        let mut m0 = if let Some(&m) = marks.first() {
            m
        } else {
            return;
        };
        self.reserve(marks.len());

        let mut tail = unsafe { self.get_unchecked_mut(m0.index as usize).cut_tail_dp(m0.x) };

        for &m in marks.iter().skip(1) {
            if m.index == m0.index {
                if m.x == m0.x {
                    continue;
                }

                let head = tail.cut_head_dp(m.x);
                self.push(head);
            } else {
                self.push(tail);
                tail = unsafe { self.get_unchecked_mut(m.index as usize).cut_tail_dp(m.x) };
            }
            m0 = m;
        }

        self.push(tail);
    }

    #[inline]
    fn split_as_dn(&mut self, marks: &[XMark]) {
        let mut m0 = if let Some(&m) = marks.first() {
            m
        } else {
            return;
        };
        self.reserve(marks.len());

        let mut tail = unsafe { self.get_unchecked_mut(m0.index as usize).cut_tail_dn(m0.x) };

        for &m in marks.iter().skip(1) {
            if m.index == m0.index {
                if m.x == m0.x {
                    continue;
                }
                let head = tail.cut_head_dn(m.x);
                self.push(head);
            } else {
                self.push(tail);
                tail = unsafe { self.get_unchecked_mut(m.index as usize).cut_tail_dn(m.x) };
            }
            m0 = m;
        }

        self.push(tail);
    }
}

impl Segment {
    #[inline(always)]
    fn cut_tail(&mut self, mid: i32) -> Self {
        let tail = Self {
            pos: self.pos,
            range: LineRange::with_min_max(mid, self.range.max),
            dir: self.dir,
        };

        self.range.max = mid;

        tail
    }

    #[inline(always)]
    fn cut_head(&mut self, mid: i32) -> Self {
        let head = Self {
            pos: self.pos,
            range: LineRange::with_min_max(self.range.min, mid),
            dir: self.dir,
        };

        self.range.min = mid;

        head
    }

    #[inline(always)]
    fn cut_tail_dp(&mut self, mid: i32) -> Self {
        let mid_y = PositiveDiagonal::new(self.range, self.pos).find_y(mid);
        let tail = Self {
            pos: mid_y,
            range: LineRange::with_min_max(mid, self.range.max),
            dir: self.dir,
        };

        self.range.max = mid;

        tail
    }

    #[inline(always)]
    fn cut_head_dp(&mut self, mid: i32) -> Self {
        let mid_y = PositiveDiagonal::new(self.range, self.pos).find_y(mid);
        let head = Self {
            pos: self.pos,
            range: LineRange::with_min_max(self.range.min, mid),
            dir: self.dir,
        };

        self.range.min = mid;
        self.pos = mid_y;

        head
    }

    #[inline(always)]
    fn cut_tail_dn(&mut self, mid: i32) -> Self {
        let mid_y = NegativeDiagonal::new(self.range, self.pos).find_y(mid);
        let tail = Self {
            pos: self.pos,
            range: LineRange::with_min_max(mid, self.range.max),
            dir: self.dir,
        };

        self.range.max = mid;
        self.pos = mid_y;

        tail
    }

    #[inline(always)]
    fn cut_head_dn(&mut self, mid: i32) -> Self {
        let mid_y = NegativeDiagonal::new(self.range, self.pos).find_y(mid);
        let head = Self {
            pos: mid_y,
            range: LineRange::with_min_max(self.range.min, mid),
            dir: self.dir,
        };

        self.range.min = mid;

        head
    }
}

impl Intersection {
    fn is_empty(&self) -> bool {
        self.vr_marks.is_empty()
            && self.hz_marks.is_empty()
            && self.dp_marks.is_empty()
            && self.dn_marks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::fill_rule::FillRule::{Negative, Positive};
    use crate::core::overlay_rule::OverlayRule::Intersect;
    use crate::core::shape_type::ShapeType;
    use crate::core::winding::WindingCount;
    use crate::gear::section::Section;
    use crate::gear::seg_iter::{DropCollinear, SegmentIterable};
    use crate::gear::segment::Segment;
    use crate::gear::source::GeometrySource;
    use crate::gear::split_buffer::{Intersection, SplitBuffer, SplitDn, SplitDp, XMark, YMark};
    use crate::gear::x_layout::XLayout;
    use crate::geom::diagonal::{Diagonal, NegativeDiagonal, PositiveDiagonal};
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_float::int::rect::IntRect;
    use i_shape::int::path::IntPath;
    use rand::Rng;

    impl GeometrySource {
        fn test_count(&self) -> usize {
            self.vr_list.len() + self.hz_list.len() + self.dp_list.len() + self.dn_list.len()
        }
    }

    impl Section {
        fn test_new(
            source: GeometrySource,
            rect: IntRect,
            avg_count_per_column: usize,
            max_parts_count: usize,
        ) -> Self {
            Self {
                layout: XLayout::with_rect(
                    rect,
                    source.test_count(),
                    avg_count_per_column,
                    max_parts_count,
                ),
                source,
            }
        }
    }

    #[test]
    fn test_0() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 3, 6, ShapeType::Subject),
                Segment::test_with_shape(0, 3, 5, ShapeType::Subject),
                Segment::test_with_shape(0, 3, 4, ShapeType::Subject),
                Segment::test_with_shape(0, 3, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 3, 2, ShapeType::Subject),
                Segment::test_with_shape(0, 3, 1, ShapeType::Subject),
            ],
            hz_list: vec![Segment::test_with_shape(0, 9, 7, ShapeType::Subject)],
            dp_list: vec![],
            dn_list: vec![],
        };

        let original = source.test_count();
        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.test_count(), original);
    }

    #[test]
    fn test_1() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 4, 0, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 2, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 4, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 5, ShapeType::Subject),
            ],
            hz_list: vec![Segment::test_with_shape(0, 5, 5, ShapeType::Subject)],
            dp_list: vec![],
            dn_list: vec![],
        };

        let original = source.test_count();
        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.test_count(), original);
    }

    #[test]
    fn test_2() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 4, 0, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 2, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 4, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 5, ShapeType::Subject),
            ],
            hz_list: vec![Segment::test_with_shape(0, 5, 4, ShapeType::Subject)],
            dp_list: vec![],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 6);
        assert_eq!(section.source.hz_list.len(), 5);
    }

    #[test]
    fn test_3() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 4, 0, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 2, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 4, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 5, ShapeType::Subject),
            ],
            hz_list: vec![
                Segment::test_with_shape(0, 5, 4, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 0, ShapeType::Subject),
            ],
            dp_list: vec![],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 6);
        assert_eq!(section.source.hz_list.len(), 10);
    }

    #[test]
    fn test_4() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 4, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 3, ShapeType::Subject),
            ],
            hz_list: vec![
                Segment::test_with_shape(0, 4, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 4, 3, ShapeType::Subject),
            ],
            dp_list: vec![],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 6);
        assert_eq!(section.source.hz_list.len(), 6);
    }

    #[test]
    fn test_5() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 6, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 5, ShapeType::Subject),
            ],
            hz_list: vec![
                Segment::test_with_shape(0, 6, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 5, ShapeType::Subject),
            ],
            dp_list: vec![],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 12);
        assert_eq!(section.source.hz_list.len(), 12);
    }

    #[test]
    fn test_6() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 6, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 5, ShapeType::Subject),
            ],
            hz_list: vec![
                Segment::test_with_shape(0, 6, 0, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 2, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 4, ShapeType::Subject),
            ],
            dp_list: vec![],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 9);
        assert_eq!(section.source.hz_list.len(), 12);
    }

    #[test]
    fn test_7() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 6, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 5, ShapeType::Subject),
            ],
            hz_list: vec![
                Segment::test_with_shape(0, 5, 0, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 2, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 4, ShapeType::Subject),
            ],
            dp_list: vec![],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 9);
        assert_eq!(section.source.hz_list.len(), 9);
    }

    #[test]
    fn test_8() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 6, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 6, 5, ShapeType::Subject),
            ],
            hz_list: vec![],
            dp_list: vec![Segment::test_with_shape(0, 5, 0, ShapeType::Subject)],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 6);
        assert_eq!(section.source.dp_list.len(), 3);
    }

    #[test]
    fn test_9() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 5, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 5, ShapeType::Subject),
            ],
            hz_list: vec![],
            dp_list: vec![Segment::test_with_shape(0, 6, 0, ShapeType::Subject)],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 5);
        assert_eq!(section.source.dp_list.len(), 4);
    }

    #[test]
    fn test_10() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 5, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 5, ShapeType::Subject),
            ],
            hz_list: vec![],
            dp_list: vec![
                Segment::test_with_shape(0, 5, 0, ShapeType::Subject),
                Segment::test_with_shape(3, 5, 0, ShapeType::Subject),
            ],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 6);
        assert_eq!(section.source.dp_list.len(), 4);
    }

    #[test]
    fn test_11() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![
                Segment::test_with_shape(0, 5, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 5, ShapeType::Subject),
            ],
            hz_list: vec![],
            dp_list: vec![
                Segment::test_with_shape(0, 6, 0, ShapeType::Subject),
                Segment::test_with_shape(3, 6, 0, ShapeType::Subject),
            ],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.vr_list.len(), 6);
        assert_eq!(section.source.dp_list.len(), 6);
    }

    #[test]
    fn test_12() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![],
            hz_list: vec![
                Segment::test_with_shape(0, 5, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 5, ShapeType::Subject),
            ],
            dp_list: vec![
                Segment::test_with_shape(0, 6, 0, ShapeType::Subject),
                Segment::test_with_shape(3, 6, 0, ShapeType::Subject),
            ],
            dn_list: vec![],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.hz_list.len(), 6);
        assert_eq!(section.source.dp_list.len(), 6);
    }

    #[test]
    fn test_13() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![],
            hz_list: vec![
                Segment::test_with_shape(0, 5, 1, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 3, ShapeType::Subject),
                Segment::test_with_shape(0, 5, 5, ShapeType::Subject),
            ],
            dp_list: vec![],
            dn_list: vec![
                Segment::test_with_shape(0, 6, 0, ShapeType::Subject),
                Segment::test_with_shape(0, 2, 0, ShapeType::Subject),
            ],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.hz_list.len(), 6);
        assert_eq!(section.source.dn_list.len(), 6);
    }

    #[test]
    fn test_14() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![],
            hz_list: vec![],
            dp_list: vec![Segment::test_with_shape(0, 2, 0, ShapeType::Subject)],
            dn_list: vec![Segment::test_with_shape(0, 2, 0, ShapeType::Subject)],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.dp_list.len(), 2);
        assert_eq!(section.source.dn_list.len(), 2);
    }

    #[test]
    fn test_15() {
        let rect = IntRect::new(0, 16, 0, 16);
        let source = GeometrySource {
            vr_list: vec![],
            hz_list: vec![],
            dp_list: vec![Segment::test_with_shape(0, 2, 0, ShapeType::Subject)],
            dn_list: vec![Segment::test_with_shape(2, 4, 0, ShapeType::Subject)],
        };

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.dp_list.len(), 1);
        assert_eq!(section.source.dn_list.len(), 1);
    }

    #[test]
    fn test_16() {
        let contour = vec![
            IntPoint::new(0, 0),
            IntPoint::new(0, 2),
            IntPoint::new(-4, 2),
            IntPoint::new(-2, 4),
            IntPoint::new(-2, 0),
        ];

        let mut section = if let Some(s) = contour_to_section(&contour, 7, 8) {
            s
        } else {
            return;
        };
        let result_0 = section.source.brute_force_intersection();
        let result_1 = section.test_intersection();

        assert_eq!(result_0.vr_marks.len(), result_1.vr_marks.len());
        assert_eq!(result_0.hz_marks.len(), result_1.hz_marks.len());
        assert_eq!(result_0.dp_marks.len(), result_1.dp_marks.len());
        assert_eq!(result_0.dn_marks.len(), result_1.dn_marks.len());
    }

    #[test]
    fn test_17() {
        let contour = vec![
            IntPoint::new(0, 0),
            IntPoint::new(2, 0),
            IntPoint::new(2, 2),
            IntPoint::new(-2, -2),
            IntPoint::new(-8, -2),
            IntPoint::new(0, -2),
        ];

        let mut section = if let Some(s) = contour_to_section(&contour, 7, 8) {
            s
        } else {
            return;
        };
        let result_0 = section.source.brute_force_intersection();
        let result_1 = section.test_intersection();

        assert_eq!(result_0.vr_marks.len(), result_1.vr_marks.len());
        assert_eq!(result_0.hz_marks.len(), result_1.hz_marks.len());
        assert_eq!(result_0.dp_marks.len(), result_1.dp_marks.len());
        assert_eq!(result_0.dn_marks.len(), result_1.dn_marks.len());
    }

    #[test]
    fn test_random_0() {
        for _ in 0..1000 {
            let mut section = if let Some(s) = get_random_90_deg_section(16, 6, 4, 6) {
                s
            } else {
                return;
            };

            let result_0 = section.source.brute_force_intersection();
            let result_1 = section.test_intersection();

            assert_eq!(result_0.vr_marks.len(), result_1.vr_marks.len());
            assert_eq!(result_0.hz_marks.len(), result_1.hz_marks.len());
            assert_eq!(result_0.dp_marks.len(), result_1.dp_marks.len());
            assert_eq!(result_0.dn_marks.len(), result_1.dn_marks.len());
        }
    }

    #[test]
    fn test_random_1() {
        for _ in 0..1000 {
            let mut section = if let Some(s) = get_random_90_deg_section(64, 8, 7, 8) {
                s
            } else {
                return;
            };
            let result_0 = section.source.brute_force_intersection();
            let result_1 = section.test_intersection();

            assert_eq!(result_0.vr_marks.len(), result_1.vr_marks.len());
            assert_eq!(result_0.hz_marks.len(), result_1.hz_marks.len());
            assert_eq!(result_0.dp_marks.len(), result_1.dp_marks.len());
            assert_eq!(result_0.dn_marks.len(), result_1.dn_marks.len());
        }
    }

    #[test]
    fn test_random_2() {
        for _ in 0..10_000 {
            let contour = random_45_deg_contour(4, 3);
            let mut section = if let Some(s) = contour_to_section(&contour, 7, 8) {
                s
            } else {
                return;
            };
            let result_0 = section.source.brute_force_intersection();
            let result_1 = section.test_intersection();

            let test_0 = result_0.vr_marks.len() != result_1.vr_marks.len();
            let test_1 = result_0.hz_marks.len() != result_1.hz_marks.len();
            let test_2 = result_0.dp_marks.len() != result_1.dp_marks.len();
            let test_3 = result_0.dn_marks.len() != result_1.dn_marks.len();

            if test_0 || test_1 || test_2 || test_3 {
                assert_eq!(result_0.vr_marks.len(), result_1.vr_marks.len());
                assert_eq!(result_0.hz_marks.len(), result_1.hz_marks.len());
                assert_eq!(result_0.dp_marks.len(), result_1.dp_marks.len());
                assert_eq!(result_0.dn_marks.len(), result_1.dn_marks.len());
            }
        }
    }

    #[test]
    fn test_random_3() {
        for _ in 0..10_000 {
            let mut section = if let Some(s) = get_random_45_deg_section(64, 8, 7, 8) {
                s
            } else {
                return;
            };
            let result_0 = section.source.brute_force_intersection();
            let result_1 = section.test_intersection();

            assert_eq!(result_0.vr_marks.len(), result_1.vr_marks.len());
            assert_eq!(result_0.hz_marks.len(), result_1.hz_marks.len());
            assert_eq!(result_0.dp_marks.len(), result_1.dp_marks.len());
            assert_eq!(result_0.dn_marks.len(), result_1.dn_marks.len());
        }
    }

    fn random_90_deg_contour(n: usize, radius: i32) -> Vec<IntPoint> {
        let mut x = 0;
        let mut y = 0;

        let mut contour = IntPath::new();
        let mut rng = rand::rng();
        let range = -radius..=radius;

        contour.push(IntPoint::new(x, y));
        for i in 0..n {
            let ds = rng.random_range(range.clone());
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

    fn random_45_deg_contour(n: usize, radius: i32) -> Vec<IntPoint> {
        let mut x = 0;
        let mut y = 0;

        let mut contour = IntPath::new();
        let mut rng = rand::rng();
        let range = -radius..=radius;

        contour.push(IntPoint::new(x, y));
        for i in 0..n {
            let ds = 2 * rng.random_range(range.clone());
            match i % 3 {
                0 => {
                    x += ds;
                }
                1 => {
                    y += ds;
                }
                _ => {
                    x += ds;
                    y += ds;
                }
            }

            contour.push(IntPoint::new(x, y));
        }

        if x != 0 {
            contour.push(IntPoint::new(0, y));
        }
        contour
    }

    fn contour_to_section(
        contour: &[IntPoint],
        avg_count_per_column: usize,
        max_parts_count: usize,
    ) -> Option<Section> {
        let iter = contour.segment_iter::<DropCollinear>()?;
        let mut hz_list = Vec::new();
        let mut vr_list = Vec::new();
        let mut dp_list = Vec::new();
        let mut dn_list = Vec::new();

        for s in iter {
            if s[0].x == s[1].x {
                vr_list.push(Segment::test_with_shape(
                    s[0].y,
                    s[1].y,
                    s[0].x,
                    ShapeType::Subject,
                ));
            } else if s[0].y == s[1].y {
                hz_list.push(Segment::test_with_shape(
                    s[0].x,
                    s[1].x,
                    s[0].y,
                    ShapeType::Subject,
                ));
            } else {
                let (a, b) = if s[0].x < s[1].x {
                    (s[0], s[1])
                } else {
                    (s[1], s[0])
                };

                if a.y < b.y {
                    dp_list.push(Segment::test_with_shape(
                        a.x,
                        b.x,
                        a.y,
                        ShapeType::Subject,
                    ))
                } else {
                    dn_list.push(Segment::test_with_shape(
                        a.x,
                        b.x,
                        b.y,
                        ShapeType::Subject,
                    ))
                }
            }
        }

        let source = GeometrySource {
            vr_list,
            hz_list,
            dp_list,
            dn_list,
        };

        let rect = IntRect::with_points(&contour)?;

        Some(Section::test_new(
            source,
            rect,
            avg_count_per_column,
            max_parts_count,
        ))
    }

    fn get_random_45_deg_section(
        n: usize,
        radius: i32,
        avg_count_per_column: usize,
        max_parts_count: usize,
    ) -> Option<Section> {
        let contour = random_45_deg_contour(n, radius);
        contour_to_section(&contour, avg_count_per_column, max_parts_count)
    }

    fn get_random_90_deg_section(
        n: usize,
        radius: i32,
        avg_count_per_column: usize,
        max_parts_count: usize,
    ) -> Option<Section> {
        let contour = random_90_deg_contour(n, radius);
        contour_to_section(&contour, avg_count_per_column, max_parts_count)
    }

    impl Section {
        fn test_intersection(&mut self) -> Intersection {
            let mut source_by_columns = self.source.new_same_size();
            let map_by_columns = self
                .source
                .map_by_columns(&self.layout, &mut source_by_columns);

            self.intersect(&mut source_by_columns, &map_by_columns)
        }
    }

    impl GeometrySource {
        fn brute_force_intersection(&self) -> Intersection {
            let mut intersection = Intersection {
                vr_marks: vec![],
                hz_marks: vec![],
                dp_marks: vec![],
                dn_marks: vec![],
            };

            for vr in self.vr_list.iter() {
                let x = vr.pos;
                for hz in self.hz_list.iter() {
                    let y = hz.pos;
                    if hz.range.not_contains(x) || vr.range.not_contains(y) {
                        continue;
                    }

                    if hz.range.strict_contains(x) {
                        intersection.hz_marks.push(XMark { index: 0, x });
                    }

                    if vr.range.strict_contains(y) {
                        intersection.vr_marks.push(YMark { index: 0, y });
                    }
                }

                for dp in self.dp_list.iter() {
                    if dp.range.not_contains(x) {
                        continue;
                    }

                    let y = PositiveDiagonal::new(dp.range, dp.pos).find_y(x);

                    if vr.range.not_contains(y) {
                        continue;
                    }

                    if dp.range.strict_contains(x) {
                        intersection.dp_marks.push(XMark { index: 0, x });
                    }

                    if vr.range.strict_contains(y) {
                        intersection.vr_marks.push(YMark { index: 0, y });
                    }
                }

                for dn in self.dn_list.iter() {
                    if dn.range.not_contains(x) {
                        continue;
                    }

                    let y = NegativeDiagonal::new(dn.range, dn.pos).find_y(x);
                    if vr.range.not_contains(y) {
                        continue;
                    }

                    if dn.range.strict_contains(x) {
                        intersection.dn_marks.push(XMark { index: 0, x });
                    }

                    if vr.range.strict_contains(y) {
                        intersection.vr_marks.push(YMark { index: 0, y });
                    }
                }
            }

            for hz in self.hz_list.iter() {
                let y = hz.pos;
                for dp in self.dp_list.iter() {
                    if dp.y_range_dp().not_contains(y) {
                        continue;
                    }

                    let x = PositiveDiagonal::new(dp.range, dp.pos).find_x(y);
                    if hz.range.not_contains(x) {
                        continue;
                    }

                    if dp.y_range_dp().strict_contains(y) {
                        intersection.dp_marks.push(XMark { index: 0, x });
                    }

                    if hz.range.strict_contains(x) {
                        intersection.hz_marks.push(XMark { index: 0, x });
                    }
                }

                for dn in self.dn_list.iter() {
                    if dn.y_range_dn().not_contains(y) {
                        continue;
                    }

                    let x = NegativeDiagonal::new(dn.range, dn.pos).find_x(y);
                    if hz.range.not_contains(x) {
                        continue;
                    }

                    if dn.y_range_dn().strict_contains(y) {
                        intersection.dn_marks.push(XMark { index: 0, x });
                    }

                    if hz.range.strict_contains(x) {
                        intersection.hz_marks.push(XMark { index: 0, x });
                    }
                }
            }

            for dp in self.dp_list.iter() {
                for dn in self.dn_list.iter() {
                    let p = Self::test_cross_dgs(dp, dn);

                    if dp.range.strict_contains(p.x) {
                        intersection.dp_marks.push(XMark { index: 0, x: p.x });
                    }

                    if dn.range.strict_contains(p.x) {
                        intersection.dn_marks.push(XMark { index: 0, x: p.x });
                    }
                }
            }

            intersection
        }

        #[inline(always)]
        fn test_cross_dgs(dp: &Segment, dn: &Segment) -> IntPoint {
            let sp = dp.y_range_dp().min.wrapping_sub(dp.range.min);
            let sn = dn.y_range_dn().min.wrapping_add(dn.range.max);

            let y = sp.wrapping_add(sn) >> 1;
            let x = PositiveDiagonal::new(dp.range, dp.pos).find_x(y);

            IntPoint::new(x, y)
        }
    }
}

impl Segment {
    fn y_range_dp(&self) -> LineRange {
        let min_y = self.pos;
        let max_y = PositiveDiagonal::new(self.range, self.pos).find_y(self.range.max);
        LineRange::with_min_max(min_y, max_y)
    }

    fn y_range_dn(&self) -> LineRange {
        let min_y = self.pos;
        let max_y = NegativeDiagonal::new(self.range, self.pos).find_y(self.range.min);
        LineRange::with_min_max(min_y, max_y)
    }
}
