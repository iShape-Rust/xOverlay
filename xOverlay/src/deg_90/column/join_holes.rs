use crate::deg_90::column::extract::{SubPath};
use crate::deg_90::column::graph::ColumnGraph;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::sort::one_key::OneKeySort;
use i_shape::int::shape::{IntContour, IntShapes};
use crate::geom::range::LineRange;

impl ColumnGraph {
    pub(super) fn join_holes(
        holes: Vec<IntContour>,
        shapes: &mut IntShapes,
        sub_paths: &mut Vec<SubPath>,
    ) {
        let mut active_segments_count = 0;

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

        }
    }

    fn direct_join_holes(
        holes: Vec<IntContour>,
        shapes: &mut IntShapes,
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
        holes: Vec<IntContour>,
        shapes: &mut IntShapes,
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

        edges.sort_by_one_key(false, |e|e.y);

        for hole in holes.into_iter() {
            let y = hole[0].y + 1;
            edges.binary_search_by_key(&p.y, |e|e.y);

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

impl BestEdge for IntContour {
    fn best_edge(&self, p: IntPoint, best: &mut i32) -> bool {
        let mut result = false;
        let mut a = self[self.len() - 1];
        for &b in self.iter() {
            if a.x < b.x && a.y < p.y && a.x <= p.x && b.x < p.x && *best < a.y {
                *best = a.y;
                result = true;
            }
            a = b;
        }
        result
    }
}
