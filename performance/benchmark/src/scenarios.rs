use crate::model::{Case, Contour, Operation, ScenarioInfo};
use x_overlay::i_float::int::number::int::IntNumber;
use x_overlay::i_float::int::point::IntPoint;

pub const SCENARIOS: [&str; 9] = [
    "checkerboard",
    "not_overlap",
    "lines_net",
    "wind_mill",
    "cross",
    "tetris_square",
    "windows",
    "nested_squares",
    "sieve",
];

pub trait Coordinate: IntNumber + Copy {
    fn from_i64(value: i64) -> Self;
}

impl Coordinate for i32 {
    fn from_i64(value: i64) -> Self {
        i32::try_from(value).expect("scenario coordinate must fit i32")
    }
}

impl Coordinate for i64 {
    fn from_i64(value: i64) -> Self {
        value
    }
}

pub fn scenario_info() -> Vec<ScenarioInfo> {
    vec![
        info(
            "checkerboard",
            "Checkerboard",
            "Two interleaved square grids with regular overlap.",
            Operation::Xor,
        ),
        info(
            "not_overlap",
            "Not overlap",
            "Two interleaved grids whose squares never intersect.",
            Operation::Union,
        ),
        info(
            "lines_net",
            "Lines net",
            "A dense grid formed by intersecting horizontal and vertical bars.",
            Operation::Intersect,
        ),
        info(
            "wind_mill",
            "Orthogonal wind mill",
            "A repeated four-fold pinwheel made only from orthogonal blades.",
            Operation::Difference,
        ),
        info(
            "cross",
            "Cross",
            "Two overlapping rectangles with square holes near four arm ends and at the center.",
            Operation::Difference,
        ),
        info(
            "tetris_square",
            "Tetris square",
            "Four diagonally paired L tetrominoes form a square frame around a 3x3-cell hole.",
            Operation::Union,
        ),
        info(
            "windows",
            "Windows",
            "A grid of square frames, each producing one hole.",
            Operation::Difference,
        ),
        info(
            "nested_squares",
            "Nested squares",
            "Union of concentric square rails with deep shape nesting.",
            Operation::Union,
        ),
        info(
            "sieve",
            "Sieve",
            "One large square perforated by a regular grid of square holes.",
            Operation::Difference,
        ),
    ]
}

fn info(id: &str, label: &str, description: &str, operation: Operation) -> ScenarioInfo {
    ScenarioInfo {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        operation,
        illustration: format!("assets/{id}.svg"),
    }
}

pub fn preview_n(scenario: &str) -> usize {
    match scenario {
        "checkerboard" | "not_overlap" | "wind_mill" | "cross" | "tetris_square" | "windows" => 3,
        "lines_net" | "nested_squares" | "sieve" => 5,
        _ => panic!("unknown scenario: {scenario}"),
    }
}

pub fn smoke_sizes(scenario: &str) -> Vec<usize> {
    match scenario {
        "wind_mill" | "cross" | "tetris_square" => vec![1, 2],
        "checkerboard" | "not_overlap" | "lines_net" | "windows" | "nested_squares" | "sieve" => {
            vec![2, 4]
        }
        _ => panic!("unknown scenario: {scenario}"),
    }
}

pub fn full_sizes(scenario: &str) -> Vec<usize> {
    sizes_from_max(max_n(scenario))
}

pub fn max_n(scenario: &str) -> usize {
    match scenario {
        "checkerboard" => 2048,
        "not_overlap" => 2048,
        "lines_net" => 4096,
        "wind_mill" => 1024,
        "cross" => 1024,
        "tetris_square" => 1024,
        "windows" => 2048,
        "nested_squares" => 16_384,
        "sieve" => 1024,
        _ => panic!("unknown scenario: {scenario}"),
    }
}

fn sizes_from_max(max_n: usize) -> Vec<usize> {
    let mut sizes = [64, 16, 4, 1]
        .into_iter()
        .filter_map(|divisor| {
            let value = max_n / divisor;
            (value > 0).then_some(value)
        })
        .collect::<Vec<_>>();
    sizes.sort_unstable();
    sizes.dedup();
    sizes
}

pub fn make_i32(scenario: &str, n: usize) -> Case<i32> {
    make_case::<i32>(scenario, n, 0)
}

pub fn make_i64(scenario: &str, n: usize) -> Case<i64> {
    make_case::<i64>(scenario, n, 1_i64 << 40)
}

pub fn make_preview(scenario: &str) -> Case<i64> {
    make_case::<i64>(scenario, preview_n(scenario), 0)
}

fn make_case<I: Coordinate>(scenario: &str, n: usize, translation: i64) -> Case<I> {
    assert!(n > 0, "scenario size must be positive");
    let mut case = match scenario {
        "checkerboard" => checkerboard(n),
        "not_overlap" => not_overlap(n),
        "lines_net" => lines_net(n),
        "wind_mill" => wind_mill(n),
        "cross" => cross(n),
        "tetris_square" => tetris_square(n),
        "windows" => windows(n),
        "nested_squares" => nested_squares(n),
        "sieve" => sieve(n),
        _ => panic!("unknown scenario: {scenario}"),
    };
    if translation != 0 {
        translate(&mut case.subject, translation, translation);
        translate(&mut case.clip, translation, translation);
    }
    convert(case)
}

fn convert<I: Coordinate>(case: Case<i64>) -> Case<I> {
    Case {
        scenario: case.scenario,
        label: case.label,
        operation: case.operation,
        n: case.n,
        subject: convert_contours(case.subject),
        clip: convert_contours(case.clip),
    }
}

fn convert_contours<I: Coordinate>(contours: Vec<Contour<i64>>) -> Vec<Contour<I>> {
    contours
        .into_iter()
        .map(|contour| {
            contour
                .into_iter()
                .map(|point| IntPoint::new(I::from_i64(point.x), I::from_i64(point.y)))
                .collect()
        })
        .collect()
}

fn translate(contours: &mut [Contour<i64>], dx: i64, dy: i64) {
    for point in contours.iter_mut().flatten() {
        point.x += dx;
        point.y += dy;
    }
}

fn checkerboard(n: usize) -> Case<i64> {
    Case {
        scenario: "checkerboard",
        label: "Checkerboard",
        operation: Operation::Xor,
        n,
        subject: square_grid(n, 20, 30),
        clip: square_grid(n.saturating_sub(1), 20, 30),
    }
}

fn not_overlap(n: usize) -> Case<i64> {
    Case {
        scenario: "not_overlap",
        label: "Not overlap",
        operation: Operation::Union,
        n,
        subject: square_grid(n, 10, 30),
        clip: square_grid(n.saturating_sub(1), 10, 30),
    }
}

fn square_grid(n: usize, size: i64, step: i64) -> Vec<Contour<i64>> {
    if n == 0 {
        return Vec::new();
    }
    let extent = (n as i64 - 1) * step + size;
    let start = -extent / 2;
    let mut contours = Vec::with_capacity(n * n);
    for row in 0..n {
        for column in 0..n {
            contours.push(rectangle(
                start + column as i64 * step,
                start + row as i64 * step,
                size,
                size,
            ));
        }
    }
    contours
}

fn lines_net(n: usize) -> Case<i64> {
    let spacing = 20;
    let thickness = 10;
    let span = n as i64 * spacing;
    let start = -span / 2;
    let first = -((n as i64 - 1) * spacing) / 2 - thickness / 2;
    let mut vertical = Vec::with_capacity(n);
    let mut horizontal = Vec::with_capacity(n);
    for index in 0..n {
        let offset = first + index as i64 * spacing;
        vertical.push(rectangle(offset, start, thickness, span));
        horizontal.push(rectangle(start, offset, span, thickness));
    }
    Case {
        scenario: "lines_net",
        label: "Lines net",
        operation: Operation::Intersect,
        n,
        subject: vertical,
        clip: horizontal,
    }
}

fn wind_mill(n: usize) -> Case<i64> {
    let cell = 100;
    let grid_start = -((n as i64 - 1) * cell) / 2;
    let mut subject = Vec::with_capacity(4 * n * n);
    let mut clip = Vec::with_capacity(4 * n * n);
    for row in 0..n {
        for column in 0..n {
            let center = (
                grid_start + column as i64 * cell,
                grid_start + row as i64 * cell,
            );
            for quarter in 0..4 {
                subject.push(transform_contour(
                    &[(5, 5), (30, 5), (30, 15), (15, 15), (15, 30), (5, 30)],
                    center,
                    quarter,
                ));
                clip.push(transform_contour(
                    &[(20, 3), (34, 3), (34, 17), (20, 17)],
                    center,
                    quarter,
                ));
            }
        }
    }
    Case {
        scenario: "wind_mill",
        label: "Orthogonal wind mill",
        operation: Operation::Difference,
        n,
        subject,
        clip,
    }
}

fn transform_contour(points: &[(i64, i64)], center: (i64, i64), quarter: usize) -> Contour<i64> {
    points
        .iter()
        .map(|&(x, y)| {
            let (rx, ry) = match quarter % 4 {
                0 => (x, y),
                1 => (-y, x),
                2 => (-x, -y),
                _ => (y, -x),
            };
            IntPoint::new(center.0 + rx, center.1 + ry)
        })
        .collect()
}

fn cross(n: usize) -> Case<i64> {
    let cell = 80_i64;
    let arm_length = 60_i64;
    let arm_thickness = 20_i64;
    let square_size = 10_i64;
    let half_length = arm_length / 2;
    let half_thickness = arm_thickness / 2;
    let half_square = square_size / 2;
    let end_square_offset = half_length - square_size;
    let grid_start = -((n as i64 - 1) * cell) / 2;
    let mut subject = Vec::with_capacity(2 * n * n);
    let mut clip = Vec::with_capacity(5 * n * n);

    for row in 0..n {
        for column in 0..n {
            let center_x = grid_start + column as i64 * cell;
            let center_y = grid_start + row as i64 * cell;
            subject.push(rectangle(
                center_x - half_length,
                center_y - half_thickness,
                arm_length,
                arm_thickness,
            ));
            subject.push(rectangle(
                center_x - half_thickness,
                center_y - half_length,
                arm_thickness,
                arm_length,
            ));

            for (dx, dy) in [
                (-end_square_offset, 0),
                (end_square_offset, 0),
                (0, -end_square_offset),
                (0, end_square_offset),
                (0, 0),
            ] {
                clip.push(rectangle(
                    center_x + dx - half_square,
                    center_y + dy - half_square,
                    square_size,
                    square_size,
                ));
            }
        }
    }

    Case {
        scenario: "cross",
        label: "Cross",
        operation: Operation::Difference,
        n,
        subject,
        clip,
    }
}

fn tetris_square(n: usize) -> Case<i64> {
    let cell = 70_i64;
    let grid_start = -((n as i64 - 1) * cell) / 2;
    let mut subject = Vec::with_capacity(2 * n * n);
    let mut clip = Vec::with_capacity(2 * n * n);

    for row in 0..n {
        for column in 0..n {
            let center = (
                grid_start + column as i64 * cell,
                grid_start + row as i64 * cell,
            );
            subject.push(offset_contour(
                &[
                    (-25, -25),
                    (5, -25),
                    (5, -15),
                    (-15, -15),
                    (-15, -5),
                    (-25, -5),
                ],
                center,
            ));
            subject.push(offset_contour(
                &[(15, 5), (25, 5), (25, 25), (-5, 25), (-5, 15), (15, 15)],
                center,
            ));
            clip.push(offset_contour(
                &[(5, -25), (25, -25), (25, 5), (15, 5), (15, -15), (5, -15)],
                center,
            ));
            clip.push(offset_contour(
                &[
                    (-25, -5),
                    (-15, -5),
                    (-15, 15),
                    (-5, 15),
                    (-5, 25),
                    (-25, 25),
                ],
                center,
            ));
        }
    }

    Case {
        scenario: "tetris_square",
        label: "Tetris square",
        operation: Operation::Union,
        n,
        subject,
        clip,
    }
}

fn offset_contour(points: &[(i64, i64)], offset: (i64, i64)) -> Contour<i64> {
    points
        .iter()
        .map(|&(x, y)| IntPoint::new(offset.0 + x, offset.1 + y))
        .collect()
}

fn windows(n: usize) -> Case<i64> {
    let extent = (n as i64 - 1) * 30 + 20;
    let start = -extent / 2;
    let mut subject = Vec::with_capacity(n * n);
    let mut clip = Vec::with_capacity(n * n);
    for row in 0..n {
        for column in 0..n {
            let x = start + column as i64 * 30;
            let y = start + row as i64 * 30;
            subject.push(rectangle(x, y, 20, 20));
            clip.push(rectangle(x + 5, y + 5, 10, 10));
        }
    }
    Case {
        scenario: "windows",
        label: "Windows",
        operation: Operation::Difference,
        n,
        subject,
        clip,
    }
}

fn nested_squares(n: usize) -> Case<i64> {
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
    Case {
        scenario: "nested_squares",
        label: "Nested squares",
        operation: Operation::Union,
        n,
        subject,
        clip,
    }
}

fn sieve(n: usize) -> Case<i64> {
    let step = 16_i64;
    let hole_size = 8_i64;
    let padding = 8_i64;
    let first_center = -((n as i64 - 1) * step) / 2;
    let outer_half = ((n as i64 - 1) * step) / 2 + hole_size / 2 + padding;
    let mut clip = Vec::with_capacity(n * n);
    for row in 0..n {
        for column in 0..n {
            let center_x = first_center + column as i64 * step;
            let center_y = first_center + row as i64 * step;
            clip.push(rectangle(
                center_x - hole_size / 2,
                center_y - hole_size / 2,
                hole_size,
                hole_size,
            ));
        }
    }
    Case {
        scenario: "sieve",
        label: "Sieve",
        operation: Operation::Difference,
        n,
        subject: vec![rectangle(
            -outer_half,
            -outer_half,
            2 * outer_half,
            2 * outer_half,
        )],
        clip,
    }
}

fn rectangle(x: i64, y: i64, width: i64, height: i64) -> Contour<i64> {
    vec![
        IntPoint::new(x, y),
        IntPoint::new(x + width, y),
        IntPoint::new(x + width, y + height),
        IntPoint::new(x, y + height),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        SCENARIOS, cross, full_sizes, make_i64, make_preview, max_n, rectangle, tetris_square,
    };
    use crate::model::Operation;
    use x_overlay::core::fill_rule::FillRule;
    use x_overlay::core::overlay::Overlay;
    use x_overlay::core::overlay_rule::OverlayRule;

    #[test]
    fn every_preview_is_orthogonal_and_centered() {
        for scenario in SCENARIOS {
            let case = make_preview(scenario);
            let contours = case.subject.iter().chain(&case.clip);
            for contour in contours.clone() {
                assert!(contour.len() >= 4, "{scenario} has a short contour");
                for index in 0..contour.len() {
                    let a = contour[index];
                    let b = contour[(index + 1) % contour.len()];
                    assert!(
                        a.x == b.x || a.y == b.y,
                        "{scenario} contains a diagonal edge"
                    );
                }
            }
            let points = contours.flatten().collect::<Vec<_>>();
            let min_x = points.iter().map(|point| point.x).min().unwrap();
            let max_x = points.iter().map(|point| point.x).max().unwrap();
            let min_y = points.iter().map(|point| point.y).min().unwrap();
            let max_y = points.iter().map(|point| point.y).max().unwrap();
            assert_eq!(min_x, -max_x, "{scenario} is not centered on x");
            assert_eq!(min_y, -max_y, "{scenario} is not centered on y");
        }
    }

    #[test]
    fn i64_cases_require_64_bit_coordinates() {
        for scenario in SCENARIOS {
            let case = make_i64(scenario, 2);
            assert!(
                case.subject
                    .iter()
                    .chain(&case.clip)
                    .flatten()
                    .all(|point| point.x > i32::MAX as i64 && point.y > i32::MAX as i64),
                "{scenario} did not enter the i64-only coordinate range"
            );
        }
    }

    #[test]
    fn benchmark_sizes_are_quarter_steps_from_the_maximum() {
        for scenario in SCENARIOS {
            assert!(max_n(scenario).is_power_of_two());
            assert_eq!(full_sizes(scenario).len(), 4);
            assert_eq!(full_sizes(scenario).last(), Some(&max_n(scenario)));
        }
        assert_eq!(full_sizes("checkerboard"), vec![32, 128, 512, 2048]);
        assert_eq!(full_sizes("lines_net"), vec![64, 256, 1024, 4096]);
        assert_eq!(full_sizes("wind_mill"), vec![16, 64, 256, 1024]);
        assert_eq!(full_sizes("cross"), vec![16, 64, 256, 1024]);
        assert_eq!(full_sizes("tetris_square"), vec![16, 64, 256, 1024]);
        assert_eq!(full_sizes("nested_squares"), vec![256, 1024, 4096, 16_384]);
        assert_eq!(full_sizes("sieve"), vec![16, 64, 256, 1024]);
    }

    #[test]
    fn nested_squares_stresses_union_shape_generation() {
        assert!(matches!(
            make_i64("nested_squares", 2).operation,
            Operation::Union
        ));
    }

    #[test]
    fn sieve_is_one_shape_with_a_square_clip_grid() {
        let case = make_i64("sieve", 4);
        assert_eq!(case.subject.len(), 1);
        assert_eq!(case.clip.len(), 16);
        assert!(matches!(case.operation, Operation::Difference));
    }

    #[test]
    fn cross_has_two_subject_rectangles_and_five_clip_squares_per_cell() {
        let case = cross(1);
        assert_eq!(
            case.subject,
            vec![rectangle(-30, -10, 60, 20), rectangle(-10, -30, 20, 60)]
        );
        assert_eq!(
            case.clip,
            vec![
                rectangle(-25, -5, 10, 10),
                rectangle(15, -5, 10, 10),
                rectangle(-5, -25, 10, 10),
                rectangle(-5, 15, 10, 10),
                rectangle(-5, -5, 10, 10),
            ]
        );
        assert!(matches!(case.operation, Operation::Difference));

        let shapes = Overlay::<i64>::with_contours(&case.subject, &case.clip)
            .overlay(FillRule::NonZero, OverlayRule::Difference);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 6);
    }

    #[test]
    fn tetris_square_has_diagonal_inputs_and_one_square_hole() {
        let case = tetris_square(1);
        assert_eq!(case.subject.len(), 2);
        assert_eq!(case.clip.len(), 2);
        assert!(matches!(case.operation, Operation::Union));

        let shapes = Overlay::<i64>::with_contours(&case.subject, &case.clip)
            .overlay(FillRule::NonZero, OverlayRule::Union);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 2);
        assert_eq!(
            shapes[0][1]
                .iter()
                .map(|point| (point.x, point.y))
                .collect::<Vec<_>>(),
            vec![(-15, -15), (-15, 15), (15, 15), (15, -15)]
        );
    }
}
