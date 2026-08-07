# Orthogonal overlay benchmark

This crate provides one deterministic performance suite for:

- iOverlay, single-threaded and multithreaded;
- xOverlay, single-threaded and multithreaded;
- Boost Polygon 90, single-threaded;
- `i32` and translated `i64` coordinates;
- grouped shape output and flat contour output.

Geometry is generated once in Rust. The exact same contours are used by both
Rust implementations, the C++ Boost adapter, and the SVG illustration tool.
The unrelated [`../random_test`](../random_test) project is intentionally kept
separate.

## Requirements

- Rust toolchain supported by iOverlay and xOverlay;
- Clang with C++20 support;
- Boost headers.

On macOS, the scripts discover a Homebrew Boost installation automatically.
For another installation, set `BOOST_ROOT` to the Boost prefix containing the
`include/boost` directory.

## Scripts

All user-facing scripts live together in [`scripts`](scripts):

| Script | Purpose |
|---|---|
| `build.sh` | Builds all Rust tools and the C++ Boost Polygon 90 runner. |
| `run.sh` | Runs the benchmark, writes JSON, regenerates SVGs, and rebuilds the page. |
| `generate-svgs.sh` | Recreates the six centered, uncropped scenario illustrations. |
| `render.sh` | Rebuilds `site/index.html` from a result JSON file and the page template. |

## Quick start

Run the complete suite:

```sh
./performance/benchmark/scripts/run.sh
```

The maximum `N` was selected once using xOverlay single-threaded with `i64`
shape output: it is a power of two whose operation completed in at most four
seconds. Benchmark runs use these fixed values and do not recalibrate them:

| Scenario | Maximum `N` |
|---|---:|
| Checkerboard | 2048 |
| Not overlap | 2048 |
| Lines net | 4096 |
| Orthogonal wind mill | 1024 |
| Windows | 2048 |
| Nested squares | 16384 |

Each scenario benchmarks the ascending series `N/64`, `N/16`, `N/4`, `N`.
The same series is used for both coordinate widths and both output forms, and
is stored in JSON as `metadata.scenario_sizes`.

The command writes:

- raw benchmark data to [`results/latest.json`](results/latest.json);
- generated illustrations to [`site/assets`](site/assets);
- the standalone report to [`site/index.html`](site/index.html).

Open `site/index.html` directly in a browser. Its JSON data, CSS, and JavaScript
are embedded, so a local web server is not required.

## Smoke run

Use the small profile while changing generators or adapters:

```sh
BENCH_PROFILE=smoke \
BENCH_BUDGET_MS=50 \
BENCH_SAMPLES=3 \
./performance/benchmark/scripts/run.sh
```

The defaults can be adjusted through these environment variables:

| Variable | Default | Meaning |
|---|---:|---|
| `BENCH_MACHINE` | auto | Overrides the machine label in the report. |
| `BENCH_SOLVER_TIMEOUT_SECONDS` | `60` | Per-measurement timeout for every solver. |
| `BENCH_BUDGET_MS` | `700` | Timing budget per benchmark measurement. |
| `BENCH_SAMPLES` | `9` | Number of timing samples. |

Machine detection uses `sysctl` first and falls back to the safe `Model Name`,
`Chip`, and `Memory` fields from `system_profiler`. Set `BENCH_MACHINE` to use a
manual label. JSON records its origin in `metadata.machine_source` as
`user_set`, `sysctl`, `system_profiler`, or `unknown`.

If any solver reaches its timeout, the report records the point as `timed_out`
and shows it as greater than the configured limit. Remaining runs for that
solver and scenario are skipped; other solvers and subsequent scenarios
continue. Rust measurements run in isolated child processes so timed-out work
is terminated rather than left running in the background.

Run one scenario by forwarding benchmark arguments:

```sh
BENCH_PROFILE=smoke ./performance/benchmark/scripts/run.sh \
  --scenario checkerboard
```

Available scenario names are:

- `checkerboard`
- `not_overlap`
- `lines_net`
- `wind_mill`
- `windows`
- `nested_squares`

## Rebuild only the report

To render the checked-in result without rerunning benchmarks:

```sh
./performance/benchmark/scripts/render.sh
```

Or provide another compatible JSON file:

```sh
./performance/benchmark/scripts/render.sh /absolute/path/to/results.json
```

## Measurement contract

Every timed operation includes:

1. loading the already-generated contours into the solver;
2. applying the selected Boolean operation with the non-zero fill rule;
3. materializing either grouped shapes or flat contours.

Scenario generation, binary transfer to the Boost process, validation, JSON
serialization, and SVG rendering are outside the timed region. The report uses
the median of raw nanoseconds-per-operation samples and retains every sample in
JSON.

The `i64` suite translates the same topology by `2^40`, ensuring that the
64-bit coordinate path cannot be represented as `i32`.

## Boost contour semantics

For grouped shapes, Boost emits `polygon_90_with_holes_data`, which corresponds
to the shape-with-holes output used by iOverlay and xOverlay.

For contours, the benchmark deliberately records Boost's native flat
`polygon_90_data` output. Boost may fracture geometry around holes into more
simple polygons. The covered region and area remain comparable, but contour
and point counts are not necessarily equivalent. JSON records the semantics as
`native_flat_polygons`, and the report marks Boost with a warning and a dashed
series.

## Report sources

The editable report files are in [`site/template`](site/template). The renderer
embeds the selected JSON, stylesheet, and JavaScript into a standalone
`site/index.html`. Scenario SVGs are generated from the benchmark geometry;
they should not be edited by hand.
