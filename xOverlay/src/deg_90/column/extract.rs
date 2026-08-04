use alloc::vec;
use crate::deg_90::column::graph::{ColumnGraph, Node};
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_shape::int::shape::{IntContour, IntShapes};
use crate::definition::segment::{CLIP_TOP, SUBJ_TOP};
use crate::deg_90::column::link::{Link, LinkIndex};

pub(crate) struct ExtResult {
    shapes: IntShapes<i32>,
    subpaths: Vec<SubPath>,
}

enum SubResult {
    Hull(IntContour<i32>),
    Hole(IntContour<i32>),
    Path(SubPath),
}

#[derive(Debug, Clone)]
pub(crate) struct SubPath {
    pub(crate) start: IntPoint,
    pub(crate) end: IntPoint,
    pub(crate) path: IntContour<i32>,
    pub(crate) holes: Vec<IntContour<i32>>,
}


#[derive(Debug, Clone, Default)]
pub(super) struct NodeVisitor {
    data: u8,
}

impl LinkIndex {
    #[inline(always)]
    fn bit(self) -> u8 {
        1 << self as u8
    }

    #[inline(always)]
    fn shift(&self, dir: bool) -> Self {
        let step = if dir { 1 } else { 3 };
        let order = self.order();
        let next = order + step;
        let module = next & 0x03;
        Self::with_order(module)
    }
}

impl NodeVisitor {
    #[inline(always)]
    pub(super) fn new(node: &Node) -> Self {
        let mut data = 0;
        for (order, link) in node.links.iter().enumerate() {
            let has_bit = link.is_not_empty() as u8;
            data |= has_bit << order;
        }

        Self { data }
    }
    #[inline(always)]
    pub(super) fn visit_if_not_yet(&mut self, order: usize) -> bool {
        let bit = 1 << order;
        let val = self.data & bit;
        self.data &= !val;
        val != 0
    }

    #[inline(always)]
    pub(super) fn is_visited(&self, order: usize) -> bool {
        self.data & (1 << order) == 0
    }

    #[inline(always)]
    fn visit_right_if_not_yet(&mut self) -> bool {
        let val = self.data & LinkIndex::Right.bit();
        self.data &= !val;
        val != 0
    }

    #[inline(always)]
    fn visit_and_next(&mut self, link: LinkIndex, dir: bool) -> Option<LinkIndex> {
        let val = self.data & link.bit();
        self.data &= !val;

        if self.data != 0 {
            let mut next = link;
            for _ in 0..4 {
                next = next.shift(dir);
                if self.visit_if_not_yet(next.order()) {
                    return Some(next)
                }
            }
            unreachable!("Must return link index!")
        }

        None
    }
}

impl ColumnGraph {
    #[inline(always)]
    pub(super) fn visitors(&self) -> Vec<NodeVisitor> {
        self.nodes.iter().map(|n| NodeVisitor::new(n)).collect()
    }

    pub(crate) fn extract(&self, visited: &mut Vec<NodeVisitor>, points: &mut Vec<IntPoint>) -> ExtResult {
        visited.resize(self.nodes.len(), Default::default());

        for (i, n) in self.nodes.iter().enumerate() {
            visited[i] = NodeVisitor::new(n)
        }

        let mut shapes = Vec::new();
        let mut holes = Vec::new();
        let mut subpaths = Vec::new();

        for i in 0..self.nodes.len() {
            if !visited[i].visit_right_if_not_yet() {
                continue;
            }

            let result = self.find_sub_result(i, true, visited, points);
            match result {
                SubResult::Hull(contour) => {
                    shapes.push(vec![contour]);
                }
                SubResult::Hole(contour) => {
                    holes.push(contour);
                }
                SubResult::Path(subpath) => {
                    subpaths.push(subpath);
                }
            }
        }

        Self::join_holes(holes, &mut shapes, &mut subpaths);

        ExtResult { shapes, subpaths }
    }

    fn find_sub_result(&self, start: usize, dir: bool, visited: &mut Vec<NodeVisitor>, points: &mut Vec<IntPoint>) -> SubResult {
        points.clear();

        let start_node = &self.nodes[start];
        points.push(start_node.point);
        let mut link_index = LinkIndex::Right;
        let start_link = start_node.links[link_index.order()];
        let mut next_node_index = start_link.index();
        while next_node_index != start {
            if let Some(next_link_index) = visited[next_node_index].visit_and_next(link_index.opposite(), dir) {
                let next_node = &self.nodes[next_node_index];
                points.add_skipping_vertical(next_node.point);

                let next_link = next_node.links[next_link_index.order()];
                next_node_index = next_link.index();
                link_index = next_link_index;
            }
        }

        points.remove_last_if_vertical();

        // a closed contour
        let contour = points.to_vec();
        if start_link.is_start_as_hull() {
            SubResult::Hull(contour)
        } else {
            SubResult::Hole(contour)
        }
    }
}
trait VerticalMiddleFilter {
    fn add_skipping_vertical(&mut self, point: IntPoint);
    fn remove_last_if_vertical(&mut self);
}

impl VerticalMiddleFilter for Vec<IntPoint> {
    #[inline(always)]
    fn add_skipping_vertical(&mut self, p: IntPoint) {
        let n = self.len();
        if n < 2 {
            self.push(p);
            return
        }
        let a = self[n - 2];
        let b = &mut self[n - 1];
        if a.x == b.x && a.x == p.x {
            b.y = p.y;
        } else {
            self.push(p);
        }
    }
    #[inline(always)]
    fn remove_last_if_vertical(&mut self) {
        let n = self.len();
        if n < 2 {
            return;
        }
        let a = self[0];
        let b = self[n - 1];
        let p = self[n - 2];
        if a.x == b.x && a.x == p.x {
            self.pop();
        }
    }
}

impl Link {
    #[inline(always)]
    fn is_start_as_hull(&self) -> bool {
        let fill = self.fill();
        fill == SUBJ_TOP || fill == CLIP_TOP
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use i_shape::int::shape::IntShape;
    use i_shape::int_shape;
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay_rule::OverlayRule;
    use crate::deg_90::column::build::ScanBuffer;
    use crate::deg_90::column::graph::ColumnGraph;
    use crate::deg_90::column::link::LinkIndex;
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;

    #[test]
    fn test_shift() {
        debug_assert_eq!(LinkIndex::Left.shift(true), LinkIndex::Up);
        debug_assert_eq!(LinkIndex::Up.shift(true), LinkIndex::Right);
        debug_assert_eq!(LinkIndex::Right.shift(true), LinkIndex::Down);
        debug_assert_eq!(LinkIndex::Down.shift(true), LinkIndex::Left);

        debug_assert_eq!(LinkIndex::Left.shift(false), LinkIndex::Down);
        debug_assert_eq!(LinkIndex::Up.shift(false), LinkIndex::Left);
        debug_assert_eq!(LinkIndex::Right.shift(false), LinkIndex::Up);
        debug_assert_eq!(LinkIndex::Down.shift(false), LinkIndex::Right);
    }

    #[test]
    fn test_square_1_column() {
        #[rustfmt::skip]
        let graph = first_column_graph_with_columns_count(&int_shape![[
            [-5, -5],
            [ 5, -5],
            [ 5,  5],
            [-5,  5],
        ]], 1);

        let mut visited = Vec::new();
        let mut points = Vec::new();
        let result = graph.extract(&mut visited, &mut points);

        debug_assert_eq!(result.shapes.len(), 1);
        debug_assert_eq!(result.shapes[0].len(), 1);
        debug_assert_eq!(result.shapes[0][0].len(), 4);
    }

    #[test]
    fn test_square_2_columns() {
        #[rustfmt::skip]
        let graph = first_column_graph_with_columns_count(&int_shape![[
            [-5, -5],
            [ 5, -5],
            [ 5,  5],
            [-5,  5],
        ]], 2);

        let mut visited = Vec::new();
        let mut points = Vec::new();
        let result = graph.extract(&mut visited, &mut points);

        debug_assert_eq!(result.shapes.len(), 1);
        debug_assert_eq!(result.shapes[0].len(), 1);
        debug_assert_eq!(result.shapes[0][0].len(), 4);
    }

    #[test]
    fn test_window_1_column() {
        #[rustfmt::skip]
        let graph = first_column_graph_with_columns_count(&int_shape![
            [
                [-5, -5],
                [ 5, -5],
                [ 5,  5],
                [-5,  5],
            ],
            [
                [-2, -2],
                [-2,  2],
                [ 2,  2],
                [ 2, -2],
            ]
    ], 1);

        let mut visited = Vec::new();
        let mut points = Vec::new();
        let result = graph.extract(&mut visited, &mut points);

        debug_assert_eq!(result.shapes.len(), 1);
        debug_assert_eq!(result.shapes[0].len(), 2);
        debug_assert_eq!(result.shapes[0][0].len(), 4);
        debug_assert_eq!(result.shapes[0][1].len(), 4);
    }

    fn first_column_graph_with_columns_count(contours: &IntShape<i32>, columns_count: usize) -> ColumnGraph {
        let config = ColumnConfig90 {
            min_columns_count: 1,
            min_column_width_power: 20,
            max_allow_segments_per_column: 1000_000_000,
            min_allowed_segments_per_column: 1_000_000,
            max_allowed_segments_per_line: 1000_000,
        };

        let mut buffer= ScanBuffer::with_capacity(16);

        let mut map = ColumnMap::with_columns_count(contours, &[], columns_count);
        debug_assert!(map.columns.len() >= 1);

        let column = &mut map.columns[0];
        column.test_partition(config);
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Subject;

        ColumnGraph::new(column, fill_rule, overlay_rule, &mut buffer)
    }
}
