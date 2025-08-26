use crate::gear::merge::Merge;
use crate::gear::section::Section;
use crate::gear::segment::Segment;
use crate::gear::split_buffer::{MarkResult, SplitBuffer, XMark, YMark};
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

        let mut start_vr = 0;
        let mut start_hz = 0;
        let mut start_dp = 0;
        let mut start_dn = 0;

        let scale = self.layout.count().ilog2();

        let mut hz_buffer = Vec::with_capacity(source_by_columns.hz_list.len() >> scale);
        let mut dp_buffer = Vec::with_capacity(source_by_columns.dp_list.len() >> scale);
        let mut dn_buffer = Vec::with_capacity(source_by_columns.dn_list.len() >> scale);

        let mut split_buffer =
            SplitBuffer::new(self.layout.boundary().clone(), self.layout.log_width());

        for (column_index, part) in map_by_columns.iter_by_parts().enumerate() {
            let (min_x, max_x) = self.layout.borders(column_index);

            // get slices to new column data

            let vr_slice = &source_by_columns.vr_list[start_vr..start_vr + part.count_vr];
            let hz_slice = &source_by_columns.hz_list[start_hz..start_hz + part.count_hz];
            let dp_slice = &source_by_columns.dp_list[start_dp..start_dp + part.count_dp];
            let dn_slice = &source_by_columns.dn_list[start_dn..start_dn + part.count_dn];

            // prepare column data

            // hz
            self.clean_by_min_x(min_x, &mut hz_buffer);
            self.add(start_hz, hz_slice, &mut hz_buffer);

            // dn
            self.clean_by_min_x(min_x, &mut dn_buffer);
            self.add(start_dn, dn_slice, &mut dn_buffer);

            // dp
            self.clean_by_min_x_for_dp(min_x, &mut dp_buffer);
            self.add(start_dp, dp_slice, &mut dp_buffer);

            // fill buffer
            let limit_x = max_x + 1;
            split_buffer.add_hz_edges(limit_x, &hz_buffer);
            split_buffer.add_dp_edges(limit_x, &dp_buffer);
            split_buffer.add_dn_edges(limit_x, &dn_buffer);

            // split

            // vr
            for (vr_index, vr) in vr_slice.iter().enumerate() {
                let e = IndexEdge {
                    index: vr_index as u32,
                    pos: vr.pos,
                    range: vr.range,
                };
                split_buffer.intersect_vr(e);
            }

            split_buffer.intersect();

            start_vr += part.count_vr;
            start_hz += part.count_hz;
            start_dp += part.count_dp;
            start_dn += part.count_dn;
        }

        let result = split_buffer.into_marks();

        if result.is_empty() {
            swap(&mut self.source, &mut source_by_columns);
        } else {
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

            map_by_columns = source_by_columns.map_by_columns(&self.layout, &mut self.source);
        }

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
    fn clean_by_min_x_for_dp(&mut self, min_x: i32, buffer: &mut Vec<IndexEdge>) {
        buffer.retain_mut(|e| {
            if e.range.min < min_x {
                false
            } else {
                let x = min_x;
                let dx = x - e.range.min;
                e.range.min = x;
                e.pos += dx;

                true
            }
        });
    }

    #[inline]
    fn clean_by_min_x(&mut self, min_x: i32, buffer: &mut Vec<IndexEdge>) {
        buffer.retain_mut(|e| {
            if e.range.min < min_x {
                false
            } else {
                e.range.min = min_x;
                true
            }
        });
    }

    fn add(&mut self, offset: usize, new_segments: &[Segment], buffer: &mut Vec<IndexEdge>) {
        for (i, s) in new_segments.iter().enumerate() {
            let index = offset + i;
            buffer.push(IndexEdge::new(index, s));
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
    fn new(index: usize, segment: &Segment) -> Self {
        Self {
            index: index as u32,
            pos: segment.pos, // for vr -> x, hz -> y, dg -> min y
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
        let mut m0 = if let Some(first) = marks.first() {
            first.clone()
        } else {
            return;
        };

        let mut tail = unsafe { self.get_unchecked_mut(m0.index as usize).cut_tail(m0.y) };

        for &m in marks.iter().skip(1) {
            if m.index == m0.index {
                if m.y == m.y {
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
        let mut m0 = if let Some(first) = marks.first() {
            first.clone()
        } else {
            return;
        };

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
        let mut m0 = if let Some(first) = marks.first() {
            first.clone()
        } else {
            return;
        };

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
        let mut m0 = if let Some(first) = marks.first() {
            first.clone()
        } else {
            return;
        };

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
        let y = PositiveDiagonal::new(self.range, self.pos).find_y(mid);
        let tail = Self {
            pos: y,
            range: LineRange::with_min_max(mid, self.range.max),
            dir: self.dir,
        };

        self.range.max = mid;

        tail
    }

    #[inline(always)]
    fn cut_head_dp(&mut self, mid: i32) -> Self {
        let y = PositiveDiagonal::new(self.range, self.pos).find_y(mid);
        let head = Self {
            pos: self.pos,
            range: LineRange::with_min_max(self.range.min, mid),
            dir: self.dir,
        };

        self.range.min = mid;
        self.pos = y;

        head
    }

    #[inline(always)]
    fn cut_tail_dn(&mut self, mid: i32) -> Self {
        let y = NegativeDiagonal::new(self.range, self.pos).find_y(mid);
        let tail = Self {
            pos: y,
            range: LineRange::with_min_max(mid, self.range.max),
            dir: self.dir,
        };

        self.range.max = mid;

        tail
    }

    #[inline(always)]
    fn cut_head_dn(&mut self, mid: i32) -> Self {
        let y = NegativeDiagonal::new(self.range, self.pos).find_y(mid);
        let head = Self {
            pos: self.pos,
            range: LineRange::with_min_max(self.range.min, mid),
            dir: self.dir,
        };

        self.range.min = mid;
        self.pos = y;

        head
    }
}

impl MarkResult {
    fn is_empty(&self) -> bool {
        self.vr_marks.is_empty()
            && self.hz_marks.is_empty()
            && self.dp_marks.is_empty()
            && self.dn_marks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::shape_type::ShapeType;
    use crate::gear::section::Section;
    use crate::gear::segment::Segment;
    use crate::gear::source::GeometrySource;
    use crate::gear::x_layout::XLayout;
    use alloc::vec;
    use i_float::int::rect::IntRect;

    impl GeometrySource {
        fn count(&self) -> usize {
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
                    source.count(),
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

        let mut section = Section::test_new(source, rect, 3, 5);

        section.split();

        assert_eq!(section.source.count(), 7);
    }
}
