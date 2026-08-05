use crate::deg_90::column::extract::SubPath;
use crate::deg_90::column::graph::ColumnGraph;
use crate::geom::range::LineRange;
use alloc::vec;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::sort::one_key::OneKeySort;
use i_shape::int::area::Area;
use i_shape::int::path::ContourExtension;
use i_shape::int::shape::{IntContour, IntShapes};

const SWEEP_MIN_ANCHORS_PER_Y: usize = 256;

impl ColumnGraph {
    pub(super) fn join_holes(
        holes: Vec<IntContour<i32>>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        if holes.is_empty() {
            return;
        }

        let mut active_segments_count: usize = 0;

        for shape in shapes.iter() {
            active_segments_count += shape[0].len() / 4;
        }

        for sub_path in sub_paths.iter() {
            active_segments_count += sub_path.path.len() / 4;
        }

        let log2 = (active_segments_count.ilog2() as usize).max(10);

        if holes.len() <= log2 {
            Self::direct_join_holes(holes, shapes, sub_paths)
        } else {
            let anchors_by_y = hole_anchors_by_y(&holes);
            if prefer_sweep(&anchors_by_y) {
                Self::sweep_join_holes_with_anchors(
                    active_segments_count,
                    holes,
                    anchors_by_y,
                    shapes,
                    sub_paths,
                )
            } else {
                Self::sort_join_holes_with_anchors(
                    active_segments_count,
                    holes,
                    anchors_by_y,
                    shapes,
                    sub_paths,
                )
            }
        }
    }

    fn direct_join_holes(
        holes: Vec<IntContour<i32>>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        for hole in holes.into_iter() {
            let p = hole.bottom_anchor();

            let mut best = i32::MIN;
            let mut best_shape_index = usize::MAX;
            let mut best_sub_path_index = usize::MAX;

            for (index, shape) in shapes.iter_mut().enumerate() {
                if shape[0].contains_point(p) && shape[0].best_edge(p, &mut best) {
                    best_shape_index = index;
                }
            }

            for (index, sub_path) in sub_paths.iter_mut().enumerate() {
                if sub_path.path.best_edge(p, &mut best) {
                    best_sub_path_index = index;
                }
            }

            if best_sub_path_index < sub_paths.len() {
                sub_paths[best_sub_path_index].holes.push(hole);
            } else {
                shapes[best_shape_index].push(hole);
            }
        }
    }

    #[cfg(test)]
    fn sweep_join_holes(
        capacity: usize,
        holes: Vec<IntContour<i32>>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        let anchors_by_y = hole_anchors_by_y(&holes);
        Self::sweep_join_holes_with_anchors(capacity, holes, anchors_by_y, shapes, sub_paths);
    }

    fn sweep_join_holes_with_anchors(
        capacity: usize,
        holes: Vec<IntContour<i32>>,
        anchors_by_y: Vec<HoleAnchor>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        let mut anchors_by_x = anchors_by_y.clone();
        anchors_by_x.sort_by(|a, b| a.point.cmp(&b.point));

        let mut positions = vec![0; holes.len()];
        for (position, anchor) in anchors_by_x.iter().enumerate() {
            positions[anchor.hole_index] = position;
        }

        let holes_capacity = holes.iter().map(|hole| hole.len() / 4).sum::<usize>();
        let mut edges = Vec::with_capacity(capacity + holes_capacity);

        for (index, shape) in shapes.iter().enumerate() {
            append_sweep_edges(
                &shape[0],
                SweepTarget::Parent(HoleParent {
                    index,
                    is_shape: true,
                }),
                &mut edges,
            );
        }

        for (index, sub_path) in sub_paths.iter().enumerate() {
            append_sweep_edges(
                &sub_path.path,
                SweepTarget::Parent(HoleParent {
                    index,
                    is_shape: false,
                }),
                &mut edges,
            );
        }

        for (index, hole) in holes.iter().enumerate() {
            append_sweep_edges(hole, SweepTarget::Hole(index), &mut edges);
        }

        edges.sort_by_one_key(false, |edge| edge.y);

        let mut active = ActiveAnchors::new(holes.len());
        let mut targets = vec![None; holes.len()];
        let mut edge_end = edges.len();
        let mut anchor_end = anchors_by_y.len();

        while edge_end > 0 || anchor_end > 0 {
            let edge_y = if edge_end > 0 {
                edges[edge_end - 1].y
            } else {
                i32::MIN
            };
            let anchor_y = if anchor_end > 0 {
                anchors_by_y[anchor_end - 1].point.y
            } else {
                i32::MIN
            };
            let y = edge_y.max(anchor_y);

            // Edges at the same y are processed before anchors. This preserves
            // the strict "edge below anchor" rule used by the direct solver.
            while edge_end > 0 && edges[edge_end - 1].y == y {
                edge_end -= 1;
                let edge = edges[edge_end];
                let start = anchors_by_x.partition_point(|a| a.point.x < edge.x_min);
                let end = anchors_by_x.partition_point(|a| a.point.x < edge.x_max);

                while let Some(position) = active.first_in(start, end) {
                    active.remove(position);
                    let hole_index = anchors_by_x[position].hole_index;
                    targets[hole_index] = Some(edge.target);
                }
            }

            while anchor_end > 0 && anchors_by_y[anchor_end - 1].point.y == y {
                anchor_end -= 1;
                let hole_index = anchors_by_y[anchor_end].hole_index;
                active.insert(positions[hole_index]);
            }
        }

        debug_assert_eq!(active.count(), 0, "some hole anchors have no parent edge");

        let mut parents = vec![None; holes.len()];
        for anchor in anchors_by_y {
            let parent = match targets[anchor.hole_index].expect("hole target must be resolved") {
                SweepTarget::Parent(parent) => parent,
                SweepTarget::Hole(target_hole_index) => {
                    parents[target_hole_index].expect("target hole parent must already be resolved")
                }
            };
            parents[anchor.hole_index] = Some(parent);
        }

        for (hole, parent) in holes.into_iter().zip(parents) {
            let parent = parent.expect("hole parent must be resolved");
            if parent.is_shape {
                shapes[parent.index].push(hole);
            } else {
                sub_paths[parent.index].holes.push(hole);
            }
        }
    }

    #[cfg(test)]
    fn sort_join_holes(
        capacity: usize,
        holes: Vec<IntContour<i32>>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        let anchors_by_y = hole_anchors_by_y(&holes);
        Self::sort_join_holes_with_anchors(capacity, holes, anchors_by_y, shapes, sub_paths);
    }

    fn sort_join_holes_with_anchors(
        capacity: usize,
        holes: Vec<IntContour<i32>>,
        anchors_by_y: Vec<HoleAnchor>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        let holes_capacity = holes.iter().map(|hole| hole.len() / 4).sum::<usize>();
        let mut edges = Vec::with_capacity(capacity + holes_capacity);
        for (index, shape) in shapes.iter().enumerate() {
            let main = &shape[0];
            let mut a = main[main.len() - 1];
            for &b in main.iter() {
                if a.x < b.x {
                    edges.push(ShapeEdge {
                        y: a.y,
                        line_range: LineRange::with_min_max(a.x, b.x),
                        index: index as u32,
                        is_shape: true,
                    });
                }
                a = b;
            }
        }

        for (index, sub_path) in sub_paths.iter().enumerate() {
            let path = &sub_path.path;
            let mut a = path[path.len() - 1];
            for &b in path.iter() {
                if a.x < b.x {
                    edges.push(ShapeEdge {
                        y: a.y,
                        line_range: LineRange::with_min_max(a.x, b.x),
                        index: index as u32,
                        is_shape: false,
                    });
                }
                a = b;
            }
        }

        let hole_index_offset = sub_paths.len();
        for (index, hole) in holes.iter().enumerate() {
            let mut a = hole[hole.len() - 1];
            for &b in hole.iter() {
                // Holes are clockwise, so left-to-right segments are their
                // top edges: the outer boundary seen by a point above them.
                if a.x < b.x {
                    edges.push(ShapeEdge {
                        y: a.y,
                        line_range: LineRange::with_min_max(a.x, b.x),
                        index: (hole_index_offset + index) as u32,
                        is_shape: false,
                    });
                }
                a = b;
            }
        }

        edges.sort_by_one_key(false, |e| e.y);

        let mut parents = vec![None; holes.len()];
        for anchor in anchors_by_y {
            let e = edges.first_under(anchor.point);
            let index = e.index as usize;
            let parent = if e.is_shape {
                HoleParent {
                    index,
                    is_shape: true,
                }
            } else if index < hole_index_offset {
                HoleParent {
                    index,
                    is_shape: false,
                }
            } else {
                let target_hole_index = index - hole_index_offset;
                parents[target_hole_index].expect("target hole parent must already be resolved")
            };
            parents[anchor.hole_index] = Some(parent);
        }

        for (hole, parent) in holes.into_iter().zip(parents) {
            let parent = parent.expect("hole parent must be resolved");
            if parent.is_shape {
                shapes[parent.index].push(hole);
            } else {
                sub_paths[parent.index].holes.push(hole);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct HoleAnchor {
    point: IntPoint,
    hole_index: usize,
}

fn hole_anchors_by_y(holes: &[IntContour<i32>]) -> Vec<HoleAnchor> {
    let mut anchors: Vec<_> = holes
        .iter()
        .enumerate()
        .map(|(hole_index, hole)| HoleAnchor {
            point: hole.bottom_anchor(),
            hole_index,
        })
        .collect();
    anchors.sort_by_one_key(false, |anchor| anchor.point.y);
    anchors
}

fn prefer_sweep(anchors_by_y: &[HoleAnchor]) -> bool {
    let mut max_count = 0;
    let mut start = 0;

    while start < anchors_by_y.len() {
        let y = anchors_by_y[start].point.y;
        let mut end = start + 1;
        while end < anchors_by_y.len() && anchors_by_y[end].point.y == y {
            end += 1;
        }

        max_count = max_count.max(end - start);
        start = end;
    }

    max_count >= SWEEP_MIN_ANCHORS_PER_Y
}

#[derive(Debug, Clone, Copy)]
enum SweepTarget {
    Parent(HoleParent),
    Hole(usize),
}

#[derive(Debug, Clone, Copy)]
struct SweepEdge {
    y: i32,
    x_min: i32,
    x_max: i32,
    target: SweepTarget,
}

fn append_sweep_edges(path: &IntContour<i32>, target: SweepTarget, edges: &mut Vec<SweepEdge>) {
    let mut a = path[path.len() - 1];
    for &b in path {
        if a.x < b.x {
            edges.push(SweepEdge {
                y: a.y,
                x_min: a.x,
                x_max: b.x,
                target,
            });
        }
        a = b;
    }
}

// A flat Fenwick tree over hole anchors sorted by x. It contains only anchors
// that the descending sweep has activated but not resolved yet.
struct ActiveAnchors {
    tree: Vec<usize>,
    search_bit: usize,
}

impl ActiveAnchors {
    fn new(count: usize) -> Self {
        let mut search_bit = 1;
        while search_bit << 1 <= count {
            search_bit <<= 1;
        }

        Self {
            tree: vec![0; count + 1],
            search_bit,
        }
    }

    #[inline]
    fn insert(&mut self, index: usize) {
        let mut i = index + 1;
        while i < self.tree.len() {
            self.tree[i] += 1;
            i += i & i.wrapping_neg();
        }
    }

    #[inline]
    fn remove(&mut self, index: usize) {
        let mut i = index + 1;
        while i < self.tree.len() {
            self.tree[i] -= 1;
            i += i & i.wrapping_neg();
        }
    }

    #[inline]
    fn count(&self) -> usize {
        self.prefix_count(self.tree.len() - 1)
    }

    #[inline]
    fn first_in(&self, start: usize, end: usize) -> Option<usize> {
        if start >= end {
            return None;
        }

        let before = self.prefix_count(start);
        if self.prefix_count(end) == before {
            None
        } else {
            Some(self.index_for_order(before + 1))
        }
    }

    #[inline]
    fn prefix_count(&self, end: usize) -> usize {
        let mut result = 0;
        let mut i = end;
        while i > 0 {
            result += self.tree[i];
            i &= i - 1;
        }
        result
    }

    #[inline]
    fn index_for_order(&self, mut order: usize) -> usize {
        let mut index = 0;
        let mut bit = self.search_bit;

        while bit > 0 {
            let next = index + bit;
            if next < self.tree.len() && self.tree[next] < order {
                index = next;
                order -= self.tree[next];
            }
            bit >>= 1;
        }

        index
    }
}

#[derive(Debug, Clone, Copy)]
struct HoleParent {
    index: usize,
    is_shape: bool,
}

trait BottomAnchor {
    fn bottom_anchor(&self) -> IntPoint;
}

impl BottomAnchor for IntContour<i32> {
    #[inline]
    fn bottom_anchor(&self) -> IntPoint {
        let mut anchor = self[0];
        for &p in self.iter().skip(1) {
            if p.y < anchor.y {
                anchor = p;
            }
        }
        anchor
    }
}

#[derive(Debug, Clone, Copy)]
struct ShapeEdge {
    y: i32,
    line_range: LineRange,
    index: u32,
    is_shape: bool,
}

trait BestEdge {
    fn best_edge(&self, p: IntPoint, best: &mut i32) -> bool;
}

impl BestEdge for IntContour<i32> {
    fn best_edge(&self, p: IntPoint, best: &mut i32) -> bool {
        debug_assert!(self.area_two() > 0);
        let mut result = false;
        let mut a = *self.last().unwrap();
        for &b in self.iter() {
            let better = *best < a.y;
            let bottom_edge = a.x < b.x;
            let under_point = a.y < p.y;
            let contains_by_x = a.x <= p.x && p.x < b.x;

            if better && bottom_edge && under_point && contains_by_x {
                *best = a.y;
                result = true;
            }
            a = b;
        }
        result
    }
}

trait FirstBottom {
    fn first_under(&self, p: IntPoint) -> &ShapeEdge;
}

impl FirstBottom for [ShapeEdge] {
    #[inline]
    fn first_under(&self, p: IntPoint) -> &ShapeEdge {
        // edges must be sorted by e.y

        let start = match self.binary_search_by_key(&p.y, |e| e.y) {
            Ok(mut index) => {
                while index + 1 < self.len() && self[index + 1].y == p.y {
                    index += 1;
                }
                index
            }
            Err(index) => {
                debug_assert!(index > 0);
                index - 1
            }
        };

        for e in self[..=start].iter().rev() {
            if e.y <= p.y && p.x >= e.line_range.min && p.x < e.line_range.max {
                return e;
            }
        }

        debug_assert!(false, "edge under point not found");
        &self[start]
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::deg_90::column::extract::SubPath;
    use crate::deg_90::column::graph::ColumnGraph;
    use crate::deg_90::column::join_holes::{
        hole_anchors_by_y, prefer_sweep, ActiveAnchors, BottomAnchor, FirstBottom, ShapeEdge,
    };
    use crate::geom::range::LineRange;
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_key_sort::sort::one_key::OneKeySort;
    use i_shape::int::path::IntPath;
    use i_shape::int::shape::{IntShape, IntShapes};
    use i_shape::{int_path, int_shape, int_shapes};
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    fn sub_path(path: IntPath<i32>) -> SubPath {
        SubPath {
            start: Default::default(),
            end: Default::default(),
            path,
            holes: vec![],
        }
    }

    #[test]
    fn test_bottom_anchor_is_not_required_to_be_first() {
        let hole = int_path![[5, 8], [9, 8], [9, 1], [1, 1], [1, 8]];

        assert_ne!(hole[0].y, 1);
        assert_eq!(hole.bottom_anchor().y, 1);
    }

    #[test]
    fn test_active_anchors() {
        let mut active = ActiveAnchors::new(8);
        active.insert(1);
        active.insert(3);
        active.insert(6);

        assert_eq!(active.count(), 3);
        assert_eq!(active.first_in(0, 1), None);
        assert_eq!(active.first_in(0, 8), Some(1));
        assert_eq!(active.first_in(2, 6), Some(3));
        assert_eq!(active.first_in(4, 8), Some(6));

        active.remove(3);
        assert_eq!(active.first_in(2, 6), None);
        assert_eq!(active.count(), 2);
    }

    #[test]
    fn test_edges_search_0() {
        let mut edges = vec![
            ShapeEdge {
                y: 0,
                line_range: LineRange::with_min_max(0, 10),
                index: 0,
                is_shape: false,
            },
            ShapeEdge {
                y: 10,
                line_range: LineRange::with_min_max(0, 10),
                index: 1,
                is_shape: false,
            },
            ShapeEdge {
                y: -10,
                line_range: LineRange::with_min_max(0, 10),
                index: 2,
                is_shape: false,
            },
            ShapeEdge {
                y: 20,
                line_range: LineRange::with_min_max(0, 10),
                index: 3,
                is_shape: false,
            },
            ShapeEdge {
                y: -20,
                line_range: LineRange::with_min_max(0, 10),
                index: 4,
                is_shape: false,
            },
            ShapeEdge {
                y: -30,
                line_range: LineRange::with_min_max(0, 10),
                index: 5,
                is_shape: false,
            },
        ];

        edges.sort_by_one_key(false, |e| e.y);

        let e0 = edges.first_under((5, 0).into());
        let e1 = edges.first_under((5, 1).into());
        let e2 = edges.first_under((5, -1).into());

        debug_assert_eq!(e0.index, 0);
        debug_assert_eq!(e1.index, 0);
        debug_assert_eq!(e2.index, 2);
    }

    #[test]
    fn test_edges_search_1() {
        let mut edges = vec![
            ShapeEdge {
                y: 0,
                line_range: LineRange::with_min_max(0, 3),
                index: 0,
                is_shape: false,
            },
            ShapeEdge {
                y: 0,
                line_range: LineRange::with_min_max(3, 7),
                index: 1,
                is_shape: false,
            },
            ShapeEdge {
                y: 0,
                line_range: LineRange::with_min_max(7, 10),
                index: 2,
                is_shape: false,
            },
            ShapeEdge {
                y: 10,
                line_range: LineRange::with_min_max(5, 10),
                index: 3,
                is_shape: false,
            },
            ShapeEdge {
                y: -10,
                line_range: LineRange::with_min_max(0, 5),
                index: 4,
                is_shape: false,
            },
            ShapeEdge {
                y: 20,
                line_range: LineRange::with_min_max(0, 5),
                index: 5,
                is_shape: false,
            },
            ShapeEdge {
                y: -20,
                line_range: LineRange::with_min_max(0, 5),
                index: 6,
                is_shape: false,
            },
            ShapeEdge {
                y: -30,
                line_range: LineRange::with_min_max(5, 10),
                index: 7,
                is_shape: false,
            },
        ];

        edges.sort_by_one_key(false, |e| e.y);

        debug_assert_eq!(edges.first_under((1, 2).into()).index, 0);
        debug_assert_eq!(edges.first_under((3, 2).into()).index, 1);
        debug_assert_eq!(edges.first_under((6, 2).into()).index, 1);
        debug_assert_eq!(edges.first_under((7, 2).into()).index, 2);
        debug_assert_eq!(edges.first_under((7, 0).into()).index, 2);
        debug_assert_eq!(edges.first_under((4, 11).into()).index, 1);
        debug_assert_eq!(edges.first_under((7, 10).into()).index, 3);
        debug_assert_eq!(edges.first_under((7, 11).into()).index, 3);
        debug_assert_eq!(edges.first_under((4, 20).into()).index, 5);
        debug_assert_eq!(edges.first_under((4, -1).into()).index, 4);
        debug_assert_eq!(edges.first_under((5, -1).into()).index, 7);
        debug_assert_eq!(edges.first_under((0, -11).into()).index, 6);
        debug_assert_eq!(edges.first_under((0, -20).into()).index, 6);
        debug_assert_eq!(edges.first_under((5, -21).into()).index, 7);
    }

    #[test]
    fn test_join_holes_0() {
        let shapes = int_shapes![
            [[[0, 0], [10, 0], [10, 10], [0, 10]],],
            [[[0, 10], [10, 10], [10, 20], [0, 20]],],
            [[[0, -10], [10, -10], [10, 0], [0, 0]],],
        ];

        let holes = int_shape![[[2, 2], [2, 4], [4, 4], [4, 2]],];

        let mut sub_paths = vec![];

        let mut shapes_0 = shapes.clone();
        let mut shapes_1 = shapes.clone();
        let mut shapes_2 = shapes.clone();

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes_0, &mut sub_paths);
        ColumnGraph::sort_join_holes(10, holes.clone(), &mut shapes_1, &mut sub_paths);
        ColumnGraph::sweep_join_holes(10, holes, &mut shapes_2, &mut sub_paths);

        assert_eq!(shapes_0, shapes_1);
        assert_eq!(shapes_0, shapes_2);
        assert_eq!(shapes_0[0].len(), 2);
        assert_eq!(shapes_0[1].len(), 1);
        assert_eq!(shapes_0[2].len(), 1);
    }

    #[test]
    fn test_join_holes_1() {
        let shapes = int_shapes![
            [[[0, 0], [10, 0], [10, 10], [0, 10]],],
            [[[0, 10], [10, 10], [10, 20], [0, 20]],],
            [[[0, -10], [10, -10], [10, 0], [0, 0]],],
        ];

        let holes = int_shape![
            [[2, 2], [2, 4], [4, 4], [4, 2]],
            [[6, 2], [6, 4], [8, 4], [8, 2]],
            [[2, 6], [2, 8], [4, 8], [4, 6]],
            [[6, 6], [6, 8], [8, 8], [8, 6]],
        ];

        let mut sub_paths = vec![];

        let mut shapes_0 = shapes.clone();
        let mut shapes_1 = shapes.clone();
        let mut shapes_2 = shapes.clone();

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes_0, &mut sub_paths);
        ColumnGraph::sort_join_holes(10, holes.clone(), &mut shapes_1, &mut sub_paths);
        ColumnGraph::sweep_join_holes(10, holes, &mut shapes_2, &mut sub_paths);

        assert_eq!(shapes_0, shapes_1);
        assert_eq!(shapes_0, shapes_2);
        assert_eq!(shapes_0[0].len(), 5);
        assert_eq!(shapes_0[1].len(), 1);
        assert_eq!(shapes_0[2].len(), 1);
    }

    #[test]
    fn test_join_holes_2() {
        let shapes = int_shapes![
            [[[0, 0], [10, 0], [10, 10], [0, 10]],],
            [[[0, 10], [10, 10], [10, 20], [0, 20]],],
            [[[0, -10], [10, -10], [10, 0], [0, 0]],],
        ];

        let holes = int_shape![
            [[2, 2], [2, 4], [4, 4], [4, 2]],
            [[6, 2], [6, 4], [8, 4], [8, 2]],
            [[2, 6], [2, 8], [4, 8], [4, 6]],
            [[6, 6], [6, 8], [8, 8], [8, 6]],
            [[2, 12], [2, 14], [4, 14], [4, 12]],
            [[2, -8], [2, -6], [4, -6], [4, -8]],
        ];

        let mut sub_paths = vec![];

        let mut shapes_0 = shapes.clone();
        let mut shapes_1 = shapes.clone();
        let mut shapes_2 = shapes.clone();

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes_0, &mut sub_paths);
        ColumnGraph::sort_join_holes(10, holes.clone(), &mut shapes_1, &mut sub_paths);
        ColumnGraph::sweep_join_holes(10, holes, &mut shapes_2, &mut sub_paths);

        assert_eq!(shapes_0, shapes_1);
        assert_eq!(shapes_0, shapes_2);
        assert_eq!(shapes_0[0].len(), 5);
        assert_eq!(shapes_0[1].len(), 2);
        assert_eq!(shapes_0[2].len(), 2);
    }

    #[test]
    fn test_hole_is_joined_to_outer_hull_not_inner_island() {
        let shapes = int_shapes![
            [[[0, 0], [10, 0], [10, 10], [0, 10]],],
            [[[4, 2], [6, 2], [6, 4], [4, 4]],],
        ];
        // The first point is above the island and shares its x-range. The old
        // code used it as the anchor and therefore selected the island.
        // Any point with minimum y (here y = 1) identifies the outer hull.
        let holes = int_shape![[[5, 8], [9, 8], [9, 1], [1, 1], [1, 8]],];
        let expected_hole = holes[0].clone();
        let mut sub_paths = vec![];
        let mut direct_shapes = shapes.clone();
        let mut sorted_shapes = shapes.clone();
        let mut sweep_shapes = shapes;

        ColumnGraph::direct_join_holes(holes.clone(), &mut direct_shapes, &mut sub_paths);
        ColumnGraph::sort_join_holes(2, holes.clone(), &mut sorted_shapes, &mut sub_paths);
        ColumnGraph::sweep_join_holes(2, holes, &mut sweep_shapes, &mut sub_paths);

        assert_eq!(direct_shapes, sorted_shapes);
        assert_eq!(direct_shapes, sweep_shapes);
        assert_eq!(direct_shapes[0].len(), 2);
        assert_eq!(direct_shapes[1].len(), 1);
        assert_eq!(direct_shapes[0][1], expected_hole);
    }

    #[test]
    fn test_bottom_anchor_skips_island_from_another_hole() {
        let shapes = int_shapes![
            [[[0, 0], [10, 0], [10, 10], [0, 10]],],
            [[[5, 2], [7, 2], [7, 4], [5, 4]],],
        ];
        let holes = int_shape![
            [[4, 9], [6, 9], [6, 6], [4, 6]],
            [[3, 5], [8, 5], [8, 1], [3, 1]],
        ];
        let mut direct_shapes = shapes.clone();
        let mut sorted_shapes = shapes.clone();
        let mut sweep_shapes = shapes;

        ColumnGraph::direct_join_holes(holes.clone(), &mut direct_shapes, &mut vec![]);
        ColumnGraph::sort_join_holes(2, holes.clone(), &mut sorted_shapes, &mut vec![]);
        ColumnGraph::sweep_join_holes(2, holes, &mut sweep_shapes, &mut vec![]);

        assert_eq!(direct_shapes, sorted_shapes);
        assert_eq!(direct_shapes, sweep_shapes);
        assert_eq!(direct_shapes[0].len(), 3);
        assert_eq!(direct_shapes[1].len(), 1);
    }

    #[test]
    #[ignore = "performance comparison; run explicitly with --release --ignored --nocapture"]
    fn performance_join_holes_scenarios() {
        run_performance_scenario("wide", ambiguous_hole_groups_wide);
        run_performance_scenario("vertical", ambiguous_hole_groups_vertical);
        run_performance_scenario("grid", ambiguous_hole_groups_grid);
        run_performance_scenario("chain", hole_chain);
    }

    #[test]
    fn test_adaptive_join_strategy_scenarios() {
        let (_, wide_below_threshold) = ambiguous_hole_groups_wide(255);
        let (_, wide) = ambiguous_hole_groups_wide(256);
        let (_, vertical) = ambiguous_hole_groups_vertical(4_096);
        let (_, grid) = ambiguous_hole_groups_grid(4_096);
        let (_, chain) = hole_chain(4_096);

        assert!(!prefer_sweep(&hole_anchors_by_y(&wide_below_threshold)));
        assert!(prefer_sweep(&hole_anchors_by_y(&wide)));
        assert!(!prefer_sweep(&hole_anchors_by_y(&vertical)));
        assert!(!prefer_sweep(&hole_anchors_by_y(&grid)));
        assert!(!prefer_sweep(&hole_anchors_by_y(&chain)));
    }

    fn run_performance_scenario(name: &str, build: fn(usize) -> (IntShapes<i32>, IntShape<i32>)) {
        let (_, largest_holes) = build(4_096);
        let selected = if prefer_sweep(&hole_anchors_by_y(&largest_holes)) {
            "sweep"
        } else {
            "sort"
        };
        std::println!("scenario={name}, adaptive={selected}");

        for size in [256, 512, 1_024, 2_048, 4_096] {
            let (source_shapes, source_holes) = build(size);
            let capacity = source_shapes.len();
            let mut old_best = Duration::MAX;
            let mut sweep_best = Duration::MAX;
            let mut expected = None;

            for _ in 0..5 {
                let mut shapes = source_shapes.clone();
                let holes = source_holes.clone();
                let start = Instant::now();

                ColumnGraph::sort_join_holes(capacity, holes, &mut shapes, &mut vec![]);

                old_best = old_best.min(start.elapsed());
                black_box(&shapes);
                expected.get_or_insert(shapes);
            }

            for _ in 0..5 {
                let mut shapes = source_shapes.clone();
                let holes = source_holes.clone();
                let start = Instant::now();

                ColumnGraph::sweep_join_holes(capacity, holes, &mut shapes, &mut vec![]);

                sweep_best = sweep_best.min(start.elapsed());
                black_box(&shapes);
                assert_eq!(expected.as_ref(), Some(&shapes));
            }

            std::println!(
                "size={size:>5}, holes={:>5}, old={old_best:?}, sweep={sweep_best:?}, speedup={:.2}x",
                source_holes.len(),
                old_best.as_secs_f64() / sweep_best.as_secs_f64(),
            );
        }
    }

    fn ambiguous_hole_groups_wide(group_count: usize) -> (IntShapes<i32>, IntShape<i32>) {
        ambiguous_hole_groups(group_count, |index| (16 * index as i32, 0))
    }

    fn ambiguous_hole_groups_vertical(group_count: usize) -> (IntShapes<i32>, IntShape<i32>) {
        ambiguous_hole_groups(group_count, |index| (0, 16 * index as i32))
    }

    fn ambiguous_hole_groups_grid(group_count: usize) -> (IntShapes<i32>, IntShape<i32>) {
        let mut columns = 1;
        while columns * columns < group_count {
            columns += 1;
        }

        ambiguous_hole_groups(group_count, |index| {
            let column = index % columns;
            let row = index / columns;
            (16 * column as i32, 16 * row as i32)
        })
    }

    fn ambiguous_hole_groups(
        group_count: usize,
        position: impl Fn(usize) -> (i32, i32),
    ) -> (IntShapes<i32>, IntShape<i32>) {
        let mut shapes = Vec::with_capacity(2 * group_count);
        let mut holes = Vec::with_capacity(2 * group_count);

        for group_index in 0..group_count {
            let (x, y) = position(group_index);

            shapes.push(vec![vec![
                IntPoint::new(x, y),
                IntPoint::new(x + 10, y),
                IntPoint::new(x + 10, y + 10),
                IntPoint::new(x, y + 10),
            ]]);
            shapes.push(vec![vec![
                IntPoint::new(x + 5, y + 2),
                IntPoint::new(x + 7, y + 2),
                IntPoint::new(x + 7, y + 4),
                IntPoint::new(x + 5, y + 4),
            ]]);

            holes.push(vec![
                IntPoint::new(x + 4, y + 9),
                IntPoint::new(x + 6, y + 9),
                IntPoint::new(x + 6, y + 6),
                IntPoint::new(x + 4, y + 6),
            ]);
            holes.push(vec![
                IntPoint::new(x + 3, y + 5),
                IntPoint::new(x + 8, y + 5),
                IntPoint::new(x + 8, y + 1),
                IntPoint::new(x + 3, y + 1),
            ]);
        }

        (shapes, holes)
    }

    fn hole_chain(hole_count: usize) -> (IntShapes<i32>, IntShape<i32>) {
        let top = 3 * hole_count as i32 + 2;
        let shapes = int_shapes![[[[0, 0], [10, 0], [10, top], [0, top]],],];
        let mut holes = Vec::with_capacity(hole_count);

        for index in 0..hole_count {
            let bottom = 2 + 3 * index as i32;
            let top = bottom + 1;
            holes.push(vec![
                IntPoint::new(5, bottom),
                IntPoint::new(2, bottom),
                IntPoint::new(2, top),
                IntPoint::new(8, top),
                IntPoint::new(8, bottom),
            ]);
        }

        (shapes, holes)
    }

    #[test]
    fn test_join_holes_3() {
        let sub_paths = vec![
            sub_path(int_path![[0, 0], [10, 0], [10, 10], [0, 10]]),
            sub_path(int_path![[0, 10], [10, 10], [10, 20], [0, 20]]),
            sub_path(int_path![[0, -10], [10, -10], [10, 0], [0, 0]]),
        ];

        let holes = int_shape![
            [[2, 2], [2, 4], [4, 4], [4, 2]],
            [[6, 2], [6, 4], [8, 4], [8, 2]],
            [[2, 6], [2, 8], [4, 8], [4, 6]],
            [[6, 6], [6, 8], [8, 8], [8, 6]],
            [[2, 12], [2, 14], [4, 14], [4, 12]],
            [[2, -8], [2, -6], [4, -6], [4, -8]],
        ];

        let mut shapes = vec![];

        let mut sub_paths_0 = sub_paths.clone();
        let mut sub_paths_1 = sub_paths.clone();
        let mut sub_paths_2 = sub_paths.clone();

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes, &mut sub_paths_0);
        ColumnGraph::sort_join_holes(10, holes.clone(), &mut shapes, &mut sub_paths_1);
        ColumnGraph::sweep_join_holes(10, holes, &mut shapes, &mut sub_paths_2);

        for ((sp0, sp1), sp2) in sub_paths_0
            .iter()
            .zip(sub_paths_1.iter())
            .zip(sub_paths_2.iter())
        {
            assert_eq!(sp0.holes.len(), sp1.holes.len());
            assert_eq!(sp0.holes.len(), sp2.holes.len());
        }

        assert_eq!(sub_paths_0[0].holes.len(), 4);
        assert_eq!(sub_paths_0[1].holes.len(), 1);
        assert_eq!(sub_paths_0[2].holes.len(), 1);
    }
}
