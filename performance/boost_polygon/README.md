# Orthogonal overlay comparison

This subproject compares xOverlay, current iOverlay, and Boost.Polygon's
Manhattan-specific solver on deterministic orthogonal workloads from the
[iShape performance suite](https://ishape-rust.github.io/iShape-js/overlay/performance/performance.html).

## Boost implementation used here

Boost contains separate geometry libraries and several polygon-set data
structures. This benchmark intentionally uses:

```cpp
boost::polygon::polygon_90_set_data<std::int32_t>
```

It does **not** use Boost.Geometry, `polygon_set_data`, or
`polygon_45_set_data`. The C++ runner has a compile-time assertion that its
geometry concept is `polygon_90_set_concept`, constructs sets with
`HORIZONTAL` scan orientation, verifies that orientation at runtime, and
prints this configuration at startup.

The result is materialized as `polygon_90_with_holes_data<int32_t>`, matching
the structured hull-with-holes output produced by xOverlay and iOverlay more
closely than Boost's fractured plain-polygon output.

The Boost column on the published performance page predates this exact setup.
The corresponding benchmark repository used `polygon_45_set_data` in its
published-era runner; explicit `polygon_90_set_data` support was added in
[August 2025](https://github.com/iShape-Rust/iOverlayPerformance/commit/da5c733b73c9cf333e0c32f473837dc896273b18).

## Workloads

The runner currently includes:

| Scenario | Default size | Operation |
|---|---:|---|
| Checkerboard | N=128, 32,513 inputs | XOR |
| Not Overlap | N=128, 32,513 inputs | Union |
| Lines Net | N=128, 256 inputs | Intersection |
| Windows | N=128, 32,768 inputs | Difference |
| Nested Squares | N=4096, 16,384 inputs | XOR |

All inputs are rectangles. Before timing, every implementation's output is
checked against an independently derived analytical area.

## Measurements

The Rust runner reports:

- xOverlay with `CPUCount::Single`;
- xOverlay with `CPUCount::Auto`;
- current iOverlay.

The C++ runner reports:

- **Boost 90 end-to-end**: insert rectangles into both sets, boolean operation,
  and materialize output;
- **Boost 90 prepared**: reuse prepared input sets, perform the boolean
  operation, and materialize output.

## Prerequisites

On macOS with Homebrew:

```sh
brew install boost
```

Boost.Polygon is header-only. The local sibling checkouts of xOverlay and
iOverlay are used by the Rust runner.

## Build and run

Run the complete comparison with a 600 ms budget per implementation:

```sh
make compare
```

Build and run only the Boost workloads:

```sh
make
make run
```

Run one selected Boost workload:

```sh
./build/boost_polygon_bench \
  --scenario checkerboard \
  --n 128 \
  --budget-ms 5000
```

Available scenarios are `checkerboard`, `not-overlap`, `lines-net`, `windows`,
`nested`, and `all`. The C++ release build uses
`-O3 -DNDEBUG -std=c++20 -march=native`.
