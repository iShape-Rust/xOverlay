use crate::core::integer::OverlayInt;
use crate::deg_90::sub_graph::SubGraph;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use core::mem;
use i_float::int::point::IntPoint;
use i_key_sort::sort::two_keys::TwoKeysSort;
use i_shape::int::shape::IntContour;
#[cfg(feature = "allow_multithreading")]
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

pub(super) trait Merge<I: OverlayInt> {
    fn merge(self) -> SubGraph<I>;
}

#[cfg(feature = "allow_multithreading")]
pub(super) trait ParallelMerge<I: OverlayInt> {
    fn parallel_merge(self) -> SubGraph<I>;
}

impl<I: OverlayInt> Merge<I> for Vec<SubGraph<I>> {
    fn merge(self) -> SubGraph<I> {
        let mut iter = self.into_iter();
        let Some(mut result) = iter.next() else {
            return SubGraph {
                range: LineRange::default(),
                chunks: Default::default(),
            };
        };

        for right in iter {
            merge_pair(&mut result, right);
        }

        result
    }
}

#[cfg(feature = "allow_multithreading")]
impl<I: OverlayInt> ParallelMerge<I> for Vec<SubGraph<I>> {
    fn parallel_merge(self) -> SubGraph<I> {
        let mut groups = self;

        while groups.len() > 1 {
            groups = groups.into_par_iter().chunks(2).map(Merge::merge).collect();
        }

        groups.merge()
    }
}

fn merge_pair<I: OverlayInt>(left: &mut SubGraph<I>, right: SubGraph<I>) {
    debug_assert!(
        left.range.max == right.range.min,
        "only neighboring column groups can be merged"
    );

    let border_x = left.range.max;
    let mut left_border = mem::take(&mut left.chunks.right);
    let mut merged = mem::take(&mut left.chunks.both);
    left_border.append(&mut merged);

    let mut right_border = right.chunks.left;
    let mut right_both = right.chunks.both;
    right_border.append(&mut right_both);

    left.chunks.middle.extend(right.chunks.middle);
    left.chunks.right = right.chunks.right;

    merge_border_contours(left_border, right_border, border_x, &mut merged);
    left.range.max = right.range.max;
    left.append_classified(merged);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
struct BorderNode<I: OverlayInt> {
    y: I,
    contour_index: usize,
    position: usize,
    side: Side,
}

fn merge_border_contours<I: OverlayInt>(
    mut left: Vec<IntContour<I>>,
    mut right: Vec<IntContour<I>>,
    border_x: I,
    merged: &mut Vec<IntContour<I>>,
) {
    let left_count = left.len();
    left.append(&mut right);
    let contours = left;

    let mut nodes = Vec::new();
    let mut arc_end = Vec::new();
    for (contour_index, contour) in contours.iter().enumerate() {
        let count = contour.len();
        debug_assert!(
            count >= 4,
            "a merged contour must have at least four points"
        );

        let side = if contour_index < left_count {
            Side::Left
        } else {
            Side::Right
        };
        let group_start = nodes.len();
        for position in 0..count {
            let point = contour[position];
            if point.x != border_x {
                continue;
            }

            let prev = contour[(position + count - 1) % count];
            let next = contour[(position + 1) % count];
            let prev_off_border = prev.y == point.y && prev.x != border_x;
            let next_off_border = next.y == point.y && next.x != border_x;
            if !prev_off_border && !next_off_border {
                continue;
            }
            debug_assert_ne!(
                prev_off_border, next_off_border,
                "a border portal must have exactly one off-border edge"
            );

            nodes.push(BorderNode {
                y: point.y,
                contour_index,
                position,
                side,
            });
            arc_end.push(usize::MAX);
        }

        let group = group_start..nodes.len();
        debug_assert_eq!(group.len() & 1, 0, "a contour must have paired portals");
        for offset in group.clone() {
            if is_arc_start(&nodes[offset], &contours, border_x) {
                let end_index = if offset + 1 < group.end {
                    offset + 1
                } else {
                    group.start
                };
                debug_assert!(
                    !is_arc_start(&nodes[end_index], &contours, border_x),
                    "contour portals must alternate between start and end"
                );
                arc_end[offset] = end_index;
            }
        }
    }

    if nodes.is_empty() {
        merged.extend(contours);
        return;
    }
    debug_assert_eq!(nodes.len() & 1, 0, "border nodes must form pairs");

    let mut border_order = (0..nodes.len()).collect::<Vec<_>>();
    border_order.sort_by_two_keys(
        false,
        |index| nodes[*index].y,
        |index| match nodes[*index].side {
            Side::Left => 0,
            Side::Right => 1,
        },
    );

    let mut border_next = Vec::<mem::MaybeUninit<usize>>::with_capacity(nodes.len());
    // `MaybeUninit<usize>` is valid without initialization. Only end-portal slots are written
    // below, and traversal only reads slots obtained from `arc_end`.
    unsafe {
        border_next.set_len(nodes.len());
    }
    for pair in border_order.chunks_exact(2) {
        let a = pair[0];
        let b = pair[1];
        let a_is_start = is_arc_start(&nodes[a], &contours, border_x);
        let b_is_start = is_arc_start(&nodes[b], &contours, border_x);
        debug_assert_ne!(
            a_is_start, b_is_start,
            "each border pair must contain one contour start and one contour end"
        );

        let (end, start) = if a_is_start { (b, a) } else { (a, b) };
        border_next[end].write(start);
    }

    let mut visited = alloc::vec![false; nodes.len()];
    let mut contour = Vec::new();
    for start in 0..nodes.len() {
        if !is_arc_start(&nodes[start], &contours, border_x) || visited[start] {
            continue;
        }
        contour.clear();
        let mut current = start;
        for _ in 0..nodes.len() {
            debug_assert!(!visited[current], "portal cycle closes at the wrong start");
            visited[current] = true;

            let end = arc_end[current];
            debug_assert_ne!(end, usize::MAX, "contour arc has no end portal");
            append_contour_arc(
                &contours[nodes[current].contour_index],
                nodes[current].position,
                nodes[end].position,
                &mut contour,
            );

            // Every arc end belongs to exactly one border pair initialized above.
            current = unsafe { border_next[end].assume_init() };
            if current == start {
                break;
            }
        }
        debug_assert_eq!(current, start, "portal traversal must close at its start");

        close_contour(&mut contour);
        debug_assert!(is_simple_contour(&contour));
        debug_assert!(
            contour.len() >= 4,
            "a merged contour must have at least four points"
        );
        merged.push(contour.to_vec());
    }
}

#[inline(always)]
fn is_arc_start<I: OverlayInt>(
    node: &BorderNode<I>,
    contours: &[IntContour<I>],
    border_x: I,
) -> bool {
    let contour = &contours[node.contour_index];
    contour[(node.position + 1) % contour.len()].x != border_x
}

#[inline(always)]
fn append_contour_arc<I: OverlayInt>(
    contour: &IntContour<I>,
    start: usize,
    end: usize,
    result: &mut IntContour<I>,
) {
    debug_assert!(is_simple_arc(contour, start, end));
    debug_assert_ne!(start, end, "a contour arc must contain an edge");

    let mut first = start;
    if result.last() == Some(&contour[start]) {
        debug_assert!(result.len() >= 2, "an accumulated arc must contain an edge");
        let a = result[result.len() - 2];
        let b = result[result.len() - 1];
        first = (start + 1) % contour.len();
        debug_assert!(
            is_collinear(a, b, contour[first]),
            "a shared border portal must join collinear edges"
        );
        result.pop();
    }

    if first <= end {
        result.extend_from_slice(&contour[first..=end]);
    } else {
        result.extend_from_slice(&contour[first..]);
        result.extend_from_slice(&contour[..=end]);
    }
}

fn close_contour<I: OverlayInt>(contour: &mut IntContour<I>) {
    while contour.len() > 1 && contour.first() == contour.last() {
        contour.pop();
    }

    loop {
        let n = contour.len();
        if n < 3 {
            break;
        }
        if is_collinear(contour[n - 1], contour[0], contour[1]) {
            contour.remove(0);
        } else if is_collinear(contour[n - 2], contour[n - 1], contour[0]) {
            contour.pop();
        } else {
            break;
        }
    }
}

#[cfg(debug_assertions)]
fn is_simple_arc<I: OverlayInt>(contour: &[IntPoint<I>], start: usize, end: usize) -> bool {
    let count = contour.len();
    let arc_len = if start <= end {
        end - start + 1
    } else {
        count - start + end + 1
    };

    for offset in 1..arc_len {
        let previous = contour[(start + offset - 1) % count];
        let point = contour[(start + offset) % count];
        if previous == point {
            return false;
        }
    }

    for offset in 2..arc_len {
        let a = contour[(start + offset - 2) % count];
        let b = contour[(start + offset - 1) % count];
        let c = contour[(start + offset) % count];
        if is_collinear(a, b, c) {
            return false;
        }
    }

    true
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn is_simple_arc<I: OverlayInt>(_: &[IntPoint<I>], _: usize, _: usize) -> bool {
    true
}

#[cfg(debug_assertions)]
fn is_simple_contour<I: OverlayInt>(contour: &[IntPoint<I>]) -> bool {
    let count = contour.len();
    if count < 3 {
        return true;
    }

    for index in 0..count {
        let a = contour[index];
        let b = contour[(index + 1) % count];
        let c = contour[(index + 2) % count];
        if a == b || is_collinear(a, b, c) {
            return false;
        }
    }

    true
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn is_simple_contour<I: OverlayInt>(_: &[IntPoint<I>]) -> bool {
    true
}

#[inline(always)]
fn is_collinear<I: OverlayInt>(a: IntPoint<I>, b: IntPoint<I>, c: IntPoint<I>) -> bool {
    (a.x == b.x && b.x == c.x) || (a.y == b.y && b.y == c.y)
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
    use crate::deg_90::column::SolverBuffer;
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;
    use crate::deg_90::sub_graph::{ContourChunk, ContourChunks};
    use crate::geom::range::LineRange;
    use crate::test_utils::i_overlay_shapes;
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_shape::int::area::Area;
    use i_shape::int::path::ContourExtension;
    use i_shape::int::shape::{IntContour, IntShape, IntShapes};
    use i_shape::{int_path, int_shape};
    use rand::rngs::StdRng;
    use rand::{RngExt, SeedableRng};
    #[cfg(feature = "allow_multithreading")]
    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    #[test]
    fn sub_graph_separates_all_four_border_classes() {
        let graph = sub_graph(
            0,
            10,
            vec![
                rectangle(0, 0, 5, 5),
                rectangle(5, 10, 5, 5),
                rectangle(0, 20, 10, 5),
                rectangle(2, 30, 6, 5),
            ],
        );

        assert_eq!(graph.chunks.left.len(), 1);
        assert_eq!(graph.chunks.right.len(), 1);
        assert_eq!(graph.chunks.both.len(), 1);
        assert_eq!(graph.chunks.middle.len(), 1);
        assert_eq!(graph.chunks.middle[0].contours.len(), 1);
        assert!(graph.chunks.middle[0].is_leaf);
        assert_eq!(
            graph.chunks.middle[0].x_range,
            LineRange::with_min_max(0, 10),
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn i64_multi_column_overlay_rebuilds_hole() {
        const BASE: i64 = 5_000_000_000;
        let subject = vec![vec![
            IntPoint::new(BASE, BASE),
            IntPoint::new(BASE + 100, BASE),
            IntPoint::new(BASE + 100, BASE + 100),
            IntPoint::new(BASE, BASE + 100),
        ]];
        let clip = vec![vec![
            IntPoint::new(BASE + 20, BASE + 20),
            IntPoint::new(BASE + 80, BASE + 20),
            IntPoint::new(BASE + 80, BASE + 80),
            IntPoint::new(BASE + 20, BASE + 80),
        ]];
        let options = IntOverlayOptions {
            columns_config: ColumnConfig90::dev(4),
        };

        let shapes =
            Overlay::<i64>::with_contours_custom(&subject, &clip, options, CPUCount::Single)
                .overlay(FillRule::NonZero, OverlayRule::Difference);

        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(shapes.area_two(), 12_800i128);
    }

    #[test]
    fn into_shapes_joins_a_middle_hole_to_an_overlapping_left_chunk() {
        let mut hole = rectangle(10, 10, 20, 20);
        hole.reverse();
        let graph = sub_graph(0, 100, vec![rectangle(0, 0, 60, 60), hole]);

        assert_eq!(graph.chunks.left.len(), 1);
        assert_eq!(graph.chunks.middle.len(), 1);
        assert!(graph.chunks.both.is_empty());

        let shapes = graph.into_shapes();
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(shapes.area_two(), 6_400);
    }

    #[test]
    fn into_shapes_joins_local_holes_to_a_both_shape() {
        let mut left_hole = rectangle(10, 10, 20, 20);
        let mut right_hole = rectangle(70, 70, 20, 20);
        left_hole.reverse();
        right_hole.reverse();
        let graph = sub_graph(
            0,
            100,
            vec![rectangle(0, 0, 100, 100), left_hole, right_hole],
        );

        assert_eq!(graph.chunks.both.len(), 1);
        assert_eq!(graph.chunks.middle.len(), 1);

        let shapes = graph.into_shapes();
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 3);
        assert_eq!(shapes.area_two(), 18_400);
    }

    #[test]
    fn into_shapes_builds_non_leaf_middle_before_leaf_middle() {
        let mut hole = rectangle(20, 20, 20, 20);
        hole.reverse();
        let graph = SubGraph {
            range: LineRange::with_min_max(0, 100),
            chunks: ContourChunks {
                middle: vec![
                    ContourChunk {
                        x_range: LineRange::with_min_max(20, 40),
                        contours: vec![hole],
                        is_leaf: true,
                    },
                    ContourChunk {
                        x_range: LineRange::with_min_max(10, 50),
                        contours: vec![rectangle(10, 10, 40, 40)],
                        is_leaf: false,
                    },
                ],
                ..Default::default()
            },
        };

        let shapes = graph.into_shapes();
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(shapes.area_two(), 2_400);
    }

    #[test]
    fn joins_rectangles_across_seam() {
        let left = sub_graph(
            -10,
            0,
            vec![int_path![[-10, -5], [0, -5], [0, 5], [-10, 5]]],
        );
        let right = sub_graph(0, 10, vec![int_path![[0, -5], [10, -5], [10, 5], [0, 5]]]);

        let contours = vec![left, right].merge().into_contours();
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].len(), 4);
        assert_eq!(contours[0].area_two(), 400);
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
        let serial = serial.into_contours();
        let parallel = parallel.into_contours();
        assert_eq!(parallel, serial);
        assert_eq!(parallel.len(), 1);
        assert_eq!(parallel[0].len(), 4);
        assert_eq!(parallel[0].area_two(), 1_000);
    }

    #[test]
    fn joins_partially_overlapping_seam_intervals() {
        let left = sub_graph(
            -10,
            0,
            vec![int_path![[-10, 0], [0, 0], [0, 10], [-10, 10]]],
        );
        let right = sub_graph(0, 10, vec![int_path![[0, 0], [10, 0], [10, 5], [0, 5]]]);

        let contours = vec![left, right].merge().into_contours();
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].len(), 6);
        assert_eq!(contours[0].area_two(), 300);
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

        let contours = vec![left, right].merge().into_contours();
        assert_eq!(contours.len(), 2);

        let shapes = column::rebuild_shapes(contours);
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
        let map = ColumnMap::<i32>::with_columns_count(&subject, &[], 2);
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
        assert_eq!(shapes.area_two(), 1848i64);
        assert!(shapes[0][0].area_two() > 0i64);
        assert!(shapes[0][1].area_two() < 0i64);
    }

    #[test]
    fn column_graphs_rebuild_a_hole_across_multiple_seams() {
        let subject = int_shape![
            [[-32, -32], [32, -32], [32, 32], [-32, 32]],
            [[-20, -10], [-20, 10], [20, 10], [20, -10]],
        ];
        let map = ColumnMap::<i32>::with_columns_count(&subject, &[], 4);
        assert_eq!(map.columns.len(), 4);

        let parts: Vec<_> = map
            .columns
            .into_iter()
            .map(|column| SubGraph::with_column(column, FillRule::NonZero, OverlayRule::Subject))
            .collect();
        let shapes = parts.merge().into_shapes();

        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(shapes.area_two(), 6592i64);
        assert!(shapes[0][0].area_two() > 0i64);
        assert!(shapes[0][1].area_two() < 0i64);
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
    fn nested_squares_union_matches_i_overlay() {
        const N: usize = 16;
        let (subject, clip) = nested_square_contours(N);
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Union;
        let expected = i_overlay_shapes(&subject, &clip, fill_rule, overlay_rule);

        for columns_count in [1, 2, 4, 8, 16] {
            let actual =
                extract_multi_column(&subject, &clip, columns_count, fill_rule, overlay_rule);

            assert_eq!(
                actual.area_two(),
                expected.area_two(),
                "area mismatch for N={N}, columns={columns_count}"
            );
            assert_eq!(
                actual.len(),
                expected.len(),
                "shape count mismatch for N={N}, columns={columns_count}"
            );
        }
    }

    #[test]
    #[ignore = "performance comparison; run explicitly with --release --ignored --nocapture"]
    fn performance_checkerboard_against_i_overlay() {
        const N: usize = 128;
        const BENCH_TIME: Duration = Duration::from_millis(600);

        // Matches iOverlay's official CheckerboardTest workload:
        // https://ishape-rust.github.io/iShape-js/overlay/performance/performance.html
        let subject = many_squares(IntPoint::new(0, 0), 20, 30, 1, N);
        let clip = many_squares(IntPoint::new(15, 15), 20, 30, 1, N - 1);
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Xor;

        let expected = i_overlay_shapes(&subject, &clip, fill_rule, overlay_rule);
        let one_column = extract_multi_column(&subject, &clip, 1, fill_rule, overlay_rule);
        let two_columns = extract_multi_column(&subject, &clip, 2, fill_rule, overlay_rule);
        let four_columns = extract_multi_column(&subject, &clip, 4, fill_rule, overlay_rule);
        let eight_columns = extract_multi_column(&subject, &clip, 8, fill_rule, overlay_rule);
        let thirty_one_columns = extract_multi_column(&subject, &clip, 31, fill_rule, overlay_rule);
        let sixty_two_columns = extract_multi_column(&subject, &clip, 62, fill_rule, overlay_rule);
        assert_eq!(one_column.area_two(), expected.area_two());
        assert_eq!(two_columns.area_two(), expected.area_two());
        assert_eq!(four_columns.area_two(), expected.area_two());
        assert_eq!(eight_columns.area_two(), expected.area_two());
        assert_eq!(thirty_one_columns.area_two(), expected.area_two());
        assert_eq!(sixty_two_columns.area_two(), expected.area_two());

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
        let x_thirty_one = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 31, fill_rule, overlay_rule)
        });
        let x_sixty_two = measure_for(BENCH_TIME, || {
            extract_multi_column(&subject, &clip, 62, fill_rule, overlay_rule)
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
        print_measurement("xOverlay columns=31", x_thirty_one);
        print_measurement("xOverlay columns=62", x_sixty_two);
        print_measurement("iOverlay", i_overlay);
        std::println!(
            "vs iOverlay: columns(1)={:.2}x, columns(2)={:.2}x, columns(4)={:.2}x, columns(8)={:.2}x, columns(31)={:.2}x, columns(62)={:.2}x",
            i_overlay.ns_per_iteration() / x_one.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_two.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_four.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_eight.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_thirty_one.ns_per_iteration(),
            i_overlay.ns_per_iteration() / x_sixty_two.ns_per_iteration(),
        );
    }

    #[test]
    #[ignore = "single-thread profiling workload; run explicitly with --release --ignored --nocapture"]
    fn profile_checkerboard_single_thread() {
        const N: usize = 128;
        const COLUMNS: usize = 62;
        const PROFILE_TIME: Duration = Duration::from_secs(5);

        let subject = many_squares(IntPoint::new(0, 0), 20, 30, 1, N);
        let clip = many_squares(IntPoint::new(15, 15), 20, 30, 1, N - 1);
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
            let overlay =
                Overlay::<i32>::with_contours_custom(&subject, &clip, options, CPUCount::Single);
            map_time += start.elapsed();
            assert_eq!(overlay.columns.len(), COLUMNS);

            let start = Instant::now();
            let mut buffer = SolverBuffer::default();
            let sub_graphs = overlay
                .columns
                .into_iter()
                .map(|column| {
                    column.test_process(
                        fill_rule,
                        overlay_rule,
                        options.columns_config,
                        &mut buffer,
                    )
                })
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

    #[test]
    #[ignore = "stage profiling workload; run explicitly with --release --ignored --nocapture"]
    fn profile_nested_squares_stages() {
        const N: usize = 4096;
        let (subject, clip) = nested_square_contours(N);
        profile_workload_stages(
            "nested-squares",
            &subject,
            &clip,
            OverlayRule::Xor,
            Duration::from_secs(3),
        );
    }

    #[test]
    #[ignore = "layout diagnostic workload; run explicitly with --release --ignored --nocapture"]
    fn profile_nested_squares_layout_variants() {
        const N: usize = 4096;
        let (subject, clip) = nested_square_contours(N);

        let fixed_density = IntOverlayOptions {
            columns_config: ColumnConfig90 {
                min_columns_count: 1,
                min_column_width_power: 8,
                max_allow_segments_per_column: 1 << 20,
                min_allowed_segments_per_column: 1 << 16,
                max_allowed_segments_per_line: 7,
            },
        };
        profile_workload_stages_with_options(
            "nested-squares/fixed-density",
            &subject,
            &clip,
            OverlayRule::Xor,
            fixed_density,
            Duration::from_secs(2),
        );

        let mut fixed_partition = fixed_density;
        fixed_partition.columns_config.max_allowed_segments_per_line = 128;
        profile_workload_stages_with_options(
            "nested-squares/fixed-density-line128",
            &subject,
            &clip,
            OverlayRule::Xor,
            fixed_partition,
            Duration::from_secs(2),
        );
    }

    #[test]
    #[ignore = "column-count tuning workload; run explicitly with --release --ignored --nocapture"]
    fn profile_nested_squares_column_count_sweep() {
        const N: usize = 4096;
        let (subject, clip) = nested_square_contours(N);

        for columns_count in [2, 4, 8, 16, 32, 64, 128, 256] {
            let options = multi_column_options(&subject, &clip, columns_count);
            for cpu_count in [CPUCount::Single, CPUCount::Auto] {
                let start = Instant::now();
                let overlay =
                    Overlay::<i32>::with_contours_custom(&subject, &clip, options, cpu_count);
                let actual_columns = overlay.columns.len();
                black_box(overlay.overlay(FillRule::NonZero, OverlayRule::Xor));
                std::println!(
                    "nested column sweep: requested={columns_count:>3}, actual={actual_columns:>3}, cpu={cpu_count:?}, elapsed={:?}",
                    start.elapsed(),
                );
            }
        }
    }

    #[test]
    #[ignore = "stage profiling workload; run explicitly with --release --ignored --nocapture"]
    fn profile_windows_stages() {
        const N: usize = 128;
        let (subject, clip) = window_contours(N);
        profile_workload_stages(
            "windows",
            &subject,
            &clip,
            OverlayRule::Difference,
            Duration::from_secs(3),
        );
    }

    #[test]
    #[ignore = "layout diagnostic workload; run explicitly with --release --ignored --nocapture"]
    fn profile_windows_layout_variants() {
        const N: usize = 128;
        let (subject, clip) = window_contours(N);

        for max_allowed_segments_per_line in [7, 128] {
            let options = IntOverlayOptions {
                columns_config: ColumnConfig90 {
                    min_columns_count: 1,
                    min_column_width_power: 8,
                    max_allow_segments_per_column: 1 << 20,
                    min_allowed_segments_per_column: 1 << 16,
                    max_allowed_segments_per_line,
                },
            };
            profile_workload_stages_with_options(
                if max_allowed_segments_per_line == 7 {
                    "windows/fixed-density"
                } else {
                    "windows/fixed-density-line128"
                },
                &subject,
                &clip,
                OverlayRule::Difference,
                options,
                Duration::from_secs(2),
            );
        }
    }

    fn profile_workload_stages(
        name: &str,
        subject: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        overlay_rule: OverlayRule,
        profile_time: Duration,
    ) {
        profile_workload_stages_with_options(
            name,
            subject,
            clip,
            overlay_rule,
            IntOverlayOptions::default(),
            profile_time,
        );
    }

    fn profile_workload_stages_with_options(
        name: &str,
        subject: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        overlay_rule: OverlayRule,
        options: IntOverlayOptions,
        profile_time: Duration,
    ) {
        for cpu_count in [CPUCount::Single, CPUCount::Auto] {
            let mut map_time = Duration::ZERO;
            let mut column_time = Duration::ZERO;
            let mut merge_time = Duration::ZERO;
            let mut rebuild_time = Duration::ZERO;
            let profile_start = Instant::now();
            let mut iterations = 0;
            let mut columns_count = 0;

            while iterations == 0 || profile_start.elapsed() < profile_time {
                let start = Instant::now();
                let overlay =
                    Overlay::<i32>::with_contours_custom(subject, clip, options, cpu_count);
                map_time += start.elapsed();
                columns_count = overlay.columns.len();

                let start = Instant::now();
                #[cfg(feature = "allow_multithreading")]
                let sub_graphs = if cpu_count.is_parallel() {
                    overlay
                        .columns
                        .into_par_iter()
                        .map_init(SolverBuffer::default, |buffer, column| {
                            column.test_process(
                                FillRule::NonZero,
                                overlay_rule,
                                options.columns_config,
                                buffer,
                            )
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut buffer = SolverBuffer::default();
                    overlay
                        .columns
                        .into_iter()
                        .map(|column| {
                            column.test_process(
                                FillRule::NonZero,
                                overlay_rule,
                                options.columns_config,
                                &mut buffer,
                            )
                        })
                        .collect::<Vec<_>>()
                };
                #[cfg(not(feature = "allow_multithreading"))]
                let sub_graphs = {
                    let mut buffer = SolverBuffer::default();
                    overlay
                        .columns
                        .into_iter()
                        .map(|column| {
                            column.test_process(
                                FillRule::NonZero,
                                overlay_rule,
                                options.columns_config,
                                &mut buffer,
                            )
                        })
                        .collect::<Vec<_>>()
                };
                column_time += start.elapsed();

                let start = Instant::now();
                #[cfg(feature = "allow_multithreading")]
                let merged = if cpu_count.is_parallel() {
                    sub_graphs.parallel_merge()
                } else {
                    sub_graphs.merge()
                };
                #[cfg(not(feature = "allow_multithreading"))]
                let merged = sub_graphs.merge();
                merge_time += start.elapsed();

                let start = Instant::now();
                #[cfg(feature = "allow_multithreading")]
                if cpu_count.is_parallel() {
                    black_box(merged.parallel_into_shapes());
                } else {
                    black_box(merged.into_shapes());
                }
                #[cfg(not(feature = "allow_multithreading"))]
                black_box(merged.into_shapes());
                rebuild_time += start.elapsed();
                iterations += 1;
            }

            let measured = map_time + column_time + merge_time + rebuild_time;
            std::println!(
                "{name}: cpu={cpu_count:?}, columns={columns_count}, iterations={iterations}, measured={measured:?}"
            );
            print_profile_stage("column map", map_time, measured, iterations);
            print_profile_stage("column process", column_time, measured, iterations);
            print_profile_stage("merge", merge_time, measured, iterations);
            print_profile_stage("rebuild shapes", rebuild_time, measured, iterations);
        }
    }

    fn window_contours(n: usize) -> (IntShape<i32>, IntShape<i32>) {
        let origin = -(n as i32) * 30 / 2;
        let mut subject = Vec::with_capacity(n * n);
        let mut clip = Vec::with_capacity(n * n);
        for row in 0..n {
            let y = origin + row as i32 * 30;
            for column in 0..n {
                let x = origin + column as i32 * 30;
                subject.push(rectangle(x, y, 20, 20));
                clip.push(rectangle(x + 5, y + 5, 10, 10));
            }
        }
        (subject, clip)
    }

    fn nested_square_contours(n: usize) -> (IntShape<i32>, IntShape<i32>) {
        let mut subject = Vec::with_capacity(2 * n);
        let mut clip = Vec::with_capacity(2 * n);
        let mut radius = 8;
        for _ in 0..n {
            clip.push(rectangle(-radius, radius - 4, 2 * radius, 4));
            clip.push(rectangle(-radius, -radius, 2 * radius, 4));
            subject.push(rectangle(-radius, -radius, 4, 2 * radius));
            subject.push(rectangle(radius - 4, -radius, 4, 2 * radius));
            radius += 8;
        }
        (subject, clip)
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

    fn many_squares(
        start: IntPoint,
        size: i32,
        offset: i32,
        row_shift: i32,
        n: usize,
    ) -> IntShape<i32> {
        let mut contours = Vec::with_capacity(n * n);
        let mut y = start.y;
        for row in 0..n {
            let mut x = start.x + row as i32 * row_shift;
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
        let overlay = Overlay::<i32>::with_contours_custom(
            subject,
            clip,
            options,
            CPUCount::Fixed(columns_count),
        );
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
    ) -> SubGraph<i32> {
        SubGraph::with_contours(LineRange::with_min_max(min, max), contours)
    }
}
