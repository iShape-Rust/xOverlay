use i_overlay::core::fill_rule::FillRule as IFillRule;
use i_overlay::core::overlay::Overlay as IOverlay;
use i_overlay::core::overlay_rule::OverlayRule as IOverlayRule;
use std::env;
use std::hint::black_box;
use std::time::{Duration, Instant};
use x_overlay::core::cpu_count::CPUCount;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;
use x_overlay::i_shape::flat::buffer::FlatContoursBuffer;
use x_overlay::i_shape::int::area::Area;
use x_overlay::i_shape::int::shape::{IntContour, IntShape, IntShapes};

struct Config {
    scenario: String,
    n_override: Option<usize>,
    budget: Duration,
}

struct Workload {
    name: &'static str,
    n: usize,
    rule: OverlayRule,
    subject: IntShape<i32>,
    clip: IntShape<i32>,
    expected_area: i64,
}

#[derive(Clone, Copy)]
struct Measurement {
    iterations: usize,
    elapsed: Duration,
}

impl Measurement {
    fn milliseconds_per_iteration(self) -> f64 {
        self.elapsed.as_secs_f64() * 1_000.0 / self.iterations as f64
    }
}

fn main() {
    let config = parse_config();
    let workloads = make_workloads(&config);

    println!(
        "Rust comparison: xOverlay={}, iOverlay={}, fill=NonZero, budget={}ms",
        env!("CARGO_PKG_VERSION"),
        i_overlay_version(),
        config.budget.as_millis()
    );
    for workload in workloads {
        run_workload(&workload, config.budget);
    }
}

fn i_overlay_version() -> &'static str {
    "8.0.0"
}

fn parse_config() -> Config {
    let mut scenario = String::from("all");
    let mut n_override = None;
    let mut budget = Duration::from_millis(600);
    let mut args = env::args().skip(1);
    while let Some(option) = args.next() {
        match option.as_str() {
            "--scenario" => scenario = args.next().expect("--scenario requires a value"),
            "--n" => {
                let value = args.next().expect("--n requires a value");
                n_override = Some(value.parse().expect("--n must be a positive integer"));
            }
            "--budget-ms" => {
                let value = args.next().expect("--budget-ms requires a value");
                budget = Duration::from_millis(
                    value
                        .parse()
                        .expect("--budget-ms must be a positive integer"),
                );
            }
            "--help" => {
                println!(
                    "Usage: overlay_comparison [--scenario NAME] [--n N] [--budget-ms MS]\n\
                     Scenarios: all, checkerboard, not-overlap, lines-net, windows, nested"
                );
                std::process::exit(0);
            }
            _ => panic!("unknown option: {option}"),
        }
    }
    assert!(budget > Duration::ZERO);
    assert!(n_override != Some(0));
    assert!(
        scenario != "all" || n_override.is_none(),
        "--n requires one scenario"
    );
    Config {
        scenario,
        n_override,
        budget,
    }
}

fn make_workloads(config: &Config) -> Vec<Workload> {
    if config.scenario != "all" {
        let default_n = if config.scenario == "nested" {
            4096
        } else {
            128
        };
        return vec![make_workload(
            &config.scenario,
            config.n_override.unwrap_or(default_n),
        )];
    }

    vec![
        checkerboard(128),
        not_overlap(128),
        lines_net(128),
        windows(128),
        nested_squares(4096),
    ]
}

fn make_workload(scenario: &str, n: usize) -> Workload {
    match scenario {
        "checkerboard" => checkerboard(n),
        "not-overlap" => not_overlap(n),
        "lines-net" => lines_net(n),
        "windows" => windows(n),
        "nested" => nested_squares(n),
        _ => panic!("unknown scenario: {scenario}"),
    }
}

fn checkerboard(n: usize) -> Workload {
    let count = n as i64;
    let clip_count = (n - 1) as i64;
    Workload {
        name: "Checkerboard / XOR",
        n,
        rule: OverlayRule::Xor,
        subject: many_squares(IntPoint::new(0, 0), 20, 30, n),
        clip: many_squares(IntPoint::new(15, 15), 20, 30, n - 1),
        expected_area: 400 * count * count + 200 * clip_count * clip_count,
    }
}

fn not_overlap(n: usize) -> Workload {
    let count = n as i64;
    let clip_count = (n - 1) as i64;
    Workload {
        name: "Not Overlap / Union",
        n,
        rule: OverlayRule::Union,
        subject: many_squares(IntPoint::new(0, 0), 10, 30, n),
        clip: many_squares(IntPoint::new(15, 15), 10, 30, n - 1),
        expected_area: 100 * (count * count + clip_count * clip_count),
    }
}

fn many_squares(start: IntPoint, size: i32, step: i32, n: usize) -> IntShape<i32> {
    let mut result = Vec::with_capacity(n * n);
    for row in 0..n {
        let y = start.y + row as i32 * step;
        for column in 0..n {
            let x = start.x + column as i32 * step;
            result.push(rectangle(x, y, size, size));
        }
    }
    result
}

fn many_lines_x(spacing: i32, n: usize) -> IntShape<i32> {
    let width = spacing / 2;
    let span = spacing * n as i32 / 2;
    let mut x = -span + width / 2;
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        result.push(rectangle(x, -span, width, 2 * span));
        x += spacing;
    }
    result
}

fn many_lines_y(spacing: i32, n: usize) -> IntShape<i32> {
    let height = spacing / 2;
    let span = spacing * n as i32 / 2;
    let mut y = -span + height / 2;
    let mut result = Vec::with_capacity(n);
    for _ in 0..n {
        result.push(rectangle(-span, y - height, 2 * span, height));
        y += spacing;
    }
    result
}

fn lines_net(n: usize) -> Workload {
    let count = n as i64;
    Workload {
        name: "Lines Net / Intersection",
        n,
        rule: OverlayRule::Intersect,
        subject: many_lines_x(20, n),
        clip: many_lines_y(20, n),
        expected_area: 100 * count * count - 50 * count,
    }
}

fn windows(n: usize) -> Workload {
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
    let count = n as i64;
    Workload {
        name: "Windows / Difference",
        n,
        rule: OverlayRule::Difference,
        subject,
        clip,
        expected_area: 300 * count * count,
    }
}

fn nested_squares(n: usize) -> Workload {
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
    let count = n as i64;
    Workload {
        name: "Nested Squares / XOR",
        n,
        rule: OverlayRule::Xor,
        subject,
        clip,
        expected_area: 128 * count * count,
    }
}

fn rectangle(x: i32, y: i32, width: i32, height: i32) -> IntContour<i32> {
    vec![
        IntPoint::new(x, y),
        IntPoint::new(x, y + height),
        IntPoint::new(x + width, y + height),
        IntPoint::new(x + width, y),
    ]
}

fn run_workload(workload: &Workload, budget: Duration) {
    let serial_validation = solve_x(workload, CPUCount::Single);
    validate(workload, &serial_validation, "xOverlay serial");
    let multi_validation = solve_x(workload, CPUCount::Auto);
    validate(workload, &multi_validation, "xOverlay multithread");
    let i_validation = solve_i(workload);
    validate(workload, &i_validation, "iOverlay");

    let serial_contours_validation = solve_x_contours(workload, CPUCount::Single);
    validate_contours(
        workload,
        &serial_contours_validation,
        "xOverlay contours serial",
    );
    let multi_contours_validation = solve_x_contours(workload, CPUCount::Auto);
    validate_contours(
        workload,
        &multi_contours_validation,
        "xOverlay contours multithread",
    );
    let i_contours_validation = solve_i_contours(workload);
    validate_flat_contours(workload, &i_contours_validation, "iOverlay contours");

    let serial = measure_for(budget, || solve_x(workload, CPUCount::Single));
    let multithread = measure_for(budget, || solve_x(workload, CPUCount::Auto));
    let i_overlay = measure_for(budget, || solve_i(workload));
    let serial_contours = measure_for(budget, || solve_x_contours(workload, CPUCount::Single));
    let multithread_contours = measure_for(budget, || solve_x_contours(workload, CPUCount::Auto));
    let i_overlay_contours = measure_for(budget, || solve_i_contours(workload));

    println!(
        "{}: n={}, inputs={}, area={}, output_shapes={}",
        workload.name,
        workload.n,
        workload.subject.len() + workload.clip.len(),
        workload.expected_area,
        serial_validation.len()
    );
    print_measurement("xOverlay serial", serial);
    print_measurement("xOverlay multithread", multithread);
    print_measurement("iOverlay", i_overlay);
    print_measurement("xOverlay contours serial", serial_contours);
    print_measurement("xOverlay contours MT", multithread_contours);
    print_measurement("iOverlay contours", i_overlay_contours);
}

fn solve_x(workload: &Workload, cpu_count: CPUCount) -> IntShapes<i32> {
    let overlay = Overlay::with_contours_custom(
        &workload.subject,
        &workload.clip,
        Default::default(),
        cpu_count,
    );
    overlay.overlay(FillRule::NonZero, workload.rule)
}

fn solve_i(workload: &Workload) -> IntShapes<i32> {
    let rule = i_overlay_rule(workload.rule);
    let mut overlay = IOverlay::with_contours(&workload.subject, &workload.clip);
    overlay.overlay(rule, IFillRule::NonZero)
}

fn solve_x_contours(workload: &Workload, cpu_count: CPUCount) -> IntShape<i32> {
    let overlay = Overlay::with_contours_custom(
        &workload.subject,
        &workload.clip,
        Default::default(),
        cpu_count,
    );
    overlay.overlay_contours(FillRule::NonZero, workload.rule)
}

fn solve_i_contours(workload: &Workload) -> FlatContoursBuffer<i32> {
    let mut overlay = IOverlay::with_contours(&workload.subject, &workload.clip);
    let mut output = FlatContoursBuffer::default();
    overlay.overlay_into(
        i_overlay_rule(workload.rule),
        IFillRule::NonZero,
        &mut output,
    );
    output
}

fn i_overlay_rule(rule: OverlayRule) -> IOverlayRule {
    match rule {
        OverlayRule::Xor => IOverlayRule::Xor,
        OverlayRule::Union => IOverlayRule::Union,
        OverlayRule::Intersect => IOverlayRule::Intersect,
        OverlayRule::Difference => IOverlayRule::Difference,
        _ => unreachable!(),
    }
}

fn validate(workload: &Workload, shapes: &IntShapes<i32>, solver: &str) {
    let area = shapes.area_two() / 2;
    assert_eq!(
        area, workload.expected_area,
        "{solver} produced the wrong area for {}",
        workload.name
    );
}

fn validate_contours(workload: &Workload, contours: &[IntContour<i32>], solver: &str) {
    let area = contours
        .iter()
        .map(|contour| contour.area_two())
        .sum::<i64>()
        / 2;
    assert_eq!(
        area, workload.expected_area,
        "{solver} produced the wrong area for {}",
        workload.name
    );
}

fn validate_flat_contours(workload: &Workload, contours: &FlatContoursBuffer<i32>, solver: &str) {
    let area = contours
        .ranges
        .iter()
        .map(|range| contours.points[range.clone()].area_two())
        .sum::<i64>()
        / 2;
    assert_eq!(
        area, workload.expected_area,
        "{solver} produced the wrong area for {}",
        workload.name
    );
}

fn measure_for<T>(mut budget: Duration, mut operation: impl FnMut() -> T) -> Measurement {
    black_box(operation());
    if budget.is_zero() {
        budget = Duration::from_millis(1);
    }
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
    let milliseconds = measurement.milliseconds_per_iteration();
    println!(
        "  {name:<24}{milliseconds:>10.3} ms/iter, {:>10.1} iter/s, iterations={}",
        1_000.0 / milliseconds,
        measurement.iterations
    );
}
