use i_overlay::core::fill_rule::FillRule as IFillRule;
use i_overlay::core::integer::OverlayInt as IOverlayInt;
use i_overlay::core::overlay::Overlay as IOverlay;
use i_overlay::core::overlay_rule::OverlayRule as IOverlayRule;
use i_overlay::core::solver::{MultithreadOptions, Solver as ISolver};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::hint::black_box;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use x_overlay::core::cpu_count::CPUCount;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::integer::OverlayInt as XOverlayInt;
use x_overlay::core::overlay::Overlay as XOverlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_shape::flat::buffer::FlatContoursBuffer;
use x_overlay::i_shape::int::shape::IntShapes;
use xoverlay_benchmark::model::{
    BenchmarkReport, Case, Contour, CoordinateType, Measurement, Operation, OutputKind,
    OutputMetrics, ReportMetadata, TimingSummary,
};
use xoverlay_benchmark::scenarios::{self, Coordinate, SCENARIOS};

#[derive(Debug)]
struct Config {
    profile: String,
    scenarios: Vec<String>,
    budget: Duration,
    samples: usize,
    output: PathBuf,
    boost_bin: Option<PathBuf>,
    solver_timeout: Duration,
    machine_override: Option<String>,
}

trait BenchCoordinate: Coordinate + IOverlayInt + XOverlayInt + Copy + Send + Sync + 'static {
    const TYPE: CoordinateType;
    const WIDTH: u8;
    fn to_i128(self) -> i128;
    fn write_le(self, writer: &mut impl Write) -> std::io::Result<()>;
}

impl BenchCoordinate for i32 {
    const TYPE: CoordinateType = CoordinateType::I32;
    const WIDTH: u8 = 4;

    fn to_i128(self) -> i128 {
        self as i128
    }

    fn write_le(self, writer: &mut impl Write) -> std::io::Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl BenchCoordinate for i64 {
    const TYPE: CoordinateType = CoordinateType::I64;
    const WIDTH: u8 = 8;

    fn to_i128(self) -> i128 {
        self as i128
    }

    fn write_le(self, writer: &mut impl Write) -> std::io::Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

fn main() {
    if run_rust_solver_child() {
        return;
    }
    let config = parse_config();
    let logical_cpus = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    let mut measurements = Vec::new();
    let mut boost_version = String::from("not-run");
    let mut boost_disabled = HashSet::new();
    let mut rust_disabled = HashSet::new();
    let mut scenario_sizes = BTreeMap::new();

    for scenario in &config.scenarios {
        let sizes = if config.profile == "smoke" {
            scenarios::smoke_sizes(scenario)
        } else {
            scenarios::full_sizes(scenario)
        };
        scenario_sizes.insert(scenario.clone(), sizes.clone());
        for n in sizes {
            let case = scenarios::make_i32(scenario, n);
            run_case(
                &case,
                &config,
                logical_cpus,
                &mut boost_version,
                &mut boost_disabled,
                &mut rust_disabled,
                &mut measurements,
            );
            let case = scenarios::make_i64(scenario, n);
            run_case(
                &case,
                &config,
                logical_cpus,
                &mut boost_version,
                &mut boost_disabled,
                &mut rust_disabled,
                &mut measurements,
            );
        }
    }

    let report = BenchmarkReport {
        schema_version: 4,
        metadata: metadata(&config, logical_cpus, boost_version, scenario_sizes),
        scenarios: scenarios::scenario_info(),
        measurements,
    };
    if let Some(parent) = config.output.parent() {
        fs::create_dir_all(parent).expect("unable to create results directory");
    }
    let json = serde_json::to_string_pretty(&report).expect("unable to serialize benchmark report");
    fs::write(&config.output, json).expect("unable to write benchmark report");
    println!("wrote {}", config.output.display());
}

#[derive(Debug, Deserialize, Serialize)]
struct RustChildResult {
    output: OutputMetrics,
    timing: TimingSummary,
}

fn run_rust_solver_child() -> bool {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("--rust-solver-child") {
        return false;
    }
    let scenario = args.next().expect("child scenario is required");
    let n = args
        .next()
        .expect("child N is required")
        .parse::<usize>()
        .expect("child N must be an integer");
    let coordinate = args.next().expect("child coordinate is required");
    let output_kind = match args.next().as_deref() {
        Some("shapes") => OutputKind::Shapes,
        Some("contours") => OutputKind::Contours,
        _ => panic!("child output kind must be shapes or contours"),
    };
    let solver = RustSolver::from_id(&args.next().expect("child solver is required"));
    let budget = Duration::from_millis(
        args.next()
            .expect("child budget is required")
            .parse::<u64>()
            .expect("child budget must be an integer"),
    );
    let samples = args
        .next()
        .expect("child sample count is required")
        .parse::<usize>()
        .expect("child sample count must be an integer");
    let result = match coordinate.as_str() {
        "i32" => measure_rust_child(
            &scenarios::make_i32(&scenario, n),
            output_kind,
            solver,
            budget,
            samples,
        ),
        "i64" => measure_rust_child(
            &scenarios::make_i64(&scenario, n),
            output_kind,
            solver,
            budget,
            samples,
        ),
        _ => panic!("child coordinate must be i32 or i64"),
    };
    println!(
        "{}",
        serde_json::to_string(&result).expect("unable to serialize Rust child result")
    );
    true
}

fn measure_rust_child<I: BenchCoordinate>(
    case: &Case<I>,
    output_kind: OutputKind,
    solver: RustSolver,
    budget: Duration,
    samples: usize,
) -> RustChildResult {
    let first_start = Instant::now();
    let first_output = solve_rust(case, output_kind, solver);
    let first_elapsed = first_start.elapsed().max(Duration::from_nanos(1));
    let output = metrics_from_result(&first_output);
    let target = budget / samples as u32;
    if first_elapsed >= target {
        return RustChildResult {
            output,
            timing: TimingSummary::new(1, vec![duration_ns(first_elapsed)]),
        };
    }

    let iterations =
        ((target.as_nanos() / first_elapsed.as_nanos()).max(1) as usize).min(1_000_000);
    let mut sample_values = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(solve_rust(case, output_kind, solver));
        }
        let elapsed = start.elapsed().as_nanos() / iterations as u128;
        sample_values.push(elapsed.min(u64::MAX as u128) as u64);
    }
    RustChildResult {
        output,
        timing: TimingSummary::new(iterations, sample_values),
    }
}

fn duration_ns(duration: Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

fn parse_config() -> Config {
    let mut profile = String::from("smoke");
    let mut selected = String::from("all");
    let mut budget_ms = 250_u64;
    let mut samples = 7_usize;
    let mut output = PathBuf::from("results/latest.json");
    let mut boost_bin = None;
    let mut solver_timeout_seconds = 60.0_f64;
    let mut machine_override = None;
    let mut args = env::args().skip(1);
    while let Some(option) = args.next() {
        match option.as_str() {
            "--profile" => profile = args.next().expect("--profile requires smoke or full"),
            "--scenario" => selected = args.next().expect("--scenario requires a value"),
            "--budget-ms" => {
                budget_ms = args
                    .next()
                    .expect("--budget-ms requires a value")
                    .parse()
                    .expect("--budget-ms must be an integer")
            }
            "--samples" => {
                samples = args
                    .next()
                    .expect("--samples requires a value")
                    .parse()
                    .expect("--samples must be an integer")
            }
            "--output" => output = PathBuf::from(args.next().expect("--output requires a path")),
            "--boost-bin" => {
                boost_bin = Some(PathBuf::from(
                    args.next().expect("--boost-bin requires a path"),
                ))
            }
            "--solver-timeout-seconds" => {
                solver_timeout_seconds = args
                    .next()
                    .expect("--solver-timeout-seconds requires a value")
                    .parse()
                    .expect("--solver-timeout-seconds must be a number")
            }
            "--machine" => {
                let value = args.next().expect("--machine requires a value");
                let value = value.trim();
                if !value.is_empty() {
                    machine_override = Some(value.to_string());
                }
            }
            "--no-boost" => boost_bin = None,
            "--help" => {
                println!(
                    "Usage: overlay-benchmark [--profile smoke|full] [--scenario NAME|all]\n\
                     [--budget-ms N] [--samples N] [--output FILE] [--boost-bin FILE] [--no-boost]\n\
                     [--solver-timeout-seconds N] [--machine NAME]"
                );
                std::process::exit(0);
            }
            _ => panic!("unknown option: {option}"),
        }
    }
    assert!(matches!(profile.as_str(), "smoke" | "full"));
    assert!(budget_ms > 0);
    assert!(samples > 0);
    assert!(solver_timeout_seconds.is_finite() && solver_timeout_seconds > 0.0);
    let scenario_list = if selected == "all" {
        SCENARIOS.iter().map(|value| (*value).to_string()).collect()
    } else {
        assert!(SCENARIOS.contains(&selected.as_str()), "unknown scenario");
        vec![selected]
    };
    Config {
        profile,
        scenarios: scenario_list,
        budget: Duration::from_millis(budget_ms),
        samples,
        output,
        boost_bin,
        solver_timeout: Duration::from_secs_f64(solver_timeout_seconds),
        machine_override,
    }
}

fn run_case<I: BenchCoordinate>(
    case: &Case<I>,
    config: &Config,
    logical_cpus: usize,
    boost_version: &mut String,
    boost_disabled: &mut HashSet<String>,
    rust_disabled: &mut HashSet<String>,
    measurements: &mut Vec<Measurement>,
) {
    println!(
        "{} n={} {} inputs={} contours",
        case.label,
        case.n,
        I::TYPE,
        case.subject.len() + case.clip.len()
    );
    let case_file = config.boost_bin.as_ref().map(|_| write_case_file(case));
    for output_kind in [OutputKind::Shapes, OutputKind::Contours] {
        let baseline = solve_x(case, output_kind, CPUCount::Single);
        let expected = metrics_from_result(&baseline);

        for (implementation, execution, threads, solver) in [
            ("iOverlay", "single_thread", 1, RustSolver::IOverlay(false)),
            (
                "iOverlay",
                "multi_thread",
                logical_cpus,
                RustSolver::IOverlay(true),
            ),
            (
                "xOverlay",
                "single_thread",
                1,
                RustSolver::XOverlay(CPUCount::Single),
            ),
            (
                "xOverlay",
                "multi_thread",
                logical_cpus,
                RustSolver::XOverlay(CPUCount::Auto),
            ),
        ] {
            let solver_key = format!("{}:{implementation}:{execution}", case.scenario);
            if rust_disabled.contains(&solver_key) {
                continue;
            }
            let Some(result) = run_rust_solver(case, output_kind, solver, config) else {
                let timeout_ns = duration_ns(config.solver_timeout);
                let timeout_seconds = config.solver_timeout.as_secs_f64();
                eprintln!(
                    "{implementation} {execution} timed out for {} n={} {} {}; remaining runs for this solver and scenario will be skipped",
                    case.scenario,
                    case.n,
                    I::TYPE,
                    output_kind
                );
                measurements.push(Measurement {
                    scenario: case.scenario.to_string(),
                    label: case.label.to_string(),
                    operation: case.operation,
                    n: case.n,
                    coordinate_type: I::TYPE,
                    output_kind,
                    implementation: implementation.to_string(),
                    execution: execution.to_string(),
                    threads,
                    output_semantics: "not_materialized_timeout".to_string(),
                    warning: Some(format!(
                        "{implementation} {execution} exceeded the {timeout_seconds:.0}-second limit. Its actual time is greater than the recorded limit; remaining runs for this solver and scenario were skipped."
                    )),
                    input: case.input_metrics(),
                    output: None,
                    timing: TimingSummary::timeout(timeout_ns),
                });
                rust_disabled.insert(solver_key);
                continue;
            };
            let output = result.output;
            assert_eq!(
                output.area_two,
                expected.area_two,
                "{implementation} area mismatch for {} n={} {} {}",
                case.scenario,
                case.n,
                I::TYPE,
                output_kind
            );
            measurements.push(Measurement {
                scenario: case.scenario.to_string(),
                label: case.label.to_string(),
                operation: case.operation,
                n: case.n,
                coordinate_type: I::TYPE,
                output_kind,
                implementation: implementation.to_string(),
                execution: execution.to_string(),
                threads,
                output_semantics: output_kind.as_str().to_string(),
                warning: None,
                input: case.input_metrics(),
                output: Some(output),
                timing: result.timing,
            });
        }

        let boost_key = case.scenario.to_string();
        if let (Some(boost_bin), Some(case_file)) = (&config.boost_bin, &case_file)
            && !boost_disabled.contains(&boost_key)
        {
            if let Some(result) = run_boost(boost_bin, case_file, output_kind, config) {
                *boost_version = result.boost_version.clone();
                assert_eq!(
                    result.area_two,
                    expected.area_two,
                    "Boost area mismatch for {} n={} {} {}",
                    case.scenario,
                    case.n,
                    I::TYPE,
                    output_kind
                );
                measurements.push(Measurement {
                    scenario: case.scenario.to_string(),
                    label: case.label.to_string(),
                    operation: case.operation,
                    n: case.n,
                    coordinate_type: I::TYPE,
                    output_kind,
                    implementation: "Boost Polygon 90".to_string(),
                    execution: "single_thread".to_string(),
                    threads: 1,
                    output_semantics: if matches!(output_kind, OutputKind::Shapes) {
                        "shapes_with_holes".to_string()
                    } else {
                        "native_flat_polygons".to_string()
                    },
                    warning: if matches!(output_kind, OutputKind::Contours) {
                        Some("Boost native flat polygon output may fracture boundaries around holes; contour and point counts are not directly equivalent to iOverlay/xOverlay.".to_string())
                    } else {
                        None
                    },
                    input: case.input_metrics(),
                    output: Some(OutputMetrics {
                        shapes: result.shapes,
                        contours: result.contours,
                        points: result.points,
                        area_two: result.area_two,
                    }),
                    timing: TimingSummary::new(result.iterations_per_sample, result.samples_ns),
                });
            } else {
                let timeout_ns = config.solver_timeout.as_nanos().min(u64::MAX as u128) as u64;
                let timeout_seconds = config.solver_timeout.as_secs_f64();
                eprintln!(
                    "Boost timed out for {} n={} {} {}; remaining Boost runs for this scenario will be skipped",
                    case.scenario,
                    case.n,
                    I::TYPE,
                    output_kind
                );
                measurements.push(Measurement {
                    scenario: case.scenario.to_string(),
                    label: case.label.to_string(),
                    operation: case.operation,
                    n: case.n,
                    coordinate_type: I::TYPE,
                    output_kind,
                    implementation: "Boost Polygon 90".to_string(),
                    execution: "single_thread".to_string(),
                    threads: 1,
                    output_semantics: "not_materialized_timeout".to_string(),
                    warning: Some(format!(
                        "Boost exceeded the {timeout_seconds:.0}-second limit. Its actual time is greater than the recorded limit; remaining Boost runs for this scenario were skipped."
                    )),
                    input: case.input_metrics(),
                    output: None,
                    timing: TimingSummary::timeout(timeout_ns),
                });
                boost_disabled.insert(boost_key);
            }
        }
    }
    if let Some(path) = case_file {
        let _ = fs::remove_file(path);
    }
}

#[derive(Clone, Copy)]
enum RustSolver {
    IOverlay(bool),
    XOverlay(CPUCount),
}

impl RustSolver {
    fn id(self) -> &'static str {
        match self {
            Self::IOverlay(false) => "i_single",
            Self::IOverlay(true) => "i_multi",
            Self::XOverlay(CPUCount::Single) => "x_single",
            Self::XOverlay(_) => "x_multi",
        }
    }

    fn from_id(id: &str) -> Self {
        match id {
            "i_single" => Self::IOverlay(false),
            "i_multi" => Self::IOverlay(true),
            "x_single" => Self::XOverlay(CPUCount::Single),
            "x_multi" => Self::XOverlay(CPUCount::Auto),
            _ => panic!("unknown Rust solver id: {id}"),
        }
    }
}

fn run_rust_solver<I: BenchCoordinate>(
    case: &Case<I>,
    output_kind: OutputKind,
    solver: RustSolver,
    config: &Config,
) -> Option<RustChildResult> {
    let executable = env::current_exe().expect("unable to locate benchmark executable");
    let mut command = Command::new(executable);
    command
        .arg("--rust-solver-child")
        .arg(case.scenario)
        .arg(case.n.to_string())
        .arg(I::TYPE.as_str())
        .arg(output_kind.as_str())
        .arg(solver.id())
        .arg(config.budget.as_millis().to_string())
        .arg(config.samples.to_string());
    let output = run_command_with_timeout(&mut command, config.solver_timeout)?;
    if !output.status.success() {
        panic!(
            "Rust solver child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Some(serde_json::from_slice(&output.stdout).expect("Rust solver child returned invalid JSON"))
}

enum RustResult<I: BenchCoordinate> {
    Shapes(IntShapes<I>),
    Contours(Vec<Contour<I>>),
    FlatContours(FlatContoursBuffer<I>),
}

fn solve_rust<I: BenchCoordinate>(
    case: &Case<I>,
    output_kind: OutputKind,
    solver: RustSolver,
) -> RustResult<I> {
    match solver {
        RustSolver::IOverlay(multithread) => solve_i(case, output_kind, multithread),
        RustSolver::XOverlay(cpu_count) => solve_x(case, output_kind, cpu_count),
    }
}

fn solve_x<I: BenchCoordinate>(
    case: &Case<I>,
    output_kind: OutputKind,
    cpu_count: CPUCount,
) -> RustResult<I> {
    let overlay = XOverlay::<I>::with_contours_custom(
        &case.subject,
        &case.clip,
        Default::default(),
        cpu_count,
    );
    match output_kind {
        OutputKind::Shapes => {
            RustResult::Shapes(overlay.overlay(FillRule::NonZero, x_rule(case.operation)))
        }
        OutputKind::Contours => RustResult::Contours(
            overlay.overlay_contours(FillRule::NonZero, x_rule(case.operation)),
        ),
    }
}

fn solve_i<I: BenchCoordinate>(
    case: &Case<I>,
    output_kind: OutputKind,
    multithread: bool,
) -> RustResult<I> {
    let mut solver = ISolver::AUTO;
    solver.multithreading = multithread.then(MultithreadOptions::default);
    let mut overlay =
        IOverlay::with_contours_custom(&case.subject, &case.clip, Default::default(), solver);
    match output_kind {
        OutputKind::Shapes => {
            RustResult::Shapes(overlay.overlay(i_rule(case.operation), IFillRule::NonZero))
        }
        OutputKind::Contours => {
            let mut output = FlatContoursBuffer::default();
            overlay.overlay_into(i_rule(case.operation), IFillRule::NonZero, &mut output);
            RustResult::FlatContours(output)
        }
    }
}

fn x_rule(operation: Operation) -> OverlayRule {
    match operation {
        Operation::Xor => OverlayRule::Xor,
        Operation::Union => OverlayRule::Union,
        Operation::Intersect => OverlayRule::Intersect,
        Operation::Difference => OverlayRule::Difference,
    }
}

fn i_rule(operation: Operation) -> IOverlayRule {
    match operation {
        Operation::Xor => IOverlayRule::Xor,
        Operation::Union => IOverlayRule::Union,
        Operation::Intersect => IOverlayRule::Intersect,
        Operation::Difference => IOverlayRule::Difference,
    }
}

fn metrics_from_result<I: BenchCoordinate>(result: &RustResult<I>) -> OutputMetrics {
    match result {
        RustResult::Shapes(shapes) => OutputMetrics {
            shapes: Some(shapes.len()),
            contours: shapes.iter().map(Vec::len).sum(),
            points: shapes.iter().flatten().map(Vec::len).sum(),
            area_two: shapes
                .iter()
                .flatten()
                .map(|contour| area_two(contour))
                .sum(),
        },
        RustResult::Contours(contours) => OutputMetrics {
            shapes: None,
            contours: contours.len(),
            points: contours.iter().map(Vec::len).sum(),
            area_two: contours.iter().map(|contour| area_two(contour)).sum(),
        },
        RustResult::FlatContours(contours) => OutputMetrics {
            shapes: None,
            contours: contours.ranges.len(),
            points: contours.points.len(),
            area_two: contours
                .ranges
                .iter()
                .map(|range| area_two(&contours.points[range.clone()]))
                .sum(),
        },
    }
}

fn area_two<I: BenchCoordinate>(contour: &[x_overlay::i_float::int::point::IntPoint<I>]) -> i128 {
    if contour.len() < 3 {
        return 0;
    }
    let mut area = 0_i128;
    for index in 0..contour.len() {
        let a = contour[index];
        let b = contour[(index + 1) % contour.len()];
        area += a.x.to_i128() * b.y.to_i128() - b.x.to_i128() * a.y.to_i128();
    }
    area
}

fn write_case_file<I: BenchCoordinate>(case: &Case<I>) -> PathBuf {
    let path = env::temp_dir().join(format!(
        "xoverlay-benchmark-{}-{}-{}-{}.bin",
        std::process::id(),
        case.scenario,
        case.n,
        I::TYPE
    ));
    let file = File::create(&path).expect("unable to create Boost input file");
    let mut writer = BufWriter::new(file);
    writer.write_all(b"XOVCASE1").unwrap();
    writer
        .write_all(&[I::WIDTH, case.operation.code(), 0, 0])
        .unwrap();
    writer
        .write_all(&(case.subject.len() as u64).to_le_bytes())
        .unwrap();
    writer
        .write_all(&(case.clip.len() as u64).to_le_bytes())
        .unwrap();
    write_contours(&mut writer, &case.subject);
    write_contours(&mut writer, &case.clip);
    writer.flush().unwrap();
    path
}

fn write_contours<I: BenchCoordinate>(writer: &mut impl Write, contours: &[Contour<I>]) {
    for contour in contours {
        writer
            .write_all(&(contour.len() as u64).to_le_bytes())
            .unwrap();
        for point in contour {
            point.x.write_le(writer).unwrap();
            point.y.write_le(writer).unwrap();
        }
    }
}

#[derive(Debug, Deserialize)]
struct BoostResult {
    boost_version: String,
    iterations_per_sample: usize,
    samples_ns: Vec<u64>,
    shapes: Option<usize>,
    contours: usize,
    points: usize,
    area_two: i128,
}

fn run_boost(
    boost_bin: &Path,
    case_file: &Path,
    output_kind: OutputKind,
    config: &Config,
) -> Option<BoostResult> {
    let mut command = Command::new(boost_bin);
    command
        .arg("--input")
        .arg(case_file)
        .arg("--output-kind")
        .arg(output_kind.as_str())
        .arg("--budget-ms")
        .arg(config.budget.as_millis().to_string())
        .arg("--samples")
        .arg(config.samples.to_string());
    let output = run_command_with_timeout(&mut command, config.solver_timeout)?;
    if !output.status.success() {
        panic!(
            "Boost runner failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Some(serde_json::from_slice(&output.stdout).expect("Boost runner returned invalid JSON"))
}

struct CapturedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_command_with_timeout(command: &mut Command, timeout: Duration) -> Option<CapturedOutput> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().expect("unable to start child process");
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                child
                    .stdout
                    .take()
                    .expect("child stdout must be piped")
                    .read_to_end(&mut stdout)
                    .expect("unable to read child stdout");
                child
                    .stderr
                    .take()
                    .expect("child stderr must be piped")
                    .read_to_end(&mut stderr)
                    .expect("unable to read child stderr");
                return Some(CapturedOutput {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => panic!("unable to poll child process: {error}"),
        }
    }
}

fn metadata(
    config: &Config,
    logical_cpus: usize,
    boost_version: String,
    scenario_sizes: BTreeMap<String, Vec<usize>>,
) -> ReportMetadata {
    let (machine, machine_source) = detect_machine(config.machine_override.as_deref());
    ReportMetadata {
        generated_at_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        profile: config.profile.clone(),
        budget_ms: config.budget.as_millis() as u64,
        samples: config.samples,
        solver_timeout_seconds: config.solver_timeout.as_secs_f64(),
        scenario_sizes,
        os: env::consts::OS.to_string(),
        architecture: env::consts::ARCH.to_string(),
        cpu: machine,
        machine_source,
        logical_cpus,
        rustc: command_text("rustc", &["--version"]),
        cxx: command_text("clang++", &["--version"])
            .lines()
            .next()
            .unwrap_or("unknown")
            .to_string(),
        rust_flags: "--release; opt-level=3; codegen-units=1; lto=false".to_string(),
        cxx_flags: "-O3 -DNDEBUG -std=c++20 -march=native".to_string(),
        i_overlay_version: "8.0.0".to_string(),
        x_overlay_version: "0.1.0".to_string(),
        boost_version,
        repository_commit: command_text("git", &["rev-parse", "HEAD"]),
    }
}

fn detect_machine(user_value: Option<&str>) -> (String, String) {
    if let Some(value) = user_value.map(str::trim).filter(|value| !value.is_empty()) {
        return (value.to_string(), "user_set".to_string());
    }
    if let Some(value) = optional_command_text("sysctl", &["-n", "machdep.cpu.brand_string"]) {
        return (value, "sysctl".to_string());
    }
    if let Some(profile) = optional_command_text("system_profiler", &["SPHardwareDataType"])
        && let Some(value) = parse_system_profiler_machine(&profile)
    {
        return (value, "system_profiler".to_string());
    }
    ("unknown".to_string(), "unknown".to_string())
}

fn parse_system_profiler_machine(profile: &str) -> Option<String> {
    let fields = ["Model Name", "Chip", "Memory"]
        .into_iter()
        .filter_map(|label| {
            profile.lines().find_map(|line| {
                let (key, value) = line.trim().split_once(':')?;
                (key == label)
                    .then(|| value.trim())
                    .filter(|value| !value.is_empty())
            })
        })
        .collect::<Vec<_>>();
    (!fields.is_empty()).then(|| fields.join(" · "))
}

fn command_text(command: &str, args: &[&str]) -> String {
    optional_command_text(command, args).unwrap_or_else(|| "unknown".to_string())
}

fn optional_command_text(command: &str, args: &[&str]) -> Option<String> {
    Command::new(command)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{detect_machine, parse_system_profiler_machine};

    #[test]
    fn user_machine_override_has_an_explicit_source() {
        assert_eq!(
            detect_machine(Some("Custom CI runner")),
            ("Custom CI runner".to_string(), "user_set".to_string())
        );
    }

    #[test]
    fn system_profiler_exposes_only_safe_machine_fields() {
        let profile =
            "Model Name: Mac mini\nChip: Apple M4\nMemory: 24 GB\nSerial Number (system): secret";
        assert_eq!(
            parse_system_profiler_machine(profile).as_deref(),
            Some("Mac mini · Apple M4 · 24 GB")
        );
    }
}
