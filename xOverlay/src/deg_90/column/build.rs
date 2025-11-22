use crate::core::fill_rule::FillRule;
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
use crate::deg_90::column::graph::{ColumnGraph, Node};
use crate::deg_90::column::link::{Link, LinkIndex};
use crate::deg_90::column_map::Column;
use crate::gear::segment::Segment;
use alloc::vec::Vec;
use core::mem::swap;
use core::num::NonZeroU32;
use i_float::int::point::IntPoint;

impl ColumnGraph {
    #[rustfmt::skip]
    pub(crate) fn new(column: &Column, fill_rule: FillRule, overlay_rule: OverlayRule, buffer: &mut ScanBuffer) -> Self {
        match fill_rule {
            FillRule::EvenOdd => Self::with_fill_strategy::<EvenOddStrategy>(column, overlay_rule, buffer),
            FillRule::NonZero => Self::with_fill_strategy::<NonZeroStrategy>(column, overlay_rule, buffer),
            FillRule::Positive => Self::with_fill_strategy::<PositiveStrategy>(column, overlay_rule, buffer),
            FillRule::Negative => Self::with_fill_strategy::<NegativeStrategy>(column, overlay_rule, buffer),
        }
    }

    #[rustfmt::skip]
    fn with_fill_strategy<Fill: FillStrategy<ShapeCountBoolean>>(
        column: &Column,
        overlay_rule: OverlayRule,
        buffer: &mut ScanBuffer
    ) -> Self {
        match overlay_rule {
            OverlayRule::Subject => Self::with_fill_and_filter_strategy::<Fill, SubjectFilter>(column, buffer),
            OverlayRule::Clip => Self::with_fill_and_filter_strategy::<Fill, ClipFilter>(column, buffer),
            OverlayRule::Intersect => Self::with_fill_and_filter_strategy::<Fill, IntersectFilter>(column, buffer),
            OverlayRule::Union => Self::with_fill_and_filter_strategy::<Fill, UnionFilter>(column, buffer),
            OverlayRule::Difference => Self::with_fill_and_filter_strategy::<Fill, DifferenceFilter>(column, buffer),
            OverlayRule::Xor => Self::with_fill_and_filter_strategy::<Fill, XorFilter>(column, buffer),
            OverlayRule::InverseDifference => Self::with_fill_and_filter_strategy::<Fill, InverseDifferenceFilter>(column, buffer),
        }
    }

    fn with_fill_and_filter_strategy<Fill, Filter>(column: &Column, buffer: &mut ScanBuffer) -> Self
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy,
    {
        let n = column.segments.len();
        buffer.clear();

        let mut nodes: Vec<Node> = Vec::with_capacity(n);

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

impl Node {
    #[inline(always)]
    fn new(point: IntPoint) -> Self {
        Self {
            point,
            links: [Link::empty(); 4],
        }
    }

    #[inline(always)]
    fn with_point_and_link(point: IntPoint, link: Link, index: LinkIndex) -> Self {
        let mut node = Self {
            point,
            links: [Link::empty(); 4],
        };
        node.set_link(link, index);
        node
    }

    #[inline(always)]
    fn with_link(link: Link, index: LinkIndex) -> Self {
        let mut node = Self {
            point: IntPoint::EMPTY,
            links: [Link::empty(); 4],
        };
        node.set_link(link, index);
        node
    }

    #[inline(always)]
    fn set_link(&mut self, link: Link, index: LinkIndex) {
        self.links[index as usize] = link;
    }

    #[inline(always)]
    fn link(&self, index: LinkIndex) -> Link {
        self.links[index as usize]
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Anchor {
    x: i32,
    node: Option<NonZeroU32>,
    count: ShapeCountBoolean,
}

impl Anchor {
    #[inline(always)]
    fn new(x: i32, node: Option<NonZeroU32>, count: ShapeCountBoolean) -> Self {
        Self { x, node, count }
    }
}

struct LeftCursor {
    node: u32,
    fill: SegmentFill,
}

struct ScanBuffer {
    active: Vec<Anchor>,
    buffer: Vec<Anchor>,
}

impl ScanBuffer {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            active: Vec::with_capacity(capacity),
            buffer: Vec::with_capacity(capacity),
        }
    }

    fn add_segments<Fill, Filter>(&mut self, segments: &[Segment], nodes: &mut Vec<Node>)
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy,
    {
        // segments are always sorted by range.min and not overlap each other
        // s(i).range.max <= s(i+1).range.min
        // segment.count.is_not_empty() === true
        // the segments not more < 100..200 elements
        // the buffers in average much less than 1000 and close to segments.len

        if self.active.is_empty() {
            self.active.init::<Fill, Filter>(segments, nodes);
            return;
        }

        let mut i = 0;
        let mut j = 0;

        let mut left_node = None;

        self.buffer.clear();

        while i < segments.len() && j < self.active.len() {
            if segments[i].range.max < self.active[j].x {
                // segments before

                let i0 = i;
                i += 1;
                while i < segments.len() && segments[i].range.max < self.active[j].x {
                    i += 1;
                }
                let base = self.active[j].count;
                self.buffer
                    .add_segments::<Fill, Filter>(base, &segments[i0..i], nodes);

                continue;
            } else if self.active[j].x < segments[i].range.min {
                // anchors before

                let j0 = j;
                j += 1;
                while j < self.active.len() && self.active[j].x < segments[i].range.min {
                    j += 1;
                }

                self.buffer.extend_from_slice(&self.active[j0..j]);

                continue;
            }

            // overlap

            let s = &segments[i];

            let j0 = j;
            let a0 = &self.active[j0];
            j += 1;
            while j < self.active.len() && self.active[j].x <= s.range.max {
                j += 1;
            }

            let start_count = if let Some(last) = self.buffer.last() && last.x == s.range.min {
                last.count
            } else {
                self.active[j0].count
            };

            let end_count = if j < self.active.len() {
                let ac = self.active[j].count;
                if let Some(next) = segments.get(i + 1)
                    && next.pos == s.pos
                    && next.range.min == s.range.max
                {
                    ac + next.count
                } else {
                    ac
                }
            } else {
                ShapeCountBoolean::empty()
            };

            self.buffer.add_anchors::<Fill, Filter>(
                s,
                &self.active[j0..j],
                start_count,
                end_count,
                &mut left_node,
                nodes,
            );

            i += 1;
        }

        if i < segments.len() {
            self.buffer.add_segments::<Fill, Filter>(
                ShapeCountBoolean::empty(),
                &segments[i..],
                nodes,
            );
        }

        if j < self.active.len() {
            self.buffer.extend_from_slice(&self.active[j..]);
        }

        self.buffer.remove_empty_anchor();
        swap(&mut self.active, &mut self.buffer);
    }

    #[inline]
    fn clear(&mut self) {
        self.active.clear();
        self.buffer.clear();
    }
}

fn for_each_anchor(
    s: &Segment,
    start_count: ShapeCountBoolean,
    end_count: ShapeCountBoolean,
    anchors: &[Anchor],
    mut f: impl FnMut(i32, Option<NonZeroU32>, ShapeCountBoolean, ShapeCountBoolean, ShapeCountBoolean),
) {
    debug_assert!(!anchors.is_empty());

    let mut a0 = &anchors[0];
    if a0.x > s.range.min {
        // min --- a0
        //  |
        // f(min)

        let c0 = start_count;
        let cb = a0.count;
        let c1 = cb + s.count;

        f(s.range.min, None, c0, c1, cb);
    };

    for ai in anchors.iter().skip(1) {
        // min --- a0 --- ai --- max
        //         |
        //       f(a0)

        let c0 = if a0.x == s.range.min {
            start_count
        } else {
            a0.count + s.count
        };
        let cb = ai.count;
        let c1 = cb + s.count;

        f(a0.x, a0.node, c0, c1, cb);
        a0 = ai;
    }

    if a0.x < s.range.max {
        // --- a0 --- max
        //     |       |
        //    f(a0)  f(max)
        {
            // a0
            let c0 = if a0.x == s.range.min {
                start_count
            } else {
                a0.count + s.count
            };
            let cb = end_count;
            let c1 = cb + s.count;

            f(a0.x, a0.node, c0, c1, cb);
        }

        {
            // max

            let c0 = end_count + s.count;
            let cb = end_count;
            let c1 = end_count;

            f(s.range.max, None, c0, c1, cb);
        }
    } else {
        // --- max(a0)
        //        |
        //       f(a0)

        let c0 = a0.count + s.count;
        let cb = end_count;
        let c1 = end_count;

        f(a0.x, a0.node, c0, c1, cb);
    }
}

fn for_each_boundaries(
    base: ShapeCountBoolean,
    segments: &[Segment],
    mut f: impl FnMut(i32, ShapeCountBoolean, ShapeCountBoolean),
) {
    debug_assert!(!segments.is_empty());

    let s0 = &segments[0];
    let mut x = s0.range.min;

    let mut c0 = base;
    let mut c1;
    {
        // s0.min
        c1 = s0.count + base;

        f(x, c0, c1);
        c0 = c1;
    }

    x = s0.range.max;

    for s in segments.iter().skip(1) {
        if x < s.range.min {
            // close last segment
            // s(prev).max

            c1 = base;

            f(x, c0, c1);
        }
        c0 = c1;

        // start new segment
        // s.min
        c1 = s.count + base;

        f(s.range.min, c0, c1);
        x = s.range.max;
        c0 = c1;
    }

    // close last segment
    f(x, c0, base);
}

trait AnchorBuffer {
    fn init<Fill, Filter>(&mut self, segments: &[Segment], nodes: &mut Vec<Node>)
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy;
    fn add_segments<Fill, Filter>(
        &mut self,
        base: ShapeCountBoolean,
        segments: &[Segment],
        nodes: &mut Vec<Node>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy;
    fn add_anchors<Fill, Filter>(
        &mut self,
        segment: &Segment,
        anchors: &[Anchor],
        start_count: ShapeCountBoolean,
        end_count: ShapeCountBoolean,
        left_node: &mut Option<LeftCursor>,
        nodes: &mut Vec<Node>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy;

    fn push_and_merge(&mut self, anchor: Anchor);

    fn remove_empty_anchor(&mut self);
}

impl AnchorBuffer for Vec<Anchor> {
    #[inline]
    fn init<Fill, Filter>(&mut self, segments: &[Segment], nodes: &mut Vec<Node>)
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy,
    {
        self.clear();
        let capacity = 2 * segments.len() + 1;
        self.reserve(capacity);

        self.add_segments::<Fill, Filter>(ShapeCountBoolean::empty(), segments, nodes);

        debug_assert!(self.len() <= capacity);
        debug_assert!(self.first().map_or(true, |first| first.count.is_empty()));
    }
    #[inline]
    fn add_segments<Fill, Filter>(
        &mut self,
        base: ShapeCountBoolean,
        segments: &[Segment],
        nodes: &mut Vec<Node>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy,
    {
        debug_assert!(!segments.is_empty());
        let y = segments.first().map_or(i32::MAX, |s| s.pos);

        let mut left_cursor: Option<LeftCursor> = None;

        for_each_boundaries(base, segments, |x, c0, c1| {
            let up_fill = Fill::fill(c0, c1);
            let up_link = Filter::is_included(up_fill);

            if up_link {
                let this = nodes.len() as u32;
                let up = this + 1;

                let mut node = Node::new(IntPoint::new(x, y));
                node.set_link(Link::new(up, up_fill), LinkIndex::Up);

                let mut top = Node::new(IntPoint::EMPTY);
                top.set_link(Link::new(this, up_fill), LinkIndex::Down);

                nodes.push(node);
                nodes.push(top);

                if let Some(cursor) = &left_cursor {
                    let left = cursor.node;
                    nodes[left as usize].set_link(Link::new(this, cursor.fill), LinkIndex::Right);
                    nodes[this as usize].set_link(Link::new(left, cursor.fill), LinkIndex::Left);
                    left_cursor = None;
                }

                let right_fill = Fill::fill(c1, base);
                let right_link = Filter::is_included(right_fill);

                if right_link {
                    left_cursor = Some(LeftCursor {
                        node: this,
                        fill: right_fill,
                    });
                };

                self.push_and_merge(Anchor::new(x, NonZeroU32::new(up), c0));
            } else {
                self.push_and_merge(Anchor::new(x, None, c0));
            }
        });

        debug_assert!(self.last().map_or(true, |last| last.count != base));
    }

    fn add_anchors<Fill, Filter>(
        &mut self,
        s: &Segment,
        anchors: &[Anchor],
        start_count: ShapeCountBoolean,
        end_count: ShapeCountBoolean,
        left_cursor: &mut Option<LeftCursor>,
        nodes: &mut Vec<Node>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy,
    {
        for_each_anchor(s, start_count, end_count, anchors, |x, index, c0, c1, cb| {
            let down_link = !index.is_none();

            let up_fill = Fill::fill(c0, c1);
            let up_link = Filter::is_included(up_fill);

            if down_link || up_link {
                let p = IntPoint::new(x, s.pos);

                let (this, up) = if let Some(node) = index {
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

                if let Some(cursor) = left_cursor {
                    let left = cursor.node;
                    nodes[left as usize].set_link(Link::new(this, cursor.fill), LinkIndex::Right);
                    nodes[this as usize].set_link(Link::new(left, cursor.fill), LinkIndex::Left);
                    *left_cursor = None;
                }

                let right_fill = Fill::fill(c1, cb);
                let right_link = Filter::is_included(right_fill);

                if right_link {
                    *left_cursor = Some(LeftCursor {
                        node: this,
                        fill: right_fill,
                    });
                };

                self.push_and_merge(Anchor::new(x, up, c0));
            } else {
                self.push_and_merge(Anchor::new(x, None, c0));
            }
        });
    }

    #[inline(always)]
    fn push_and_merge(&mut self, anchor: Anchor) {
        if let Some(last) = self.last_mut()
            && last.count == anchor.count
        {
            debug_assert!(last.x <= anchor.x);
            *last = anchor;
        } else {
            self.push(anchor);
        }
    }

    #[inline(always)]
    fn remove_empty_anchor(&mut self) {
        if self.len() == 1 && self[0].count.is_empty() {
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
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_overlay::vector::edge::Reverse;
    use i_shape::int::path::IntPath;
    use i_shape::int::shape::{IntContour, IntShape};
    use i_shape::int_shape;
    use rand::Rng;

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
    //
    // #[test]
    // fn test_random_0() {
    //     for _ in 0..10_000 {
    //         let contour = random_90_deg_contour(4, 4);
    //         test_contours(&vec![contour]);
    //     }
    // }
    //
    // #[test]
    // fn test_random_1() {
    //     for _ in 0..10_000 {
    //         let contour = random_90_deg_contour(5, 4);
    //         test_contours(&vec![contour]);
    //     }
    // }
    //
    // #[test]
    // fn test_random_2() {
    //     for _ in 0..10_000 {
    //         let contour = random_90_deg_contour(6, 4);
    //         test_contours(&vec![contour]);
    //     }
    // }

    fn test_contours(contours: &IntShape) {
        let config = ColumnConfig90 {
            min_columns_count: 1,
            min_column_width_power: 20,
            max_allow_segments_per_column: 1000_000_000,
            min_allowed_segments_per_column: 1_000_000,
            max_allowed_segments_per_line: 1000_000,
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

    fn contour_to_template_s_fills(contours: &[IntContour]) -> Option<Vec<SegFill>> {
        let mut overlay = i_overlay::core::overlay::Overlay::with_contours(&contours, &[]);
        let fill_rule = i_overlay::core::fill_rule::FillRule::NonZero;
        let overlay_rule = i_overlay::core::overlay_rule::OverlayRule::Subject;

        let graph = overlay.build_graph_view(fill_rule)?;
        let shapes = graph.extract_shape_vectors(overlay_rule);
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
        contours: &IntShape,
        config: ColumnConfig90,
        buffer: &mut ScanBuffer,
    ) -> Vec<SegFill> {
        let mut map = ColumnMap::with_subj_and_clip(contours, &[], CPUCount::Single, config);
        buffer.clear();
        debug_assert!(map.columns.len() == 1);
        let column = &mut map.columns[0];
        column.test_partition(config.clone());
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Subject;

        let graph = ColumnGraph::new(column, fill_rule, overlay_rule, buffer);
        let mut s_fills = graph.hz_edges();
        s_fills.sort_by_key(|s| s.a);

        s_fills
    }

    impl ColumnGraph {
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
