use crate::deg_90::column::extract::SubPath;
use crate::deg_90::column::graph::ColumnGraph;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::sort::one_key::OneKeySort;
use i_shape::int::area::Area;
use i_shape::int::shape::{IntContour, IntShapes};

impl ColumnGraph {
    pub(super) fn join_holes(
        holes: Vec<IntContour<i32>>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
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
            Self::sort_join_holes(active_segments_count, holes, shapes, sub_paths)
        }
    }

    fn direct_join_holes(
        holes: Vec<IntContour<i32>>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        for hole in holes.into_iter() {
            let p = hole[0];

            let mut best = i32::MIN;
            let mut best_shape_index = usize::MAX;
            let mut best_sub_path_index = usize::MAX;

            for (index, shape) in shapes.iter_mut().enumerate() {
                if shape[0].best_edge(p, &mut best) {
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

    fn sort_join_holes(
        capacity: usize,
        holes: Vec<IntContour<i32>>,
        shapes: &mut IntShapes<i32>,
        sub_paths: &mut Vec<SubPath>,
    ) {
        let mut edges = Vec::with_capacity(capacity);
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

        edges.sort_by_one_key(false, |e| e.y);

        for hole in holes.into_iter() {
            let e = edges.first_under(hole[0]);
            let index = e.index as usize;
            if e.is_shape {
                shapes[index].push(hole);
            } else {
                sub_paths[index].holes.push(hole);
            }
        }
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
    use crate::deg_90::column::extract::SubPath;
    use crate::deg_90::column::graph::ColumnGraph;
    use crate::deg_90::column::join_holes::{FirstBottom, ShapeEdge};
    use crate::geom::range::LineRange;
    use alloc::vec;
    use i_key_sort::sort::one_key::OneKeySort;
    use i_shape::int::path::IntPath;
    use i_shape::{int_path, int_shape, int_shapes};

    fn sub_path(path: IntPath<i32>) -> SubPath {
        SubPath {
            start: Default::default(),
            end: Default::default(),
            path,
            holes: vec![],
        }
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

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes_0, &mut sub_paths);
        ColumnGraph::sort_join_holes(10, holes, &mut shapes_1, &mut sub_paths);

        assert_eq!(shapes_0, shapes_1);
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

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes_0, &mut sub_paths);
        ColumnGraph::sort_join_holes(10, holes, &mut shapes_1, &mut sub_paths);

        assert_eq!(shapes_0, shapes_1);
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

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes_0, &mut sub_paths);
        ColumnGraph::sort_join_holes(10, holes, &mut shapes_1, &mut sub_paths);

        assert_eq!(shapes_0, shapes_1);
        assert_eq!(shapes_0[0].len(), 5);
        assert_eq!(shapes_0[1].len(), 2);
        assert_eq!(shapes_0[2].len(), 2);
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

        ColumnGraph::direct_join_holes(holes.clone(), &mut shapes, &mut sub_paths_0);
        ColumnGraph::sort_join_holes(10, holes, &mut shapes, &mut sub_paths_1);

        for (sp0, sp1) in sub_paths_0.iter().zip(sub_paths_1.iter()) {
            assert_eq!(sp0.holes.len(), sp1.holes.len());
        }

        assert_eq!(sub_paths_0[0].holes.len(), 4);
        assert_eq!(sub_paths_0[1].holes.len(), 1);
        assert_eq!(sub_paths_0[2].holes.len(), 1);
    }
}
