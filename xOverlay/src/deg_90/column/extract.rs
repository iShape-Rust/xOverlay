use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column::graph::{ColumnGraph, Node};
use crate::deg_90::column::link::LinkIndex;
use alloc::vec;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_shape::int::shape::{IntContour, IntShapes};

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
                    return Some(next);
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

    pub(crate) fn extract(
        &self,
        overlay_rule: OverlayRule,
        visited: &mut Vec<NodeVisitor>,
        points: &mut Vec<IntPoint>,
    ) -> ExtResult {
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

            let result = self.find_sub_result(i, true, overlay_rule, visited, points);
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

    fn find_sub_result(
        &self,
        start: usize,
        dir: bool,
        overlay_rule: OverlayRule,
        visited: &mut Vec<NodeVisitor>,
        points: &mut Vec<IntPoint>,
    ) -> SubResult {
        points.clear();

        let start_node = &self.nodes[start];
        points.push(start_node.point);
        let mut link_index = LinkIndex::Right;
        let start_link = start_node.links[link_index.order()];
        let mut next_node_index = start_link.index();
        while next_node_index != start {
            if let Some(next_link_index) =
                visited[next_node_index].visit_and_next(link_index.opposite(), dir)
            {
                let next_node = &self.nodes[next_node_index];
                points.add_skipping_vertical(next_node.point);

                let next_link = next_node.links[next_link_index.order()];
                next_node_index = next_link.index();
                link_index = next_link_index;
            }
        }

        points.remove_last_if_vertical();

        // a closed contour
        let mut contour = points.to_vec();
        if overlay_rule.is_fill_top(start_link.fill()) {
            SubResult::Hull(contour)
        } else {
            contour[1..].reverse();
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
            return;
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

#[cfg(test)]
mod tests {
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay_rule::OverlayRule;
    use crate::deg_90::column::build::ScanBuffer;
    use crate::deg_90::column::graph::ColumnGraph;
    use crate::deg_90::column::link::LinkIndex;
    use crate::deg_90::column_map::ColumnMap;
    use crate::deg_90::config::ColumnConfig90;
    use alloc::vec;
    use alloc::vec::Vec;
    use i_float::int::point::IntPoint;
    use i_shape::int::area::Area;
    use i_shape::int::path::ContourExtension;
    use i_shape::int::shape::{IntContour, IntShape, IntShapes};
    use i_shape::int_shape;
    use rand::rngs::StdRng;
    use rand::{RngExt, SeedableRng};

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
        let result = graph.extract(OverlayRule::Subject, &mut visited, &mut points);

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
        let result = graph.extract(OverlayRule::Subject, &mut visited, &mut points);

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
        let result = graph.extract(OverlayRule::Subject, &mut visited, &mut points);

        assert_eq!(result.shapes.len(), 1);
        assert_eq!(result.shapes[0].len(), 2);
        assert_eq!(result.shapes[0][0].len(), 4);
        assert_eq!(result.shapes[0][1].len(), 4);
        assert!(result.shapes[0][0].area_two() > 0);
        assert!(result.shapes[0][1].area_two() < 0);
        assert_eq!(result.shapes.area_two(), 168);
    }

    #[test]
    fn test_clip_inside_subject_is_hull() {
        let subj = int_shape![[[-10, -10], [10, -10], [10, 10], [-10, 10]]];
        let clip = int_shape![[[-4, -4], [4, -4], [4, 4], [-4, 4]]];

        let actual = extract_one_column(&subj, &clip, FillRule::NonZero, OverlayRule::Clip);

        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].len(), 1);
        assert_eq!(actual.area_two(), 128);
    }

    #[test]
    fn test_empty_positive_result() {
        let clockwise = int_shape![[[-5, -5], [-5, 5], [5, 5], [5, -5]]];

        let actual = extract_one_column(&clockwise, &[], FillRule::Positive, OverlayRule::Subject);

        assert!(actual.is_empty());
    }

    #[test]
    fn test_many_holes_against_i_overlay() {
        let mut subj = int_shape![[[-50, -50], [50, -50], [50, 50], [-50, 50]]];

        for row in 0..4 {
            for column in 0..4 {
                let x = -40 + column * 20;
                let y = -40 + row * 20;
                subj.push(vec![
                    IntPoint::new(x, y),
                    IntPoint::new(x, y + 8),
                    IntPoint::new(x + 8, y + 8),
                    IntPoint::new(x + 8, y),
                ]);
            }
        }

        let actual = extract_one_column(&subj, &[], FillRule::NonZero, OverlayRule::Subject);
        let expected = i_overlay_shapes(&subj, &[], FillRule::NonZero, OverlayRule::Subject);

        assert_eq!(actual.area_two(), expected.area_two());
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].len(), 17);
        assert!(actual[0][1..].iter().all(|hole| hole.area_two() < 0));
    }

    #[test]
    fn test_random_rectangles_against_i_overlay() {
        let mut rng = StdRng::seed_from_u64(0xA11C_E5E5_90);

        for case_index in 0..512 {
            let subj = random_rectangles(&mut rng, 6);
            let clip = random_rectangles(&mut rng, 6);

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
                    let actual = extract_one_column(&subj, &clip, fill_rule, overlay_rule);
                    let expected = i_overlay_shapes(&subj, &clip, fill_rule, overlay_rule);

                    assert_same_raster(
                        &actual,
                        &expected,
                        &subj,
                        &clip,
                        case_index,
                        fill_rule,
                        overlay_rule,
                    );

                    assert_eq!(
                        actual.area_two(),
                        expected.area_two(),
                        "area mismatch: case={case_index}, fill={fill_rule:?}, overlay={overlay_rule:?}, subj={subj:?}, clip={clip:?}, actual={actual:?}, expected={expected:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_large_random_rectangles_against_i_overlay() {
        let profiles = [
            (100, 128, 20),
            (250, 96, 32),
            (500, 72, 40),
            (1_000, 56, 48),
        ];

        for (profile_index, (rect_count, extent, max_size)) in profiles.into_iter().enumerate() {
            let geometry_seed = 0x5EED_90_0000_u64 ^ profile_index as u64;
            let mut rng = StdRng::seed_from_u64(geometry_seed);
            let subj_count = rect_count / 2;
            let clip_count = rect_count - subj_count;
            let subj = random_rectangles_in_area(&mut rng, subj_count, extent, max_size);
            let clip = random_rectangles_in_area(&mut rng, clip_count, extent, max_size);

            let mut comparison_index = 0_u64;
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
                    let actual = extract_one_column(&subj, &clip, fill_rule, overlay_rule);
                    let expected = i_overlay_shapes(&subj, &clip, fill_rule, overlay_rule);

                    assert_eq!(
                        actual.area_two(),
                        expected.area_two(),
                        "area mismatch: profile={profile_index}, rectangles={rect_count}, geometry_seed={geometry_seed:#x}, fill={fill_rule:?}, overlay={overlay_rule:?}"
                    );
                    assert_valid_directions(
                        &actual,
                        profile_index,
                        rect_count,
                        fill_rule,
                        overlay_rule,
                    );
                    assert_same_random_samples(
                        &actual,
                        &expected,
                        &subj,
                        &clip,
                        geometry_seed ^ comparison_index.rotate_left(17),
                        profile_index,
                        rect_count,
                        fill_rule,
                        overlay_rule,
                    );

                    comparison_index += 1;
                }
            }
        }
    }

    fn random_rectangles(rng: &mut StdRng, count: usize) -> IntShape<i32> {
        random_rectangles_in_area(rng, count, 8, 8)
    }

    fn random_rectangles_in_area(
        rng: &mut StdRng,
        count: usize,
        extent: i32,
        max_size: i32,
    ) -> IntShape<i32> {
        let mut contours = Vec::with_capacity(count);
        let reverse_even = rng.random_range(0..2) == 0;

        for index in 0..count {
            let x = rng.random_range(-extent..extent) * 2;
            let y = rng.random_range(-extent..extent) * 2;
            let width = rng.random_range(1..max_size) * 2;
            let height = rng.random_range(1..max_size) * 2;

            let mut contour = vec![
                IntPoint::new(x, y),
                IntPoint::new(x + width, y),
                IntPoint::new(x + width, y + height),
                IntPoint::new(x, y + height),
            ];

            // Guarantee that every large case contains both contour directions,
            // while the random phase keeps their assignment seed-dependent.
            if (index % 2 == 0) == reverse_even {
                contour.reverse();
            }

            contours.push(contour);
        }

        contours
    }

    fn assert_valid_directions(
        shapes: &IntShapes<i32>,
        profile_index: usize,
        rect_count: usize,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        for (shape_index, shape) in shapes.iter().enumerate() {
            assert!(
                shape.first().is_some_and(|hull| hull.area_two() > 0),
                "invalid hull direction: profile={profile_index}, rectangles={rect_count}, shape={shape_index}, fill={fill_rule:?}, overlay={overlay_rule:?}"
            );
            assert!(
                shape[1..].iter().all(|hole| hole.area_two() < 0),
                "invalid hole direction: profile={profile_index}, rectangles={rect_count}, shape={shape_index}, fill={fill_rule:?}, overlay={overlay_rule:?}"
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn assert_same_random_samples(
        actual: &IntShapes<i32>,
        expected: &IntShapes<i32>,
        subj: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        sample_seed: u64,
        profile_index: usize,
        rect_count: usize,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        let (min_x, min_y, max_x, max_y) = input_bounds(subj, clip);
        let mut rng = StdRng::seed_from_u64(sample_seed);

        for sample_index in 0..256 {
            // Rectangle coordinates are even, so odd probes never hit a boundary.
            let x = 2 * rng.random_range((min_x / 2 - 1)..(max_x / 2 + 1)) + 1;
            let y = 2 * rng.random_range((min_y / 2 - 1)..(max_y / 2 + 1)) + 1;
            let point = IntPoint::new(x, y);

            assert_eq!(
                shapes_contain(actual, point),
                shapes_contain(expected, point),
                "sample mismatch at {point:?}: profile={profile_index}, rectangles={rect_count}, sample={sample_index}, sample_seed={sample_seed:#x}, fill={fill_rule:?}, overlay={overlay_rule:?}"
            );
        }
    }

    fn input_bounds(subj: &[IntContour<i32>], clip: &[IntContour<i32>]) -> (i32, i32, i32, i32) {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for contour in subj.iter().chain(clip) {
            for &p in contour {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }

        (min_x, min_y, max_x, max_y)
    }

    fn extract_one_column(
        subj: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> IntShapes<i32> {
        let config = ColumnConfig90 {
            min_columns_count: 1,
            min_column_width_power: 20,
            max_allow_segments_per_column: 1_000_000_000,
            min_allowed_segments_per_column: 1_000_000,
            max_allowed_segments_per_line: 1_000_000,
        };

        let mut map = ColumnMap::with_columns_count(subj, clip, 1);
        let column = &mut map.columns[0];
        column.test_partition(config);

        let mut buffer = ScanBuffer::with_capacity(32);
        let graph = ColumnGraph::new(column, fill_rule, overlay_rule, &mut buffer);
        let result = graph.extract(overlay_rule, &mut Vec::new(), &mut Vec::new());
        assert!(result.subpaths.is_empty());
        result.shapes
    }

    fn i_overlay_shapes(
        subj: &[IntContour<i32>],
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

        let mut overlay = IOverlay::with_contours(subj, clip);
        overlay.overlay(i_overlay_rule, i_fill_rule)
    }

    fn assert_same_raster(
        actual: &IntShapes<i32>,
        expected: &IntShapes<i32>,
        subj: &[IntContour<i32>],
        clip: &[IntContour<i32>],
        case_index: usize,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for contour in subj.iter().chain(clip) {
            for &p in contour {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }

        for y in ((min_y - 1)..=(max_y + 1)).step_by(2) {
            for x in ((min_x - 1)..=(max_x + 1)).step_by(2) {
                let point = IntPoint::new(x, y);
                assert_eq!(
                    shapes_contain(actual, point),
                    shapes_contain(expected, point),
                    "raster mismatch at {point:?}: case={case_index}, fill={fill_rule:?}, overlay={overlay_rule:?}, subj={subj:?}, clip={clip:?}"
                );
            }
        }
    }

    fn shapes_contain(shapes: &IntShapes<i32>, point: IntPoint) -> bool {
        shapes.iter().any(|shape| {
            shape.first().is_some_and(|hull| hull.contains_point(point))
                && !shape[1..].iter().any(|hole| hole.contains_point(point))
        })
    }

    fn first_column_graph_with_columns_count(
        contours: &IntShape<i32>,
        columns_count: usize,
    ) -> ColumnGraph {
        let config = ColumnConfig90 {
            min_columns_count: 1,
            min_column_width_power: 20,
            max_allow_segments_per_column: 1000_000_000,
            min_allowed_segments_per_column: 1_000_000,
            max_allowed_segments_per_line: 1000_000,
        };

        let mut buffer = ScanBuffer::with_capacity(16);

        let mut map = ColumnMap::with_columns_count(contours, &[], columns_count);
        debug_assert!(map.columns.len() >= 1);

        let column = &mut map.columns[0];
        column.test_partition(config);
        let fill_rule = FillRule::NonZero;
        let overlay_rule = OverlayRule::Subject;

        ColumnGraph::new(column, fill_rule, overlay_rule, &mut buffer)
    }
}
