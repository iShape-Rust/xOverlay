use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use x_overlay::i_float::int::number::int::IntNumber;
use x_overlay::i_float::int::point::IntPoint;

pub type Contour<I> = Vec<IntPoint<I>>;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Xor,
    Union,
    Intersect,
    Difference,
}

impl Operation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Xor => "xor",
            Self::Union => "union",
            Self::Intersect => "intersect",
            Self::Difference => "difference",
        }
    }

    pub const fn code(self) -> u8 {
        match self {
            Self::Xor => 0,
            Self::Union => 1,
            Self::Intersect => 2,
            Self::Difference => 3,
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateType {
    I32,
    I64,
}

impl CoordinateType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
        }
    }
}

impl fmt::Display for CoordinateType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Shapes,
    Contours,
}

impl OutputKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shapes => "shapes",
            Self::Contours => "contours",
        }
    }
}

impl fmt::Display for OutputKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct Case<I: IntNumber> {
    pub scenario: &'static str,
    pub label: &'static str,
    pub operation: Operation,
    pub n: usize,
    pub subject: Vec<Contour<I>>,
    pub clip: Vec<Contour<I>>,
}

impl<I: IntNumber> Case<I> {
    pub fn input_metrics(&self) -> InputMetrics {
        let subject_points = self.subject.iter().map(Vec::len).sum::<usize>();
        let clip_points = self.clip.iter().map(Vec::len).sum::<usize>();
        InputMetrics {
            subject_contours: self.subject.len(),
            clip_contours: self.clip.len(),
            subject_points,
            clip_points,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InputMetrics {
    pub subject_contours: usize,
    pub clip_contours: usize,
    pub subject_points: usize,
    pub clip_points: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OutputMetrics {
    pub shapes: Option<usize>,
    pub contours: usize,
    pub points: usize,
    pub area_two: i128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TimingSummary {
    pub iterations_per_sample: usize,
    pub samples_ns: Vec<u64>,
    pub median_ns: u64,
    pub min_ns: u64,
    pub max_ns: u64,
    pub timed_out: bool,
}

impl TimingSummary {
    pub fn new(iterations_per_sample: usize, mut samples_ns: Vec<u64>) -> Self {
        samples_ns.sort_unstable();
        let median_ns = samples_ns[samples_ns.len() / 2];
        let min_ns = samples_ns[0];
        let max_ns = *samples_ns.last().expect("at least one timing sample");
        Self {
            iterations_per_sample,
            samples_ns,
            median_ns,
            min_ns,
            max_ns,
            timed_out: false,
        }
    }

    pub fn timeout(limit_ns: u64) -> Self {
        Self {
            iterations_per_sample: 0,
            samples_ns: Vec::new(),
            median_ns: limit_ns,
            min_ns: limit_ns,
            max_ns: limit_ns,
            timed_out: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Measurement {
    pub scenario: String,
    pub label: String,
    pub operation: Operation,
    pub n: usize,
    pub coordinate_type: CoordinateType,
    pub output_kind: OutputKind,
    pub implementation: String,
    pub execution: String,
    pub threads: usize,
    pub output_semantics: String,
    pub warning: Option<String>,
    pub input: InputMetrics,
    pub output: Option<OutputMetrics>,
    pub timing: TimingSummary,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScenarioInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub operation: Operation,
    pub illustration: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_polygons: Option<InputPolygonCount>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InputPolygonCount {
    pub formula: String,
    pub max_n: usize,
    pub subject: usize,
    pub clip: usize,
    pub total: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReportMetadata {
    pub generated_at_unix_seconds: u64,
    pub profile: String,
    pub budget_ms: u64,
    pub samples: usize,
    pub solver_timeout_seconds: f64,
    pub scenario_sizes: BTreeMap<String, Vec<usize>>,
    pub os: String,
    pub architecture: String,
    pub cpu: String,
    pub machine_source: String,
    pub logical_cpus: usize,
    pub rustc: String,
    pub cxx: String,
    pub rust_flags: String,
    pub cxx_flags: String,
    pub i_overlay_version: String,
    pub x_overlay_version: String,
    pub boost_version: String,
    pub repository_commit: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkReport {
    pub schema_version: u32,
    pub metadata: ReportMetadata,
    pub scenarios: Vec<ScenarioInfo>,
    pub measurements: Vec<Measurement>,
}
