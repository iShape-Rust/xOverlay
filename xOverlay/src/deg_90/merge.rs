use crate::deg_90::sub_graph::SubGraph;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use core::cmp::Ordering;
use i_float::int::point::IntPoint;
use i_key_sort::sort::one_key::OneKeySort;
use i_key_sort::sort::two_keys::TwoKeysSort;
use i_shape::int::shape::IntContour;
#[cfg(feature = "allow_multithreading")]
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

pub(super) trait Merge {
    fn merge(self) -> SubGraph;
}

#[cfg(feature = "allow_multithreading")]
pub(super) trait ParallelMerge {
    fn parallel_merge(self) -> SubGraph;
}

impl Merge for Vec<SubGraph> {
    fn merge(self) -> SubGraph {
        let mut iter = self.into_iter();
        let Some(mut result) = iter.next() else {
            return SubGraph {
                range: LineRange::default(),
                contours: Vec::new(),
            };
        };

        for right in iter {
            result = merge_pair(result, right);
        }

        result
    }
}

#[cfg(feature = "allow_multithreading")]
impl ParallelMerge for Vec<SubGraph> {
    fn parallel_merge(self) -> SubGraph {
        let mut groups = self;

        while groups.len() > 1 {
            groups = groups.into_par_iter().chunks(2).map(Merge::merge).collect();
        }

        groups.merge()
    }
}

fn merge_pair(left: SubGraph, right: SubGraph) -> SubGraph {
    debug_assert_eq!(
        left.range.max, right.range.min,
        "only neighboring column groups can be merged"
    );

    let border_x = left.range.max;
    let contours = merge_contours(left.contours, right.contours, border_x);

    SubGraph {
        range: LineRange::with_min_max(left.range.min, right.range.max),
        contours,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug)]
struct BorderNode {
    y: i32,
    ccw_dir: bool,
    contour_index: usize,
    vertex_index_in_contour: usize,
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    a: IntPoint,
    b: IntPoint,
    prev: usize,
    next: usize,
    alive: bool,
    side: Side,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BorderEdge {
    y0: i32,
    y1: i32,
    edge_index: usize,
}

fn merge_contours(
    left: Vec<IntContour<i32>>,
    right: Vec<IntContour<i32>>,
    border_x: i32,
) -> Vec<IntContour<i32>> {
    #[cfg(debug_assertions)]
    validate_border_nodes(&left, Side::Left, border_x);
    #[cfg(debug_assertions)]
    validate_border_nodes(&right, Side::Right, border_x);

    let (mut left, mut untouched) = partition_border_contours(left, border_x);
    let (mut right, mut right_untouched) = partition_border_contours(right, border_x);
    untouched.append(&mut right_untouched);

    let cuts = border_cuts(&left, &right, border_x);
    split_border_edges(&mut left, border_x, &cuts);
    split_border_edges(&mut right, border_x, &cuts);

    // A local hole may touch the artificial column border. Its seam edge then
    // mirrors a hull seam edge on the same side. Resolve these local pairs
    // first, producing the C-shaped boundary that participates in the actual
    // inter-column merge.
    left = reduce_local_border(left, Side::Left, border_x);
    right = reduce_local_border(right, Side::Right, border_x);
    // Local contour rebuilding removes collinear seam points. Restore the
    // shared event partition before matching partially overlapping spans.
    split_border_edges(&mut left, border_x, &cuts);
    split_border_edges(&mut right, border_x, &cuts);

    // Same-side reduction can close a contour away from this seam. Do not put
    // such contours through the cross-seam edge graph either.
    let (left, mut left_untouched) = partition_border_contours(left, border_x);
    let (right, mut right_untouched) = partition_border_contours(right, border_x);
    untouched.append(&mut left_untouched);
    untouched.append(&mut right_untouched);

    let mut edges = Vec::new();
    append_edges(&left, Side::Left, &mut edges);
    append_edges(&right, Side::Right, &mut edges);

    let mut left_border = border_edges(&edges, Side::Left, border_x);
    let mut right_border = border_edges(&edges, Side::Right, border_x);
    left_border.sort_by_two_keys(false, |edge| edge.y0, |edge| edge.y1);
    right_border.sort_by_two_keys(false, |edge| edge.y0, |edge| edge.y1);

    #[cfg(debug_assertions)]
    {
        assert_unique_spans(&left_border);
        assert_unique_spans(&right_border);
    }

    let matches = matching_edges(&left_border, &right_border);
    if matches.is_empty() {
        untouched.extend(left);
        untouched.extend(right);
        return untouched;
    }

    apply_matches(&mut edges, &matches);
    untouched.extend(collect_contours(&edges));
    untouched
}

fn partition_border_contours(
    contours: Vec<IntContour<i32>>,
    border_x: i32,
) -> (Vec<IntContour<i32>>, Vec<IntContour<i32>>) {
    let mut touching = Vec::new();
    let mut untouched = Vec::new();

    for contour in contours {
        if has_border_edge(&contour, border_x) {
            touching.push(contour);
        } else {
            untouched.push(contour);
        }
    }

    (touching, untouched)
}

fn has_border_edge(contour: &IntContour<i32>, border_x: i32) -> bool {
    let count = contour.len();
    if count < 2 {
        return false;
    }

    (0..count).any(|index| {
        let a = contour[index];
        let b = contour[(index + 1) % count];
        a.x == border_x && b.x == border_x && a.y != b.y
    })
}

fn reduce_local_border(
    contours: Vec<IntContour<i32>>,
    side: Side,
    border_x: i32,
) -> Vec<IntContour<i32>> {
    let mut edges = Vec::new();
    append_edges(&contours, side, &mut edges);

    let mut border = border_edges(&edges, side, border_x);
    border.sort_by_two_keys(false, |edge| edge.y0, |edge| edge.y1);
    let (_, matches) = reduce_same_side_edges(&edges, border);
    if matches.is_empty() {
        return contours;
    }

    apply_matches(&mut edges, &matches);
    collect_contours(&edges)
}

fn apply_matches(edges: &mut [Edge], matches: &[(usize, usize)]) {
    for &(left_edge, right_edge) in matches {
        let left = edges[left_edge];
        let right = edges[right_edge];
        debug_assert_eq!(left.a, right.b, "matched seam edges must be mirrored");
        debug_assert_eq!(left.b, right.a, "matched seam edges must be mirrored");
        edges[left_edge].alive = false;
        edges[right_edge].alive = false;
    }

    // All common seam atoms are marked first. Adjacent atoms therefore act as
    // one removed chain and do not leave intermediate seam vertices behind.
    for &(left_edge, right_edge) in matches {
        let left_prev = previous_alive(edges, left_edge);
        let left_next = next_alive(edges, left_edge);
        let right_prev = previous_alive(edges, right_edge);
        let right_next = next_alive(edges, right_edge);

        connect(edges, left_prev, right_next);
        connect(edges, right_prev, left_next);
    }
}

fn border_cuts(left: &[IntContour<i32>], right: &[IntContour<i32>], border_x: i32) -> Vec<i32> {
    let mut cuts = Vec::new();
    for contour in left.iter().chain(right) {
        for &point in contour {
            if point.x == border_x {
                cuts.push(point.y);
            }
        }
    }
    cuts.sort_by_one_key(false, |y| *y);
    cuts.dedup();
    cuts
}

fn split_border_edges(contours: &mut [IntContour<i32>], border_x: i32, cuts: &[i32]) {
    for contour in contours {
        if contour.len() < 2 {
            continue;
        }

        let mut result = Vec::with_capacity(contour.len());
        for index in 0..contour.len() {
            let a = contour[index];
            let b = contour[(index + 1) % contour.len()];
            result.push(a);

            if a.x != border_x || b.x != border_x || a.y == b.y {
                continue;
            }

            let y0 = a.y.min(b.y);
            let y1 = a.y.max(b.y);
            if a.y < b.y {
                for &y in cuts {
                    if y0 < y && y < y1 {
                        result.push(IntPoint::new(border_x, y));
                    }
                }
            } else {
                for &y in cuts.iter().rev() {
                    if y0 < y && y < y1 {
                        result.push(IntPoint::new(border_x, y));
                    }
                }
            }
        }

        *contour = result;
    }
}

fn append_edges(contours: &[IntContour<i32>], side: Side, edges: &mut Vec<Edge>) {
    for contour in contours {
        if contour.len() < 2 {
            continue;
        }

        let offset = edges.len();
        let count = contour.len();
        edges.reserve(count);

        for index in 0..count {
            edges.push(Edge {
                a: contour[index],
                b: contour[(index + 1) % count],
                prev: offset + (index + count - 1) % count,
                next: offset + (index + 1) % count,
                alive: true,
                side,
            });
        }
    }
}

fn border_edges(edges: &[Edge], side: Side, border_x: i32) -> Vec<BorderEdge> {
    edges
        .iter()
        .enumerate()
        .filter_map(|(edge_index, edge)| {
            if edge.side != side
                || edge.a.x != border_x
                || edge.b.x != border_x
                || edge.a.y == edge.b.y
            {
                return None;
            }

            Some(BorderEdge {
                y0: edge.a.y.min(edge.b.y),
                y1: edge.a.y.max(edge.b.y),
                edge_index,
            })
        })
        .collect()
}

fn matching_edges(left: &[BorderEdge], right: &[BorderEdge]) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < left.len() && j < right.len() {
        let a = left[i];
        let b = right[j];
        match (a.y0, a.y1).cmp(&(b.y0, b.y1)) {
            Ordering::Less => i += 1,
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                matches.push((a.edge_index, b.edge_index));
                i += 1;
                j += 1;
            }
        }
    }

    matches
}

fn reduce_same_side_edges(
    edges: &[Edge],
    border: Vec<BorderEdge>,
) -> (Vec<BorderEdge>, Vec<(usize, usize)>) {
    let mut remaining = Vec::with_capacity(border.len());
    let mut matches = Vec::new();
    let mut start = 0;

    while start < border.len() {
        let key = (border[start].y0, border[start].y1);
        let mut end = start + 1;
        while end < border.len() && (border[end].y0, border[end].y1) == key {
            end += 1;
        }

        let mut up = Vec::new();
        let mut down = Vec::new();
        for border_edge in &border[start..end] {
            let edge = edges[border_edge.edge_index];
            if edge.a.y < edge.b.y {
                up.push(*border_edge);
            } else {
                down.push(*border_edge);
            }
        }

        let pair_count = up.len().min(down.len());
        for index in 0..pair_count {
            matches.push((up[index].edge_index, down[index].edge_index));
        }
        remaining.extend_from_slice(&up[pair_count..]);
        remaining.extend_from_slice(&down[pair_count..]);

        start = end;
    }

    remaining.sort_by_two_keys(false, |edge| edge.y0, |edge| edge.y1);
    (remaining, matches)
}

fn previous_alive(edges: &[Edge], edge_index: usize) -> usize {
    let mut current = edges[edge_index].prev;
    for _ in 0..edges.len() {
        if edges[current].alive {
            return current;
        }
        current = edges[current].prev;
    }
    panic!("a contour cannot consist only of a removed seam")
}

fn next_alive(edges: &[Edge], edge_index: usize) -> usize {
    let mut current = edges[edge_index].next;
    for _ in 0..edges.len() {
        if edges[current].alive {
            return current;
        }
        current = edges[current].next;
    }
    panic!("a contour cannot consist only of a removed seam")
}

fn connect(edges: &mut [Edge], from: usize, to: usize) {
    debug_assert!(edges[from].alive && edges[to].alive);
    debug_assert_eq!(
        edges[from].b, edges[to].a,
        "stitched contour edges must share a vertex"
    );
    edges[from].next = to;
    edges[to].prev = from;
}

fn collect_contours(edges: &[Edge]) -> Vec<IntContour<i32>> {
    let mut visited = alloc::vec![false; edges.len()];
    let mut contours = Vec::new();

    for start in 0..edges.len() {
        if !edges[start].alive || visited[start] {
            continue;
        }

        let mut contour = Vec::new();
        let mut current = start;
        loop {
            assert!(
                edges[current].alive,
                "a contour points to a removed seam edge"
            );
            assert!(
                !visited[current],
                "a stitched contour does not close at its start"
            );
            visited[current] = true;
            contour.push(edges[current].a);

            let next = edges[current].next;
            assert_eq!(edges[current].b, edges[next].a, "broken contour linkage");
            current = next;
            if current == start {
                break;
            }
        }

        simplify_contour(&mut contour);
        if contour.len() >= 4 {
            contours.push(contour);
        }
    }

    contours
}

fn simplify_contour(contour: &mut IntContour<i32>) {
    if contour.len() < 3 {
        return;
    }

    let mut result = Vec::with_capacity(contour.len());
    for &point in contour.iter() {
        if result.last() == Some(&point) {
            continue;
        }

        while result.len() >= 2 {
            let a = result[result.len() - 2];
            let b = result[result.len() - 1];
            if !is_collinear(a, b, point) {
                break;
            }
            result.pop();
        }
        result.push(point);
    }

    loop {
        let n = result.len();
        if n < 3 {
            break;
        }
        if is_collinear(result[n - 1], result[0], result[1]) {
            result.remove(0);
        } else if is_collinear(result[n - 2], result[n - 1], result[0]) {
            result.pop();
        } else {
            break;
        }
    }

    *contour = result;
}

#[inline(always)]
fn is_collinear(a: IntPoint, b: IntPoint, c: IntPoint) -> bool {
    (a.x == b.x && b.x == c.x) || (a.y == b.y && b.y == c.y)
}

#[cfg(debug_assertions)]
fn collect_border_nodes(contours: &[IntContour<i32>], border_x: i32) -> Vec<BorderNode> {
    let mut nodes = Vec::new();
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

            nodes.push(BorderNode {
                y: point.y,
                ccw_dir: !next_on_border,
                contour_index,
                vertex_index_in_contour,
            });
        }
    }
    nodes.sort_by_one_key(false, |node| node.y);
    nodes
}

#[cfg(debug_assertions)]
fn validate_border_nodes(contours: &[IntContour<i32>], side: Side, border_x: i32) {
    let nodes = collect_border_nodes(contours, border_x);
    if nodes.is_empty() {
        return;
    }

    debug_assert_eq!(nodes.len() & 1, 0, "border nodes must form intervals");
    debug_assert!(nodes.windows(2).all(|pair| pair[0].y <= pair[1].y));

    for node in &nodes {
        let contour = &contours[node.contour_index];
        let n = contour.len();
        let prev = contour[(node.vertex_index_in_contour + n - 1) % n];
        let next = contour[(node.vertex_index_in_contour + 1) % n];
        debug_assert_eq!(node.ccw_dir, next.x != border_x);
        debug_assert_ne!(prev.x == border_x, next.x == border_x);
    }

    let forward_count = nodes.iter().filter(|node| node.ccw_dir).count();
    debug_assert_eq!(
        forward_count * 2,
        nodes.len(),
        "every seam interval must have one forward and one backward portal on the {side:?} side"
    );
}

#[cfg(debug_assertions)]
fn assert_unique_spans(edges: &[BorderEdge]) {
    for pair in edges.windows(2) {
        debug_assert_ne!(
            (pair[0].y0, pair[0].y1),
            (pair[1].y0, pair[1].y1),
            "filled shapes on one side cannot overlap on a seam"
        );
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    #[cfg(feature = "allow_multithreading")]
    use super::ParallelMerge;
    use super::{Merge, SubGraph};
    use crate::core::cpu_count::CPUCount;
    use crate::core::fill_rule::FillRule;
    use crate::core::options::IntOverlayOptions;
    use crate::core::overlay::Overlay;
    use crate::core::overlay_rule::OverlayRule;
    use crate::deg_90::column;
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;
    use crate::geom::range::LineRange;
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_shape::int::area::Area;
    use i_shape::int::path::ContourExtension;
    use i_shape::int::shape::{IntContour, IntShape, IntShapes};
    use i_shape::{int_path, int_shape};
    use rand::rngs::StdRng;
    use rand::{RngExt, SeedableRng};
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    #[test]
    fn joins_rectangles_across_seam() {
        let left = sub_graph(
            -10,
            0,
            vec![int_path![[-10, -5], [0, -5], [0, 5], [-10, 5]]],
        );
        let right = sub_graph(0, 10, vec![int_path![[0, -5], [10, -5], [10, 5], [0, 5]]]);

        let merged = vec![left, right].merge();
        assert_eq!(merged.contours.len(), 1);
        assert_eq!(merged.contours[0].len(), 4);
        assert_eq!(merged.contours[0].area_two(), 400);
    }

    #[cfg(feature = "allow_multithreading")]
    #[test]
    fn parallel_binary_merge_handles_odd_column_count() {
        let make_parts = || {
            (-2..3)
                .map(|index| {
                    let x = index * 10;
                    sub_graph(x, x + 10, vec![rectangle(x, -5, 10, 10)])
                })
                .collect::<Vec<_>>()
        };

        let serial = make_parts().merge();
        let parallel = make_parts().parallel_merge();

        assert_eq!(parallel.range, serial.range);
        assert_eq!(parallel.contours, serial.contours);
        assert_eq!(parallel.contours.len(), 1);
        assert_eq!(parallel.contours[0].len(), 4);
        assert_eq!(parallel.contours[0].area_two(), 1_000);
    }

    #[test]
    fn joins_partially_overlapping_seam_intervals() {
        let left = sub_graph(
            -10,
            0,
            vec![int_path![[-10, 0], [0, 0], [0, 10], [-10, 10]]],
        );
        let right = sub_graph(0, 10, vec![int_path![[0, 0], [10, 0], [10, 5], [0, 5]]]);

        let merged = vec![left, right].merge();
        assert_eq!(merged.contours.len(), 1);
        assert_eq!(merged.contours[0].len(), 6);
        assert_eq!(merged.contours[0].area_two(), 300);
    }

    #[test]
    fn mirrored_cs_create_a_hole() {
        let left = sub_graph(
            -10,
            0,
            vec![int_path![
                [-10, -10],
                [0, -10],
                [0, -5],
                [-5, -5],
                [-5, 5],
                [0, 5],
                [0, 10],
                [-10, 10],
            ]],
        );
        let right = sub_graph(
            0,
            10,
            vec![int_path![
                [0, -10],
                [10, -10],
                [10, 10],
                [0, 10],
                [0, 5],
                [5, 5],
                [5, -5],
                [0, -5],
            ]],
        );

        let merged = vec![left, right].merge();
        assert_eq!(merged.contours.len(), 2);

        let shapes = column::rebuild_shapes(merged.contours);
        assert_eq!(
            shapes.len(),
            1,
            "areas: {:?}",
            shapes
                .iter()
                .map(|shape| shape.iter().map(|c| c.area_two()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(shapes.area_two(), 600);
        assert!(shapes[0][0].area_two() > 0);
        assert!(shapes[0][1].area_two() < 0);
    }

    #[test]
    fn column_graphs_rebuild_a_hole_across_the_seam() {
        let subject = int_shape![
            [[-16, -16], [16, -16], [16, 16], [-16, 16]],
            [[-5, -5], [-5, 5], [5, 5], [5, -5]],
        ];
        let map = ColumnMap::with_columns_count(&subject, &[], 2);
        assert_eq!(map.columns.len(), 2);

        let parts: Vec<_> = map
            .columns
            .into_iter()
            .map(|column| SubGraph::with_column(column, FillRule::NonZero, OverlayRule::Subject))
            .collect();
        let shapes = parts.merge().into_shapes();

        assert_eq!(
            shapes.len(),
            1,
            "areas: {:?}",
            shapes
                .iter()
                .map(|shape| shape.iter().map(|c| c.area_two()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(shapes.area_two(), 1848);
        assert!(shapes[0][0].area_two() > 0);
        assert!(shapes[0][1].area_two() < 0);
    }

    #[test]
    fn column_graphs_rebuild_a_hole_across_multiple_seams() {
        let subject = int_shape![
            [[-32, -32], [32, -32], [32, 32], [-32, 32]],
            [[-20, -10], [-20, 10], [20, 10], [20, -10]],
        ];
        let map = ColumnMap::with_columns_count(&subject, &[], 4);
        assert_eq!(map.columns.len(), 4);

        let parts: Vec<_> = map
            .columns
            .into_iter()
            .map(|column| SubGraph::with_column(column, FillRule::NonZero, OverlayRule::Subject))
            .collect();
        let shapes = parts.merge().into_shapes();

        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(shapes.area_two(), 6592);
        assert!(shapes[0][0].area_two() > 0);
        assert!(shapes[0][1].area_two() < 0);
    }

    #[test]
    fn randomized_multi_column_merge_matches_i_overlay() {
        const GEOMETRY_SEED: u64 = 0xC011_4D6E_5EED;
        let mut rng = StdRng::seed_from_u64(GEOMETRY_SEED);

        for case_index in 0..128 {
            let subject = random_rectangles(&mut rng, 8, SideAnchor::Left);
            let clip = random_rectangles(&mut rng, 8, SideAnchor::Right);

            for fill_rule in [
                FillRule::EvenOdd,
                FillRule::NonZero,
                FillRule::Positive,
                FillRule::Negative,
            ] {
                for overlay_rule in [
                    OverlayRule::Subject,
                    OverlayRule::Clip,
                    OverlayRule::Intersect,
                    OverlayRule::Union,
                    OverlayRule::Difference,
                    OverlayRule::InverseDifference,
                    OverlayRule::Xor,
                ] {
                    let expected = i_overlay_shapes(&subject, &clip, fill_rule, overlay_rule);

                    for columns_count in [2, 4, 8] {
                        let actual = extract_multi_column(
                            &subject,
                            &clip,
                            columns_count,
                            fill_rule,
                            overlay_rule,
                        );

                        assert_eq!(
                            actual.area_two(),
                            expected.area_two(),
                            "area mismatch: seed={GEOMETRY_SEED:#x}, case={case_index}, columns={columns_count}, fill={fill_rule:?}, overlay={overlay_rule:?}, subject={subject:?}, clip={clip:?}, actual={actual:?}, expected={expected:?}"
                        );
                        assert_valid_directions(
                            &actual,
                            GEOMETRY_SEED,
                            case_index,
                            columns_count,
                            fill_rule,
                            overlay_rule,
                        );
                        assert_same_random_samples(
                            &actual,
                            &expected,
                            GEOMETRY_SEED ^ (case_index as u64).rotate_left(19),
                            case_index,
                            columns_count,
                            fill_rule,
                            overlay_rule,
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "performance comparison; run explicitly with --release --ignored --nocapture"]
    fn performance_checkerboard_against_i_overlay() {
        const N: usize = 128;
        const BENCH_TIME: Duration = Duration::from_millis(600);

        // Matches iOverlay's official CheckerboardTest workload:
        // https://ishape-rust.github.io/iShape-js/overlay/performance/performance.html
        let subject = many_squares(IntPoint::new(0, 0), 20, 30, N);
        let clip = many_squares(IntPoint::new(15, 15), 20, 30, N - 1);
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Xor;

        let expected = i_overlay_shapes(&subject, &clip, fill_rule, overlay_rule);
        let one_column = extract_multi_column(&subject, &clip, 1, fill_rule, overlay_rule);
        let two_columns = extract_multi_column(&subject, &clip, 2, fill_rule, overlay_rule);
        let four_columns = extract_multi_column(&subject, &clip, 4, fill_rule, overlay_rule);
        let eight_columns = extract_multi_column(&subject, &clip, 8, fill_rule, overlay_rule);
        let thirty_columns = extract_multi_column(&subject, &clip, 30, fill_rule, overlay_rule);
        let sixty_columns = extract_multi_column(&subject, &clip, 60, fill_rule, overlay_rule);
        assert_eq!(one_column.area_two(), expected.area_two());
        assert_eq!(two_columns.area_two(), expected.area_two());
        assert_eq!(four_columns.area_two(), expected.area_two());
        assert_eq!(eight_columns.area_two(), expected.area_two());
        assert_eq!(thirty_columns.area_two(), expected.area_two());
        assert_eq!(sixty_columns.area_two(), expected.area_two());

        let x_one = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 1, fill_rule, overlay_rule)
        });
        let x_two = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 2, fill_rule, overlay_rule)
        });
        let x_four = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 4, fill_rule, overlay_rule)
        });
        let x_eight = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 8, fill_rule, overlay_rule)
        });
        let x_thirty = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 30, fill_rule, overlay_rule)
        });
        let x_sixty = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 60, fill_rule, overlay_rule)
        });
        let i_overlay = measure_for(BENCH_TIME, || {
            i_overlay_shapes(&subject, &clip, fill_rule, overlay_rule)
        });

        std::println!(
            "checkerboard: n={N}, squares={}, fill={fill_rule:?}, overlay={overlay_rule:?}, budget={BENCH_TIME:?}",
            N * N + (N - 1) * (N - 1),
        );
        print_measurement("xOverlay columns=1", x_one);
        print_measurement("xOverlay columns=2", x_two);
        print_measurement("xOverlay columns=4", x_four);
        print_measurement("xOverlay columns=8", x_eight);
        print_measurement("xOverlay columns=30", x_thirty);
        print_measurement("xOverlay columns=60", x_sixty);
        print_measurement("iOverlay", i_overlay);
        std::println!(
            "vs iOverlay: columns(1)={:.2}x, columns(2)={:.2}x, columns(4)={:.2}x, columns(8)={:.2}x, columns(30)={:.2}x, columns(60)={:.2}x",
            i_overlay.ns_per_iteration() / x_one.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_two.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_four.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_eight.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_thirty.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_sixty.ns_per_iteration(),
        );
    }

    #[test]
    #[ignore = "single-thread profiling workload; run explicitly with --release --ignored --nocapture"]
    fn profile_checkerboard_single_thread() {
        const N: usize = 128;
        const COLUMNS: usize = 60;
        const PROFILE_TIME: Duration = Duration::from_secs(5);

        let subject = many_squares(IntPoint::new(0, 0), 20, 30, N);
        let clip = many_squares(IntPoint::new(15, 15), 20, 30, N - 1);
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Xor;
        let options = multi_column_options(&subject, &clip, COLUMNS);

        let mut map_time = Duration::ZERO;
        let mut column_time = Duration::ZERO;
        let mut merge_time = Duration::ZERO;
        let mut rebuild_time = Duration::ZERO;
        let profile_start = Instant::now();
        let mut iterations = 0;

        while iterations == 0 || profile_start.elapsed() < PROFILE_TIME {
            let start = Instant::now();
            let overlay = Overlay::with_contours_custom(&subject, &clip, options, CPUCount::Single);
            map_time += start.elapsed();
            assert_eq!(overlay.columns.len(), COLUMNS);

            let start = Instant::now();
            let sub_graphs = overlay
                .columns
                .into_iter()
                .map(|column| column.test_process(fill_rule, overlay_rule, options.columns_config))
                .collect::<Vec<_>>();
            column_time += start.elapsed();

            let start = Instant::now();
            let merged = sub_graphs.merge();
            merge_time += start.elapsed();

            let start = Instant::now();
            black_box(merged.into_shapes());
            rebuild_time += start.elapsed();
            iterations += 1;
        }

        let measured = map_time + column_time + merge_time + rebuild_time;
        std::println!(
            "single-thread checkerboard: n={N}, columns={COLUMNS}, iterations={iterations}, measured={measured:?}"
        );
        print_profile_stage("column map", map_time, measured, iterations);
        print_profile_stage("column process", column_time, measured, iterations);
        print_profile_stage("serial merge", merge_time, measured, iterations);
        print_profile_stage("rebuild shapes", rebuild_time, measured, iterations);
    }

    fn print_profile_stage(label: &str, elapsed: Duration, total: Duration, iterations: usize) {
        std::println!(
            "{label:>18}: {:8.3} ms/iter, {:5.1}%",
            elapsed.as_secs_f64() * 1_000.0 / iterations as f64,
            100.0 * elapsed.as_secs_f64() / total.as_secs_f64(),
        );
    }

    #[derive(Clone, Copy)]
    struct Measurement {
        iterations: usize,
        elapsed: Duration,
    }

    impl Measurement {
        fn ns_per_iteration(self) -> f64 {
            self.elapsed.as_secs_f64() * 1_000_000_000.0 / self.iterations as f64
        }
    }

    fn measure_for(budget: Duration, mut operation: impl FnMut() -> IntShapes<i32>) -> Measurement {
        black_box(operation());

        let start = Instant::now();
        let mut iterations = 0;
        while iterations == 0 || start.elapsed() < budget {
            black_box(operation());
            iterations += 1;
        }

        Measurement {
            iterations,
            elapsed: start.elapsed(),
        }
    }

    fn print_measurement(name: &str, measurement: Measurement) {
        let milliseconds = measurement.ns_per_iteration() / 1_000_000.0;
        let iterations_per_second = 1_000.0 / milliseconds;
        std::println!(
            "{name:>20}: {milliseconds:>10.3} ms/iter, {iterations_per_second:>10.1} iter/s, iterations={}",
            measurement.iterations,
        );
    }

    #[derive(Clone, Copy)]
    enum SideAnchor {
        Left,
        Right,
    }

    fn random_rectangles(rng: &mut StdRng, count: usize, anchor: SideAnchor) -> IntShape<i32> {
        let anchor_x = match anchor {
            SideAnchor::Left => -64,
            SideAnchor::Right => 62,
        };
        let anchor_y = match anchor {
            SideAnchor::Left => -62,
            SideAnchor::Right => 60,
        };
        let mut contours = Vec::with_capacity(count + 1);
        let mut anchor = rectangle(anchor_x, anchor_y, 2, 2);
        if rng.random_range(0..2) == 0 {
            anchor.reverse();
        }
        contours.push(anchor);

        for _ in 0..count {
            let x = 2 * rng.random_range(-28..25);
            let y = 2 * rng.random_range(-28..25);
            let width = 2 * rng.random_range(1..9);
            let height = 2 * rng.random_range(1..9);
            let mut contour = rectangle(x, y, width, height);
            if rng.random_range(0..2) == 0 {
                contour.reverse();
            }
            contours.push(contour);
        }

        contours
    }

    fn many_squares(start: IntPoint, size: i32, offset: i32, n: usize) -> IntShape<i32> {
        let mut contours = Vec::with_capacity(n * n);
        let mut y = start.y;
        for _ in 0..n {
            let mut x = start.x;
            for _ in 0..n {
                contours.push(vec![
                    IntPoint::new(x, y),
                    IntPoint::new(x, y + size),
                    IntPoint::new(x + size, y + size),
                    IntPoint::new(x + size, y),
                ]);
                x += offset;
            }
            y += offset;
        }

        contours
    }

    fn rectangle(x: i32, y: i32, width: i32, height: i32) -> IntContour<i32> {
        vec![
            IntPoint::new(x, y),
            IntPoint::new(x + width, y),
            IntPoint::new(x + width, y + height),
            IntPoint::new(x, y + height),
        ]
    }

    fn extract_multi_column(
        subject: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        columns_count: usize,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> IntShapes<i32> {
        let options = multi_column_options(subject, clip, columns_count);
        let overlay =
            Overlay::with_contours_custom(subject, clip, options, CPUCount::Fixed(columns_count));
        assert_eq!(overlay.columns.len(), columns_count);
        overlay.overlay(fill_rule, overlay_rule)
    }

    fn multi_column_options(
        subject: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        columns_count: usize,
    ) -> IntOverlayOptions {
        let width = input_width(subject, clip);
        let min_column_width_power = (0..usize::BITS as usize)
            .find(|&power| ((width.saturating_sub(1) >> power) + 1) == columns_count)
            .unwrap_or_else(|| {
                panic!("column count {columns_count} is not representable for input width {width}")
            });
        IntOverlayOptions {
            columns_config: ColumnConfig90 {
                min_columns_count: columns_count,
                min_column_width_power,
                min_allowed_segments_per_column: 1_000_000,
                max_allow_segments_per_column: 1_000_000_000,
                max_allowed_segments_per_line: 128,
            },
            ..Default::default()
        }
    }

    fn input_width(subject: &[IntContour<i32>], clip: &[IntContour<i32>]) -> usize {
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        for contour in subject.iter().chain(clip) {
            for point in contour {
                min_x = min_x.min(point.x);
                max_x = max_x.max(point.x);
            }
        }
        (max_x - min_x) as usize
    }

    fn i_overlay_shapes(
        subject: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> IntShapes<i32> {
        use i_overlay::core::fill_rule::FillRule as IFillRule;
        use i_overlay::core::overlay::Overlay as IOverlay;
        use i_overlay::core::overlay_rule::OverlayRule as IOverlayRule;

        let i_fill_rule = match fill_rule {
            FillRule::EvenOdd => IFillRule::EvenOdd,
            FillRule::NonZero => IFillRule::NonZero,
            FillRule::Positive => IFillRule::Positive,
            FillRule::Negative => IFillRule::Negative,
        };
        let i_overlay_rule = match overlay_rule {
            OverlayRule::Subject => IOverlayRule::Subject,
            OverlayRule::Clip => IOverlayRule::Clip,
            OverlayRule::Intersect => IOverlayRule::Intersect,
            OverlayRule::Union => IOverlayRule::Union,
            OverlayRule::Difference => IOverlayRule::Difference,
            OverlayRule::InverseDifference => IOverlayRule::InverseDifference,
            OverlayRule::Xor => IOverlayRule::Xor,
        };

        let mut overlay = IOverlay::with_contours(subject, clip);
        overlay.overlay(i_overlay_rule, i_fill_rule)
    }

    fn assert_valid_directions(
        shapes: &IntShapes<i32>,
        geometry_seed: u64,
        case_index: usize,
        columns_count: usize,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        for (shape_index, shape) in shapes.iter().enumerate() {
            assert!(
                shape.first().is_some_and(|hull| hull.area_two() > 0),
                "invalid hull direction: seed={geometry_seed:#x}, case={case_index}, columns={columns_count}, shape={shape_index}, fill={fill_rule:?}, overlay={overlay_rule:?}"
            );
            assert!(
                shape[1..].iter().all(|hole| hole.area_two() < 0),
                "invalid hole direction: seed={geometry_seed:#x}, case={case_index}, columns={columns_count}, shape={shape_index}, fill={fill_rule:?}, overlay={overlay_rule:?}"
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn assert_same_random_samples(
        actual: &IntShapes<i32>,
        expected: &IntShapes<i32>,
        sample_seed: u64,
        case_index: usize,
        columns_count: usize,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        let mut rng = StdRng::seed_from_u64(sample_seed);
        for sample_index in 0..128 {
            // All generated boundary coordinates are even. Odd samples cannot
            // land on an edge, so both solvers have an unambiguous answer.
            let point = IntPoint::new(
                2 * rng.random_range(-33..33) + 1,
                2 * rng.random_range(-33..33) + 1,
            );
            assert_eq!(
                shapes_contain(actual, point),
                shapes_contain(expected, point),
                "sample mismatch at {point:?}: sample_seed={sample_seed:#x}, case={case_index}, columns={columns_count}, sample={sample_index}, fill={fill_rule:?}, overlay={overlay_rule:?}, actual={actual:?}, expected={expected:?}"
            );
        }
    }

    fn shapes_contain(shapes: &IntShapes<i32>, point: IntPoint) -> bool {
        shapes.iter().any(|shape| {
            shape.first().is_some_and(|hull| hull.contains_point(point))
                && !shape[1..].iter().any(|hole| hole.contains_point(point))
        })
    }

    fn sub_graph(
        min: i32,
        max: i32,
        contours: Vec<Vec<i_float::int::point::IntPoint>>,
    ) -> SubGraph {
        SubGraph {
            range: LineRange::with_min_max(min, max),
            contours,
        }
    }
}
