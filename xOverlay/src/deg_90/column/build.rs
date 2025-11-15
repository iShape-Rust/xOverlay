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
use crate::definition::winding_count::ShapeCountBoolean;
use crate::deg_90::column::graph::{ColumnGraph, Node};
use crate::deg_90::column_map::Column;
use crate::gear::segment::Segment;
use alloc::vec::Vec;
use core::mem::swap;
use i_float::int::point::IntPoint;
use crate::definition::segment::NONE;
use crate::deg_90::column::link::{Link, LinkIndex};

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
struct CountAnchor {
    x: i32,
    count: ShapeCountBoolean,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Anchor {
    x: i32,
    node: u32,
    left_count: ShapeCountBoolean,
}

impl Anchor {
    #[inline(always)]
    fn add(&self, count: ShapeCountBoolean) -> Self {
        Self {
            x: self.x,
            node: 0,
            left_count: self.left_count + count,
        }
    }
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

        self.buffer.clear();

        while i < segments.len() && j < self.active.len() {
            if segments[i].range.max < self.active[j].x {
                // segments before

                let i0 = i;
                i += 1;
                while i < segments.len() && segments[i].range.max < self.active[j].x {
                    i += 1;
                }
                self.buffer
                    .add_segments::<Fill, Filter>(&segments[i0..i], nodes);

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
            j += 1;
            while j < self.active.len() && self.active[j].x <= s.range.max {
                j += 1;
            }

            self.buffer
                .add_anchors::<Fill, Filter>(s, &self.active[j0..j], nodes);

            i += 1;
        }

        if i < segments.len() {
            self.buffer
                .add_segments::<Fill, Filter>(&segments[i..], nodes);
        }

        if j < self.active.len() {
            self.buffer.extend_from_slice(&self.active[j..]);
        }

        swap(&mut self.active, &mut self.buffer);
    }

    #[inline]
    fn clear(&mut self) {
        self.active.clear();
        self.buffer.clear();
    }
}

trait AnchorBuffer {
    fn init<Fill, Filter>(&mut self, segments: &[Segment], nodes: &mut Vec<Node>)
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy;
    fn add_segments<Fill, Filter>(&mut self, segments: &[Segment], nodes: &mut Vec<Node>)
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy;
    fn add_anchors<Fill, Filter>(
        &mut self,
        segment: &Segment,
        anchors: &[Anchor],
        nodes: &mut Vec<Node>,
    ) where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy;
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

        self.add_segments::<Fill, Filter>(segments, nodes);

        debug_assert!(self.len() <= capacity);
        debug_assert!(
            self.first()
                .map_or(true, |first| first.left_count.is_empty())
        );
    }
    #[inline]
    fn add_segments<Fill, Filter>(&mut self, segments: &[Segment], nodes: &mut Vec<Node>)
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy,
    {
        debug_assert!(!segments.is_empty());
        debug_assert!(
            self.last()
                .map_or(true, |last| last.x < segments[0].range.min)
        );

        let mut cnt_buffer = Vec::new();

        let mut prev_cnt = ShapeCountBoolean::empty();
        let mut prev_node_x = i32::MAX;
        for s in segments {
            if prev_node_x != s.range.min {
                prev_cnt = ShapeCountBoolean::empty();
                cnt_buffer.push(CountAnchor {
                    x: s.range.min,
                    count: ShapeCountBoolean::empty(),
                });
            }
            let count = prev_cnt + s.count;
            cnt_buffer.push(CountAnchor {
                x: s.range.max,
                count,
            });
            debug_assert!(prev_cnt != count);
            prev_cnt = count;
            prev_node_x = s.range.max;
        }

        // empty end
        cnt_buffer.push(CountAnchor {
            x: i32::MIN,
            count: ShapeCountBoolean::empty(),
        });

        // first prev node is always empty
        let mut prev_node = u32::MAX;

        let mut cnt_iter = cnt_buffer.iter();
        let mut a0 = if let Some(first) = cnt_iter.next() {
            first
        } else {
            return;
        };

        let mut fill_0 = Fill::fill(a0.count, ShapeCountBoolean::empty());
        let mut incl_0 = Filter::is_included(fill_0);
        let y = segments.first().map_or(i32::MAX, |s| s.pos);

        for a1 in cnt_iter {
            let c0 = a0.count;
            let c1 = a1.count;

            let fill_1 = Fill::fill(c1, ShapeCountBoolean::empty());
            let incl_1 = Filter::is_included(fill_1);

            let fill_v = Fill::fill(c0, c1);
            let incl_v = Filter::is_included(fill_v);

            if incl_0 || incl_1 {
                let this = nodes.len();
                nodes.push(Node::new(IntPoint::new(a0.x, y)));

                if incl_0 {
                    nodes[this].set_link(Link::new(prev_node, fill_0), LinkIndex::Left);
                }

                if incl_v {
                    // create empty top node with down link to current
                    let top = nodes.len() as u32;
                    let top_node = Node::with_link(Link::new(this as u32, fill_v), LinkIndex::Down);

                    nodes.push(top_node);

                    nodes[this].set_link(Link::new(top, fill_v), LinkIndex::Up);
                }

                if incl_1 {
                    let next_node = nodes.len() as u32;
                    nodes[this].set_link(Link::new(next_node, fill_1), LinkIndex::Right);
                    prev_node = this as u32;
                }

                self.push(Anchor {
                    x: a0.x,
                    node: this as u32,
                    left_count: a0.count,
                });
            }

            fill_0 = fill_1;
            incl_0 = incl_1;
            a0 = a1;
        }

        debug_assert!(self.last().map_or(true, |last| !last.left_count.is_empty()));
    }

    fn add_anchors<Fill, Filter>(&mut self, s: &Segment, anchors: &[Anchor], nodes: &mut Vec<Node>)
    where
        Fill: FillStrategy<ShapeCountBoolean>,
        Filter: FilterStrategy,
    {
        debug_assert!(!anchors.is_empty());

        let mut a0 = &anchors[0];
        debug_assert!(s.range.min <= a0.x);
        debug_assert!(a0.x <= s.range.max);

        let mut i = 0;
        let mut c0 = ShapeCountBoolean::empty();
        let mut fill_0 = NONE;
        let mut incl_0 = false;
        let mut prev = usize::MAX;

        if s.range.min > a0.x {
            let c1 = s.count;

            let fill_1 = Fill::fill(c1, ShapeCountBoolean::empty());
            let incl_1 = Filter::is_included(fill_1);

            if incl_1 {
                let fill_v = Fill::fill(c0, c1);

                let this = nodes.len();
                nodes.push(Node::new(IntPoint::new(s.range.min, s.pos)));

                // create empty top node with down link to current
                let top = nodes.len() as u32;
                let top_node = Node::with_link(Link::new(this as u32, fill_v), LinkIndex::Down);

                nodes.push(top_node);

                nodes[this].set_link(Link::new(top, fill_v), LinkIndex::Up);

                let next_node = nodes.len() as u32;
                nodes[this].set_link(Link::new(next_node, fill_1), LinkIndex::Right);
                prev = this;
            }
            c0 = c1;
            fill_0 = fill_1;
            incl_0 = incl_1;
            i += 1;
            if i < anchors.len() {
                a0 = &anchors[i];
            }
        }

        while a0.x < s.range.max {
            // bottom count
            let cb = anchors.get(i + 1).map_or(ShapeCountBoolean::empty(), |a| a.left_count);
            let c1 = cb + s.count;

            let fill_1 = Fill::fill(c1, cb);
            let incl_1 = Filter::is_included(fill_1);

            let fill_v = Fill::fill(c0, c1);
            let incl_v = Filter::is_included(fill_v);

            if !(incl_0 || incl_1 || incl_v) {
                i += 1;
                if i < anchors.len() {
                    a0 = &anchors[i];
                    continue;
                } else {
                    break;
                }
            }

            let bottom_node = &nodes[a0.node as usize];
            let this = bottom_node.link(LinkIndex::Up).index();

            nodes[this].point = IntPoint::new(a0.x, s.pos);

            if incl_0 {
                nodes[this].set_link(Link::new(prev as u32, fill_0), LinkIndex::Left);
                nodes[prev].set_link(Link::new(this as u32, fill_0), LinkIndex::Right);
            }

            if incl_v {
                // create empty top node with down link to current
                let top = nodes.len() as u32;
                let top_node = Node::with_link(Link::new(this as u32, fill_v), LinkIndex::Down);
                nodes.push(top_node);

                nodes[this].set_link(Link::new(top, fill_v), LinkIndex::Up);
            }

            prev = this;

            c0 = c1;
            fill_0 = fill_1;
            incl_0 = incl_1;
            i += 1;
            if i < anchors.len() {
                a0 = &anchors[i];
            } else {
                break;
            }
        }

        if a0.x == s.range.max {
            let fill_v = Fill::fill(c0, ShapeCountBoolean::empty());
            let incl_v = Filter::is_included(fill_v);

            let bottom_node = &nodes[a0.node as usize];
            let this = bottom_node.link(LinkIndex::Up).index();

            nodes[this].point = IntPoint::new(a0.x, s.pos);

            if incl_0 {
                nodes[this].set_link(Link::new(prev as u32, fill_0), LinkIndex::Left);
                nodes[prev].set_link(Link::new(this as u32, fill_0), LinkIndex::Right);
            }

            if incl_v {
                // create empty top node with down link to current
                let top = nodes.len() as u32;
                let top_node = Node::with_link(Link::new(this as u32, fill_v), LinkIndex::Down);
                nodes.push(top_node);

                nodes[this].set_link(Link::new(top, fill_v), LinkIndex::Up);
            }
        } else {
            let fill_v = Fill::fill(c0, ShapeCountBoolean::empty());
            let incl_v = Filter::is_included(fill_v);

            if incl_0 || incl_v {
                let this = nodes.len();
                nodes.push(Node::new(IntPoint::new(s.range.max, s.pos)));

                if incl_0 {
                    nodes[this].set_link(Link::new(prev as u32, fill_0), LinkIndex::Left);
                    nodes[prev].set_link(Link::new(this as u32, fill_0), LinkIndex::Right);
                }

                if incl_v {
                    // create empty top node with down link to current
                    let top = nodes.len() as u32;
                    let top_node = Node::with_link(Link::new(this as u32, fill_v), LinkIndex::Down);
                    nodes.push(top_node);

                    nodes[this].set_link(Link::new(top, fill_v), LinkIndex::Up);
                }
            }
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
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_overlay::vector::edge::Reverse;
    use i_shape::int::path::IntPath;
    use i_shape::int::shape::{IntContour, IntShape};
    use i_shape::int_shape;
    use rand::Rng;
    use crate::deg_90::column::graph::ColumnGraph;
    use crate::deg_90::column::link::LinkIndex;

    #[test]
    fn test_0() {
        #[rustfmt::skip]
        test_contours(&int_shape![[
            [0, 0],
            [2, 0],
            [2, 2],
            [0, 2],
        ],]);
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
    fn test_3() {
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
    fn test_random_0() {
        for _ in 0..1000 {
            let contour = random_90_deg_contour(4, 4);
            test_contours(&vec![contour]);
        }
    }

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
        fn with_abf(a: IntPoint, b: IntPoint, f: SegmentFill) -> Self {
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
                    s_fills.push(SegFill::with_abf(e.a, e.b, e.fill))
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
        graph.edges()
    }

    impl ColumnGraph {
        fn edges(&self) -> Vec<SegFill> {
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

                    edges.push(SegFill::with_abf(a, b, link.fill()));
                }
            }
            edges
        }
    }
}
