use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use xoverlay_benchmark::scenarios::{SCENARIOS, make_preview};
use xoverlay_benchmark::svg;

fn main() {
    let output = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/performance/assets")
    });
    fs::create_dir_all(&output).expect("unable to create SVG output directory");
    for scenario in SCENARIOS {
        let case = make_preview(scenario);
        let path = output.join(format!("{scenario}.svg"));
        fs::write(&path, svg::render(&case)).expect("unable to write SVG preview");
        println!("generated {}", path.display());
    }
}
