use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use xoverlay_benchmark::model::BenchmarkReport;
use xoverlay_benchmark::scenarios;

fn main() {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("results/latest.json"));
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("site/index.html"));
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let template = fs::read_to_string(crate_root.join("site/template/index.html"))
        .expect("unable to read report template");
    let css = fs::read_to_string(crate_root.join("site/template/style.css"))
        .expect("unable to read report stylesheet");
    let javascript = fs::read_to_string(crate_root.join("site/template/app.js"))
        .expect("unable to read report script");
    let source = fs::read_to_string(resolve(crate_root, &input))
        .expect("unable to read benchmark result JSON");
    let mut report: BenchmarkReport =
        serde_json::from_str(&source).expect("unable to parse benchmark result JSON");
    include_current_scenarios(&mut report);
    let json = serde_json::to_string_pretty(&report)
        .expect("unable to serialize benchmark result JSON")
        .replace("</", "<\\/");
    let html = template
        .replace("__REPORT_CSS__", &css)
        .replace("__RESULTS_JSON__", &json)
        .replace("__REPORT_JS__", &javascript);
    let output = resolve(crate_root, &output);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("unable to create report output directory");
    }
    fs::write(&output, html).expect("unable to write report");
    println!("generated {}", output.display());
}

fn include_current_scenarios(report: &mut BenchmarkReport) {
    for scenario in scenarios::scenario_info() {
        if report.scenarios.iter().any(|item| item.id == scenario.id) {
            continue;
        }
        let sizes = if report.metadata.profile == "smoke" {
            scenarios::smoke_sizes(&scenario.id)
        } else {
            scenarios::full_sizes(&scenario.id)
        };
        report
            .metadata
            .scenario_sizes
            .insert(scenario.id.clone(), sizes);
        report.scenarios.push(scenario);
    }
}

fn resolve(crate_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        crate_root.join(path)
    }
}
