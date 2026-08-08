use crate::core::fill_rule::FillRule;
use crate::core::integer::OverlayInt;
use crate::core::overlay_rule::OverlayRule;
use crate::core::winding::WindingCount;
use crate::definition::fill::{
    EvenOddStrategy, FillStrategy, NegativeStrategy, NonZeroStrategy, PositiveStrategy,
};
use crate::definition::filter::{
    ClipFilter, DifferenceFilter, FilterStrategy, IntersectFilter, InverseDifferenceFilter,
    SubjectFilter, UnionFilter, XorFilter,
};
use crate::definition::segment::SegmentFill;
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::column::anchor::Anchor;
use crate::deg_90::column::graph::{ColumnGraph, Node};
use crate::deg_90::column::link::{Link, LinkIndex};
use crate::deg_90::column::stack_iter::StackIter;
use crate::deg_90::column_map::Column;
use crate::geom::segment::Segment;
use alloc::vec::Vec;
use core::mem::swap;
use core::num::NonZeroU32;
use i_float::int::point::IntPoint;

impl<I: OverlayInt> ColumnGraph<I> {
    #[cfg(test)]
    #[rustfmt::skip]
    pub(crate) fn new<W: WindingCount>(column: &Column<I, W>, fill_rule: FillRule, overlay_rule: OverlayRule, buffer: &mut ScanBuffer<I, W>) -> Self {
        Self::with_nodes(column, fill_rule, overlay_rule, buffer, Vec::new())
    }

    #[rustfmt::skip]
    pub(crate) fn with_nodes<W: WindingCount>(column: &Column<I, W>, fill_rule: FillRule, overlay_rule: OverlayRule, buffer: &mut ScanBuffer<I, W>, nodes: Vec<Node<I>>) -> Self {
        match fill_rule {
            FillRule::EvenOdd => Self::with_fill_strategy::<W, EvenOddStrategy>(column, overlay_rule, buffer, nodes),
            FillRule::NonZero => Self::with_fill_strategy::<W, NonZeroStrategy>(column, overlay_rule, buffer, nodes),
            FillRule::Positive => Self::with_fill_strategy::<W, PositiveStrategy>(column, overlay_rule, buffer, nodes),
            FillRule::Negative => Self::with_fill_strategy::<W, NegativeStrategy>(column, overlay_rule, buffer, nodes),
        }
    }

    #[rustfmt::skip]
    fn with_fill_strategy<W, Fill>(
        column: &Column<I, W>,
        overlay_rule: OverlayRule,
        buffer: &mut ScanBuffer<I, W>,
        nodes: Vec<Node<I>>,
    ) -> Self
    where
        W: WindingCount,
        Fill: FillStrategy<ShapeCountBoolean<W>>,
    {
        match overlay_rule {
            OverlayRule::Subject => Self::with_fill_and_filter_strategy::<W, Fill, SubjectFilter>(column, buffer, nodes),
            OverlayRule::Clip => Self::with_fill_and_filter_strategy::<W, Fill, ClipFilter>(column, buffer, nodes),
            OverlayRule::Intersect => Self::with_fill_and_filter_strategy::<W, Fill, IntersectFilter>(column, buffer, nodes),
            OverlayRule::Union => Self::with_fill_and_filter_strategy::<W, Fill, UnionFilter>(column, buffer, nodes),
            OverlayRule::Difference => Self::with_fill_and_filter_strategy::<W, Fill, DifferenceFilter>(column, buffer, nodes),
            OverlayRule::Xor => Self::with_fill_and_filter_strategy::<W, Fill, XorFilter>(column, buffer, nodes),
            OverlayRule::InverseDifference => Self::with_fill_and_filter_strategy::<W, Fill, InverseDifferenceFilter>(column, buffer, nodes),
        }
    }

    fn with_fill_and_filter_strategy<W, Fill, Filter>(
        column: &Column<I, W>,
        buffer: &mut ScanBuffer<I, W>,
        mut nodes: Vec<Node<I>>,
    ) -> Self
    where
        W: WindingCount,
        Fill: FillStrategy<ShapeCountBoolean<W>>,
        Filter: FilterStrategy,
    {
        let n = column.segments.len();
        buffer.clear();

        nodes.clear();
        nodes.reserve(n);

        let mut i = 0;
        while i < n {
            let start = i;
            let pos = column.segments[i].pos;
            i += 1;

            while i < n && column.segments[i].pos == pos {
                i += 1;
            }
            buffer.add_segments::<Fill, Filter>(&column.segments[start..i], &mut nodes);
        }

        Self { nodes }
    }
}

impl<I: OverlayInt> Node<I> {
    #[inline(always)]
    fn new(point: IntPoint<I>) -> Self {
        Self {
            point,
            links: [Link::empty(); 4],
        }
    }

    #[inline(always)]
    fn set_link(&mut self, link: Link, index: LinkIndex) {
        self.links[index.order()] = link;
    }
}

struct NodeCursor {
    node: u32,
    fill: SegmentFill,
}

const IN_PLACE_SEGMENTS_LIMIT: usize = 16;

#[derive(Default)]
pub(crate) struct ScanBuffer<I: OverlayInt, W: WindingCount = i16> {
    active: Vec<Anchor<I, W>>,
    buffer: Vec<Anchor<I, W>>,
}

impl<I: OverlayInt, W: WindingCount> ScanBuffer<I, W> {
    #[cfg(test)]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            active: Vec::with_capacity(capacity),
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub(super) fn reserve(&mut self, capacity: usize) {
        if self.active.capacity() < capacity {
            self.active.reserve(capacity - self.active.len());
        }
        if self.buffer.capacity() < capacity {
            self.buffer.reserve(capacity - self.buffer.len());
        }
    }

    fn add_segments<Fill, Filter>(&mut self, segments: &[Segment<I, W>], nodes: &mut Vec<Node<I>>)
    where
        Fill: FillStrategy<ShapeCountBoolean<W>>,
        Filter: FilterStrategy,
    {
        if segments.len() < IN_PLACE_SEGMENTS_LIMIT {
            self.add_segments_in_place::<Fill, Filter>(segments, nodes);
        } else {
            self.add_segments_buffered::<Fill, Filter>(segments, nodes);
        }
    }

    fn add_segments_buffered<Fill, Filter>(
        &mut self,
        segments: &[Segment<I, W>],
        nodes: &mut Vec<Node<I>>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean<W>>,
        Filter: FilterStrategy,
    {
        // segments are always sorted by range.min and not overlap each other
        // s(i).range.max <= s(i+1).range.min
        // segment.count.is_not_empty() === true
        // the segments not more < 100..200 elements
        // the buffers in average much less than 1000 and close to segments.len

        let first_segment = segments.first().unwrap();
        let y = first_segment.pos;
        let first_x = first_segment.range.min;
        let last_x = segments.last().unwrap().range.max;
        let active_start = self.active.partition_point(|anchor| anchor.x < first_x);
        let active_end = self.active.partition_point(|anchor| anchor.x <= last_x);

        // Skip unchanged topology outside the current line's segment range by carrying those
        // active anchors directly to the next line.
        self.buffer
            .extend_copy_from_slice(&self.active[..active_start]);

        Self::append_changed_topology::<Fill, Filter>(
            segments,
            &self.active[active_start..],
            last_x,
            y,
            nodes,
            &mut self.buffer,
        );

        if let Some((first, rest)) = self.active[active_end..].split_first() {
            self.buffer.push_and_merge(*first);
            self.buffer.extend_copy_from_slice(rest);
        }

        self.buffer.remove_empty_anchor();
        swap(&mut self.active, &mut self.buffer);
        self.buffer.clear();
    }

    fn add_segments_in_place<Fill, Filter>(
        &mut self,
        segments: &[Segment<I, W>],
        nodes: &mut Vec<Node<I>>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean<W>>,
        Filter: FilterStrategy,
    {
        let first_segment = segments.first().unwrap();
        let y = first_segment.pos;
        let first_x = first_segment.range.min;
        let last_x = segments.last().unwrap().range.max;
        let active_start = self.active.partition_point(|anchor| anchor.x < first_x);
        let active_end = self.active.partition_point(|anchor| anchor.x <= last_x);

        debug_assert!(self.buffer.is_empty());
        Self::append_changed_topology::<Fill, Filter>(
            segments,
            &self.active[active_start..],
            last_x,
            y,
            nodes,
            &mut self.buffer,
        );

        let mut replace_start = active_start;
        if replace_start > 0
            && self
                .buffer
                .first()
                .is_some_and(|first| first.count.left == self.active[replace_start - 1].count.left)
        {
            replace_start -= 1;
        }

        if let Some(suffix) = self.active.get(active_end) {
            if self
                .buffer
                .last()
                .is_some_and(|last| last.count.left == suffix.count.left)
            {
                self.buffer.pop();
            } else if self.buffer.is_empty()
                && replace_start == active_start
                && replace_start > 0
                && self.active[replace_start - 1].count.left == suffix.count.left
            {
                replace_start -= 1;
            }
        }

        // With only a few new segments, replace just the topology that can change and keep the
        // unchanged active prefix and suffix in place.
        drop(
            self.active
                .splice(replace_start..active_end, self.buffer.drain(..)),
        );
        self.active.remove_empty_anchor();
    }

    fn append_changed_topology<Fill, Filter>(
        segments: &[Segment<I, W>],
        active: &[Anchor<I, W>],
        last_x: I,
        y: I,
        nodes: &mut Vec<Node<I>>,
        buffer: &mut Vec<Anchor<I, W>>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean<W>>,
        Filter: FilterStrategy,
    {
        let mut left_cursor: Option<NodeCursor> = None;

        for sp in StackIter::new(segments, active) {
            // The first anchor beyond last_x still supplies count.left to the iterator.
            if sp.x > last_x {
                break;
            }

            let down_link = !sp.node.is_none();

            let up_fill = Fill::fill(sp.c0, sp.c1);
            let up_link = Filter::is_included(up_fill);

            if down_link || up_link {
                let p = IntPoint::new(sp.x, y);

                let (this, up) = if let Some(node) = sp.node {
                    // down node is existed
                    // up node is still possible

                    let this = node.get();
                    let n = nodes.len();
                    let node = &mut nodes[this as usize];
                    node.point = p;

                    let mut up_node = 0;
                    if up_link {
                        let up = n as u32;
                        node.set_link(Link::new(up, up_fill), LinkIndex::Up);

                        let mut top = Node::new(IntPoint::EMPTY);
                        top.set_link(Link::new(this, up_fill), LinkIndex::Down);
                        nodes.push(top);
                        up_node = up;
                    };

                    (this, NonZeroU32::new(up_node))
                } else {
                    // down node is not exist
                    // this and up node must be created

                    let this = nodes.len() as u32;
                    let up = this + 1;

                    let mut node = Node::new(p);
                    node.set_link(Link::new(up, up_fill), LinkIndex::Up);

                    let mut top = Node::new(IntPoint::EMPTY);
                    top.set_link(Link::new(this, up_fill), LinkIndex::Down);

                    nodes.push(node);
                    nodes.push(top);

                    (this, NonZeroU32::new(up))
                };

                if let Some(cursor) = &left_cursor {
                    let left = cursor.node;
                    nodes[left as usize].set_link(Link::new(this, cursor.fill), LinkIndex::Right);
                    nodes[this as usize].set_link(Link::new(left, cursor.fill), LinkIndex::Left);
                    left_cursor = None;
                }

                let right_fill = Fill::fill(sp.c1, sp.cb);
                let right_link = Filter::is_included(right_fill);

                if right_link {
                    left_cursor = Some(NodeCursor {
                        node: this,
                        fill: right_fill,
                    });
                };

                buffer.push_and_merge(Anchor::new(sp.x, up, sp.c0, sp.c1));
            } else {
                buffer.push_and_merge(Anchor::new(sp.x, None, sp.c0, sp.c1));
            }
        }
    }

    #[inline]
    pub(crate) fn clear(&mut self) {
        self.active.clear();
        self.buffer.clear();
    }
}
trait AnchorBuffer<I: OverlayInt, W: WindingCount> {
    fn extend_copy_from_slice(&mut self, anchors: &[Anchor<I, W>]);

    fn push_and_merge(&mut self, anchor: Anchor<I, W>);

    fn remove_empty_anchor(&mut self);
}

impl<I: OverlayInt, W: WindingCount> AnchorBuffer<I, W> for Vec<Anchor<I, W>> {
    #[inline(always)]
    fn extend_copy_from_slice(&mut self, anchors: &[Anchor<I, W>]) {
        let len = self.len();
        self.reserve(anchors.len());

        // SAFETY: reserve provides enough uninitialized space, and `&mut self` prevents an
        // overlapping source slice in safe code.
        unsafe {
            core::ptr::copy_nonoverlapping(
                anchors.as_ptr(),
                self.as_mut_ptr().add(len),
                anchors.len(),
            );
            self.set_len(len + anchors.len());
        }
    }

    #[inline(always)]
    fn push_and_merge(&mut self, anchor: Anchor<I, W>) {
        if let Some(last) = self.last_mut()
            && last.count.left == anchor.count.left
        {
            debug_assert!(last.x <= anchor.x);
            *last = anchor;
        } else {
            self.push(anchor);
        }
    }

    #[inline(always)]
    fn remove_empty_anchor(&mut self) {
        if self.len() == 1 && self[0].count.left.is_empty() {
            _ = self.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::cpu_count::CPUCount;
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay_rule::OverlayRule;
    use crate::definition::segment::SegmentFill;
    use crate::deg_90::column::build::ScanBuffer;
    use crate::deg_90::column::graph::ColumnGraph;
    use crate::deg_90::column::link::LinkIndex;
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_overlay::vector::edge::Reverse;
    use i_shape::int::path::IntPath;
    use i_shape::int::shape::{IntContour, IntShape};
    use i_shape::int_shape;
    use rand::RngExt;

    #[test]
    fn test_0() {
        #[rustfmt::skip]
        test_contours(&int_shape![[
            [0, 0],
            [2, 0],
            [2, 2],
            [0, 2],
        ]]);
    }

    #[test]
    fn test_1() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [0, 0],
                [2, 0],
                [2, 2],
                [0, 2],
            ],
            [
                [2, 0],
                [4, 0],
                [4, 2],
                [2, 2],
            ],
        ]);
    }

    #[test]
    fn test_2() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [0, 0],
                [2, 0],
                [2, 2],
                [0, 2],
            ],
            [
                [0, 2],
                [2, 2],
                [2, 4],
                [0, 4],
            ],
        ]);
    }

    #[test]
    fn test_3() {
        #[rustfmt::skip]
        test_contours(&int_shape![[
            [ 0,  0],
            [ 0, -2],
            [ 2, -2],
            [ 2,  2],
            [-2,  2],
            [-2,  0],
        ]]);
    }

    #[test]
    fn test_4() {
        #[rustfmt::skip]
        test_contours(&int_shape![[
            [ 0,  0],
            [-2,  0],
            [-2, -2],
            [ 2, -2],
            [ 2,  2],
            [ 0,  2],
        ]]);
    }

    #[test]
    fn test_5() {
        // cross
        #[rustfmt::skip]
        test_contours(&int_shape![[
            [ -5,   5],
            [-15,   5],
            [-15,  -5],
            [ -5,  -5],
            [ -5, -15],
            [  5, -15],
            [  5,  -5],
            [ 15,  -5],
            [ 15,   5],
            [  5,   5],
            [  5,  15],
            [ -5,  15],
        ]]);
    }

    #[test]
    fn test_6() {
        // enclosing rects
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [-10,  10],
                [-10, -10],
                [ 10, -10],
                [ 10,  10],
            ],
            [
                [-5,  5],
                [-5, -5],
                [ 5, -5],
                [ 5,  5],
            ],
        ]);
    }

    #[test]
    fn test_7() {
        // window
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [-10,  10],
                [-10, -10],
                [ 10, -10],
                [ 10,  10],
            ],
            [
                [-5, -5],
                [-5,  5],
                [ 5,  5],
                [ 5, -5],
            ],
        ]);
    }

    #[test]
    fn test_8() {
        // split
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [-5,  10],
                [-5, -10],
                [ 5, -10],
                [ 5,  10],
            ],
            [
                [-5, -5],
                [-5,  5],
                [ 5,  5],
                [ 5, -5],
            ],
        ]);
    }

    #[test]
    fn test_9() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [-5,  10],
                [-5, -10],
                [ 5, -10],
                [ 5,  10],
            ],
            [
                [-5,  5],
                [-5, -5],
                [ 5, -5],
                [ 5,  5],
            ],
        ]);
    }

    #[test]
    fn test_10() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [ 0, -2],
                [-2, -2],
                [-2,  0],
                [ 2,  0],
                [ 2,  2],
                [ 0,  2],
            ],
        ]);
    }

    #[test]
    fn test_11() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [ 0, -2],
                [ 2, -2],
                [ 2,  0],
                [-2,  0],
                [-2,  2],
                [ 0,  2],
            ],
        ]);
    }

    #[test]
    fn test_12() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [ 0,  0],
                [ 0,  2],
                [ 2,  2],
                [ 2,  0],
                [ 0,  0],
                [ 0, -2],
                [-2, -2],
                [-2,  0],
            ],
        ]);
    }

    #[test]
    fn test_13() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [ 0,  0],
                [ 4,  0],
                [ 4, -2],
                [ 0, -2],
                [ 0,  0],
                [ 2,  0],
                [ 2, -2],
                [ 0, -2],
            ],
        ]);
    }

    #[test]
    fn test_14() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [0, 0],
                [2, 0],
                [2, 2],
                [0, 2],
            ],
            [
                [0, 0],
                [2, 0],
                [2, 2],
                [0, 2],
            ],
            [
                [2, 0],
                [4, 0],
                [4, 2],
                [2, 2],
            ],
        ]);
    }

    #[test]
    fn test_15() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [0, 0],
                [2, 0],
                [2, 4],
                [0, 4],
            ],
            [
                [0, 0],
                [2, 0],
                [2, 4],
                [0, 4],
            ],
            [
                [2, 0],
                [4, 0],
                [4, 2],
                [2, 2],
            ],
        ]);
    }

    #[test]
    fn test_16() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [ 0,  0],
                [ 2,  0],
                [ 2, -1],
                [-1, -1],
                [-1,  0],
                [ 1,  0],
                [ 1,  1],
                [ 0,  1],
            ]
        ]);
    }

    #[test]
    fn test_17() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [0, 1],
                [3, 1],
                [3, 0],
                [0, 0],
            ],
            [
                [1, 1],
                [2, 1],
                [2, 2],
                [1, 2],
            ],
        ]);
    }

    #[test]
    fn test_18() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [
                [0, 0], [3, 0], [3, -3], [2, -3], [2, 0], [-1, 0], [-1, 3], [-2, 3], [-2, 2], [0, 2], [0, 1], [-3, 1], [-3, 4], [0, 4]
            ],
        ]);
    }

    #[test]
    fn test_19() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [[5, 2], [10, 2], [10, 3], [5, 3]],
            [[4, 0], [5, 0], [5, 4], [4, 4]],
            [[5, 7], [10, 7], [10, 8], [5, 8]],
            [[5, 7], [7, 7], [7, 9], [5, 9]],
            [[6, 6], [8, 6], [8, 10], [6, 10]],
            [[0, 3], [1, 3], [1, 7], [0, 7]],
            [[4, 4], [11, 4], [11, 6], [4, 6]],
            [[7, 1], [11, 1], [11, 5], [7, 5]],
        ]);
    }

    #[test]
    fn test_20() {
        #[rustfmt::skip]
        test_contours(&int_shape![
            [[0, 0], [0, 1], [1, 1], [1, 0]],
            [[1, 0], [2, 0], [2, 1], [1, 1]]
        ]);
    }

    #[test]
    fn test_random_0() {
        for _ in 0..1000 {
            let contour = random_90_deg_contour(4, 4);
            test_contours(&vec![contour]);
        }
    }
    //
    #[test]
    fn test_random_1() {
        for _ in 0..1000 {
            let contour = random_90_deg_contour(5, 4);
            test_contours(&vec![contour]);
        }
    }

    #[test]
    fn test_random_2() {
        for _ in 0..10_000 {
            let contour = random_90_deg_contour(6, 4);
            test_contours(&vec![contour]);
        }
    }

    #[test]
    fn test_random_3() {
        for _ in 0..20_000 {
            let contour = random_90_deg_contour(8, 4);
            test_contours(&vec![contour]);
        }
    }

    #[test]
    fn test_random_4() {
        for _ in 0..20_000 {
            let contour = random_90_deg_contour(10, 4);
            test_contours(&vec![contour]);
        }
    }

    #[test]
    fn test_random_5() {
        for _ in 0..20_000 {
            let contour = random_90_deg_contour(12, 4);
            test_contours(&vec![contour]);
        }
    }

    #[test]
    fn test_random_6() {
        for _ in 0..20_000 {
            let contour = random_90_deg_contour(16, 4);
            test_contours(&vec![contour]);
        }
    }

    #[test]
    fn test_random_positive_rects_0() {
        for _ in 0..20_000 {
            let rects = random_positive_rects(4, 4, 2);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_positive_rects_1() {
        for _ in 0..20_000 {
            let rects = random_positive_rects(8, 4, 2);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_positive_rects_2() {
        for _ in 0..20_000 {
            let rects = random_positive_rects(8, 8, 4);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_positive_rects_3() {
        for _ in 0..10_000 {
            let rects = random_positive_rects(16, 10, 4);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_positive_rects_4() {
        for _ in 0..4_000 {
            let rects = random_positive_rects(32, 20, 4);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_rects_0() {
        for _ in 0..20_000 {
            let rects = random_rects(2, 4, 2);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_rects_1() {
        for _ in 0..20_000 {
            let rects = random_rects(4, 8, 4);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_rects_2() {
        for _ in 0..10_000 {
            let rects = random_rects(8, 10, 4);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_rects_3() {
        for _ in 0..4_000 {
            let rects = random_rects(16, 20, 4);
            test_contours(&rects);
        }
    }

    #[test]
    fn test_random_rects_4() {
        for _ in 0..4_000 {
            let rects = random_rects(32, 20, 4);
            test_contours(&rects);
        }
    }

    fn test_contours(contours: &IntShape<i32>) {
        let config = ColumnConfig90 {
            min_columns_count: 1,
            min_column_width_power: 20,
            max_allow_segments_per_column: 1_000_000_000,
            min_allowed_segments_per_column: 1_000_000,
            max_allowed_segments_per_line: 1_000_000,
        };

        let mut buffer = ScanBuffer::with_capacity(0);

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

    fn random_positive_rects(n: usize, radius: usize, size: usize) -> Vec<Vec<IntPoint>> {
        let mut rects = Vec::with_capacity(n);

        let mut rng = rand::rng();
        for _ in 0..n {
            let x = rng.random_range(0..radius) as i32;
            let y = rng.random_range(0..radius) as i32;
            let a = rng.random_range(1..size) as i32;
            let b = rng.random_range(1..size) as i32;

            let rect = vec![
                IntPoint::new(x, y),
                IntPoint::new(x + a, y),
                IntPoint::new(x + a, y + b),
                IntPoint::new(x, y + b),
            ];

            rects.push(rect);
        }

        rects
    }

    fn random_rects(n: usize, radius: usize, size: usize) -> Vec<Vec<IntPoint>> {
        let mut rects = Vec::with_capacity(n);

        let mut rng = rand::rng();
        for _ in 0..n {
            let x = rng.random_range(0..radius) as i32;
            let y = rng.random_range(0..radius) as i32;
            let a = rng.random_range(1..size) as i32;
            let b = rng.random_range(1..size) as i32;

            let rect = vec![
                IntPoint::new(x, y),
                IntPoint::new(x + a, y),
                IntPoint::new(x + a, y + b),
                IntPoint::new(x, y + b),
            ];

            rects.push(rect);
        }

        for _ in 0..n {
            let x = rng.random_range(0..radius) as i32;
            let y = rng.random_range(0..radius) as i32;
            let a = rng.random_range(1..size) as i32;
            let b = rng.random_range(1..size) as i32;

            let rect = vec![
                IntPoint::new(x, y),
                IntPoint::new(x, y + b),
                IntPoint::new(x + a, y + b),
                IntPoint::new(x + a, y),
            ];

            rects.push(rect);
        }

        rects
    }

    #[derive(Debug, PartialEq, Eq)]
    struct SegFill {
        a: IntPoint,
        b: IntPoint,
        f: u8,
    }

    impl SegFill {
        fn with_abf_normalize(a: IntPoint, b: IntPoint, f: SegmentFill) -> Self {
            if a < b {
                SegFill { a, b, f }
            } else {
                SegFill {
                    a: b,
                    b: a,
                    f: f.reverse(),
                }
            }
        }

        fn with_abf(a: IntPoint, b: IntPoint, f: SegmentFill) -> Self {
            if a < b {
                SegFill { a, b, f }
            } else {
                SegFill { a: b, b: a, f }
            }
        }
    }

    fn contour_to_template_s_fills(contours: &[IntContour<i32>]) -> Option<Vec<SegFill>> {
        let mut overlay = i_overlay::core::overlay::Overlay::with_contours(contours, &[]);
        let fill_rule = i_overlay::core::fill_rule::FillRule::NonZero;
        let overlay_rule = i_overlay::core::overlay_rule::OverlayRule::Subject;

        let graph = overlay.build_graph_view(fill_rule)?;
        let shapes = graph.extract_vector_shapes(overlay_rule, &mut Default::default());
        let mut s_fills = Vec::new();

        for shape in shapes {
            for contour in shape {
                for e in contour {
                    if e.a.y == e.b.y {
                        s_fills.push(SegFill::with_abf_normalize(e.a, e.b, e.fill))
                    }
                }
            }
        }

        s_fills.sort_by_key(|s| s.a);

        Some(s_fills)
    }

    fn contour_to_subject_s_fills(
        contours: &IntShape<i32>,
        config: ColumnConfig90,
        buffer: &mut ScanBuffer<i32>,
    ) -> Vec<SegFill> {
        let mut map = ColumnMap::with_subj_and_clip(contours, &[], CPUCount::Single, config);
        buffer.clear();
        debug_assert!(map.columns.len() == 1);
        let column = &mut map.columns[0];
        column.test_partition(config);
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Subject;

        let graph = ColumnGraph::new(column, fill_rule, overlay_rule, buffer);
        let mut s_fills = graph.hz_edges();
        s_fills.sort_by_key(|s| s.a);

        s_fills
    }

    impl ColumnGraph<i32> {
        fn hz_edges(&self) -> Vec<SegFill> {
            let mut visitors = self.visitors();
            let mut edges = Vec::with_capacity(visitors.len());
            for (node_index, node) in self.nodes.iter().enumerate() {
                for (order, link) in node.links.iter().enumerate() {
                    if link.is_empty() {
                        continue;
                    }

                    if !visitors[node_index].visit_if_not_yet(order) {
                        continue;
                    }

                    let dest = link.index();
                    _ = visitors[dest].visit_if_not_yet(LinkIndex::opposite_order(order));

                    let a = node.point;
                    let b = self.nodes[dest].point;

                    if a.y == b.y {
                        edges.push(SegFill::with_abf(a, b, link.fill()));
                    }
                }
            }
            edges
        }
    }
}
