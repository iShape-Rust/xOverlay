use crate::core::fill_rule::FillRule;
use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column::build::ScanBuffer;
use crate::deg_90::column::extract::NodeVisitor;
use crate::deg_90::column::graph::{ColumnGraph, Node};
use crate::deg_90::column_map::Column;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::sort::one_key_cmp::OneKeyAndCmpSort;
use i_shape::int::area::Area;
use i_shape::int::shape::{IntContour, IntShapes};

#[derive(Default)]
pub(in crate::deg_90) struct SolverBuffer {
    scan: ScanBuffer,
    nodes: Vec<Node>,
    visited: Vec<NodeVisitor>,
    points: Vec<IntPoint>,
}

impl Column {
    pub(in crate::deg_90) fn extract_contours(
        &self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
        buffer: &mut SolverBuffer,
    ) -> Vec<IntContour<i32>> {
        buffer.scan.reserve(self.segments.len());
        let nodes = core::mem::take(&mut buffer.nodes);
        let graph = ColumnGraph::with_nodes(self, fill_rule, overlay_rule, &mut buffer.scan, nodes);
        let mut contours = graph.extract(overlay_rule, &mut buffer.visited, &mut buffer.points);
        buffer.nodes = graph.nodes;

        normalize_contour_directions(&mut contours, self.range);
        contours
    }
}

fn normalize_contour_directions(contours: &mut [IntContour<i32>], range: LineRange) {
    let mut reverse = alloc::vec![None; contours.len()];
    collect_reversed_contours(contours, range.min, false, &mut reverse);
    collect_reversed_contours(contours, range.max, true, &mut reverse);

    for (contour, must_reverse) in contours.iter_mut().zip(reverse) {
        if must_reverse == Some(true) {
            contour.reverse();
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BorderNode {
    y: i32,
    ccw_dir: bool,
    contour_index: usize,
    vertex_index_in_contour: usize,
}

fn collect_reversed_contours(
    contours: &[IntContour<i32>],
    border_x: i32,
    outer_edge_up: bool,
    reverse: &mut [Option<bool>],
) {
    let mut border_nodes = collect_border_nodes(contours, border_x);
    border_nodes.entries.sort_by_one_key_then_by(
        false,
        |node| node.y,
        |a, b| {
            let a_pair_y = border_pair_y(contours, *a, border_x);
            let b_pair_y = border_pair_y(contours, *b, border_x);
            // If nested intervals start together, process the longest (outermost)
            // first so active depth matches their containment depth.
            b_pair_y.cmp(&a_pair_y)
        },
    );

    let mut active_ends = Vec::new();
    for node in border_nodes.entries {
        while active_ends.last().is_some_and(|&end_y| end_y <= node.y) {
            active_ends.pop();
        }

        let pair_y = border_pair_y(contours, node, border_x);
        debug_assert!(node.y < pair_y, "only lower border nodes are retained");
        if let Some(&outer_end) = active_ends.last() {
            debug_assert!(
                pair_y <= outer_end,
                "border intervals must be disjoint or properly nested"
            );
        }

        let expected_up = outer_edge_up ^ (active_ends.len() & 1 != 0);
        // At the lower endpoint `ccw_dir == false` means that the stored
        // contour follows the seam upward.
        let actual_up = !node.ccw_dir;
        let must_reverse = actual_up != expected_up;
        let entry = &mut reverse[node.contour_index];
        if let Some(previous) = *entry {
            debug_assert_eq!(
                previous, must_reverse,
                "all border intervals of one contour must agree on orientation"
            );
        } else {
            *entry = Some(must_reverse);
        }
        active_ends.push(pair_y);
    }
}

struct BorderNodes {
    entries: Vec<BorderNode>,
    #[cfg(debug_assertions)]
    exits: Vec<BorderNode>,
}

fn collect_border_nodes(contours: &[IntContour<i32>], border_x: i32) -> BorderNodes {
    let mut result = BorderNodes {
        entries: Vec::new(),
        #[cfg(debug_assertions)]
        exits: Vec::new(),
    };
    for (contour_index, contour) in contours.iter().enumerate() {
        let n = contour.len();
        if n < 3 {
            continue;
        }

        for vertex_index_in_contour in 0..n {
            let point = contour[vertex_index_in_contour];
            if point.x != border_x {
                continue;
            }

            let prev = contour[(vertex_index_in_contour + n - 1) % n];
            let next = contour[(vertex_index_in_contour + 1) % n];
            let prev_on_border = prev.x == border_x;
            let next_on_border = next.x == border_x;
            if prev_on_border == next_on_border {
                continue;
            }

            let node = BorderNode {
                y: point.y,
                ccw_dir: !next_on_border,
                contour_index,
                vertex_index_in_contour,
            };
            let pair_y = if prev_on_border { prev.y } else { next.y };
            if point.y < pair_y {
                result.entries.push(node);
            } else {
                #[cfg(debug_assertions)]
                result.exits.push(node);
            }
        }
    }

    #[cfg(debug_assertions)]
    debug_assert_eq!(
        result.entries.len(),
        result.exits.len(),
        "every retained border entry must have a debug exit"
    );

    result
}

#[inline(always)]
fn border_pair_y(contours: &[IntContour<i32>], node: BorderNode, border_x: i32) -> i32 {
    let contour = &contours[node.contour_index];
    let n = contour.len();
    let prev = contour[(node.vertex_index_in_contour + n - 1) % n];
    if prev.x == border_x {
        prev.y
    } else {
        let next = contour[(node.vertex_index_in_contour + 1) % n];
        debug_assert_eq!(next.x, border_x);
        next.y
    }
}

pub(in crate::deg_90) fn rebuild_shapes(contours: Vec<IntContour<i32>>) -> IntShapes<i32> {
    let mut shapes = Vec::new();
    let mut holes = Vec::new();

    for contour in contours {
        let area = contour.area_two();
        if area > 0 {
            shapes.push(alloc::vec![contour]);
        } else if area < 0 {
            holes.push(contour);
        }
    }

    ColumnGraph::join_holes(holes, &mut shapes);
    shapes
}

#[cfg(test)]
mod performance_tests {
    extern crate std;

    use super::normalize_contour_directions;
    use crate::geom::range::LineRange;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_shape::int::area::Area;
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    #[test]
    fn sorted_border_nodes_normalize_nested_contours() {
        let mut contours = alloc::vec![rectangle(0, 30), rectangle(5, 25), rectangle(10, 20),];

        normalize_contour_directions(&mut contours, LineRange::with_min_max(0, 10));

        assert!(contours[0].area_two() > 0);
        assert!(contours[1].area_two() < 0);
        assert!(contours[2].area_two() > 0);
    }

    #[test]
    #[ignore = "performance comparison; run explicitly with --release --ignored --nocapture"]
    fn performance_normalize_contour_directions() {
        for size in [256, 512, 1_024, 2_048, 4_096] {
            let source = border_rectangles(size);
            let mut best = Duration::MAX;

            for _ in 0..5 {
                let mut contours = source.clone();
                let start = Instant::now();
                normalize_contour_directions(&mut contours, LineRange::with_min_max(0, 10));
                best = best.min(start.elapsed());
                assert!(contours.iter().all(|contour| contour.area_two() > 0));
                black_box(&contours);
            }

            std::println!("normalize_contours={size:>5}, best={best:?}");
        }
    }

    fn border_rectangles(count: usize) -> Vec<Vec<IntPoint>> {
        let mut contours = Vec::with_capacity(count);
        for index in 0..count {
            let y = 3 * index as i32;
            contours.push(alloc::vec![
                IntPoint::new(0, y),
                IntPoint::new(10, y),
                IntPoint::new(10, y + 1),
                IntPoint::new(0, y + 1),
            ]);
        }
        contours
    }

    fn rectangle(y0: i32, y1: i32) -> Vec<IntPoint> {
        alloc::vec![
            IntPoint::new(0, y0),
            IntPoint::new(10, y0),
            IntPoint::new(10, y1),
            IntPoint::new(0, y1),
        ]
    }
}
