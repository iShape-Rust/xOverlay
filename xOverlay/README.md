# xOverlay

[![status: prototype](https://img.shields.io/badge/status-prototype-orange.svg)](#project-status)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

xOverlay is a high-performance polygon Boolean engine specialized for orthogonal (Manhattan) geometry. It processes contours made exclusively of horizontal and vertical edges using integer coordinates.

> [!WARNING]
> xOverlay is currently a prototype. Its API may change between revisions.

## Table of Contents

- [Why xOverlay?](#why-xoverlay)
- [Features](#features)
- [Getting Started](#getting-started)
  - [Quick Start](#quick-start)
- [Boolean Operations](#boolean-operations)
- [Fill Rules](#fill-rules)
- [Input and Output](#input-and-output)
  - [Input Requirements](#input-requirements)
  - [Validation and Custom Contours](#validation-and-custom-contours)
  - [Output Structure](#output-structure)
  - [Flat Contour Output](#flat-contour-output)
- [Integer Types](#integer-types)
  - [Winding Count Type](#winding-count-type)
- [Multithreading](#multithreading)
- [xOverlay and iOverlay](#xoverlay-and-ioverlay)
- [Project Status](#project-status)
- [License](#license)

&nbsp;
## Why xOverlay?

General-purpose polygon clipping algorithms spend time handling arbitrary segment intersections. Orthogonal geometry has stronger constraints: every edge is horizontal or vertical. xOverlay is built around those constraints and is intended for large Manhattan-style workloads such as layouts, grids, tiled maps, and rectilinear CAD data.

Use xOverlay when all input edges are axis-aligned and integer coordinates fit your pipeline. For arbitrary angles or floating-point input, use [iOverlay](https://github.com/iShape-Rust/iOverlay).

&nbsp;
## Features

- **Boolean operations**: union, intersection, difference, inverse difference, and XOR.
- **Single-input extraction**: resolve a subject or clip using its fill rule.
- **Orthogonal geometry**: a solver specialized for horizontal and vertical edges.
- **Multiple contours and holes**: output can be returned as shapes with their holes or as a flat contour list.
- **Fill rules**: even-odd, non-zero, positive, and negative.
- **Integer coordinates**: `i16`, `i32`, and `i64`, preserved from input to output.
- **Configurable winding counts**: `i16` by default, with `i32` and `i64` available for deep overlap.
- **Optional parallel execution**: enabled by default for large inputs.
- **`no_std` core**: available when default features are disabled.

&nbsp;
## Getting Started

While xOverlay is in the prototype stage, add it directly from the repository:

```toml
[dependencies]
x_overlay = { git = "https://github.com/iShape-Rust/xOverlay.git", branch = "main" }
```

### Quick Start

```rust
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;

let subject = vec![vec![
    IntPoint::new(0_i32, 0),
    IntPoint::new(8, 0),
    IntPoint::new(8, 6),
    IntPoint::new(0, 6),
]];

let clip = vec![vec![
    IntPoint::new(4_i32, 2),
    IntPoint::new(10, 2),
    IntPoint::new(10, 8),
    IntPoint::new(4, 8),
]];

let result = Overlay::<i32>::with_contours(&subject, &clip)
    .overlay(FillRule::NonZero, OverlayRule::Intersect);

assert_eq!(result.len(), 1);
assert_eq!(result[0].len(), 1);
```

Contours are closed automatically; do not repeat the first point at the end.

&nbsp;
## Boolean Operations

For subject `A` and clip `B`, xOverlay supports:

| Rule | Result |
|---|---|
| `OverlayRule::Subject` | Filled area of `A` |
| `OverlayRule::Clip` | Filled area of `B` |
| `OverlayRule::Intersect` | `A ∩ B` |
| `OverlayRule::Union` | `A ∪ B` |
| `OverlayRule::Difference` | `A − B` |
| `OverlayRule::InverseDifference` | `B − A` |
| `OverlayRule::Xor` | Area belonging to exactly one of `A` or `B` |

`Subject` and `Clip` are also useful for resolving self-overlapping contour sets according to the selected fill rule.

&nbsp;
## Fill Rules

The fill rule determines which regions of a contour set are considered filled:

| Rule | Filled region |
|---|---|
| `FillRule::EvenOdd` | Regions with an odd crossing count |
| `FillRule::NonZero` | Regions with a non-zero winding number |
| `FillRule::Positive` | Regions with a positive winding number |
| `FillRule::Negative` | Regions with a negative winding number |

`NonZero` is the default choice for consistently oriented outer contours and holes. `EvenOdd` is useful when contour orientation should not affect nesting.

&nbsp;
## Input and Output

### Input Requirements

Each subject and clip is a slice of contours. A contour is a `Vec<IntPoint<I>>` and must follow these rules:

- Every edge, including the closing edge, must be horizontal or vertical: consecutive points must have the same `x` or the same `y` coordinate.
- A contour must contain at least four points.
- The first point does not need to be repeated at the end.
- Coordinates must use one supported integer type consistently.

xOverlay's core solver assumes valid orthogonal input. It does not validate contour length,
axis alignment, closing-edge alignment, or other structural invariants, and it does not return
validation errors. Supplying invalid contours may produce invalid geometry.

### Validation and Custom Contours

Input validation can be provided as a separate checked wrapper around the core solver. This keeps
validation policy out of the performance-oriented path while allowing applications that receive
untrusted or externally generated geometry to validate it before constructing an `Overlay`.

A contour trait can also be added as an adapter layer for custom contour representations. Such a
trait could expose point iteration and reusable orthogonal validation without coupling the solver
to a particular container type. The checked wrapper and contour trait are possible extensions;
they are not part of the current API.

### Output Structure

`Overlay::overlay` returns `IntShapes<I>`, which is equivalent to:

```text
Vec<                 // shapes
    Vec<             // contours in one shape
        Vec<Point>   // points in one contour
    >
>
```

For each shape:

- the first contour is the outer boundary;
- remaining contours are holes;
- by default, outer boundaries are counterclockwise and holes are clockwise.

### Flat Contour Output

When shape-to-hole grouping is not needed, use `overlay_contours`:

```rust
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;

let subject = vec![vec![
    IntPoint::new(0_i32, 0),
    IntPoint::new(8, 0),
    IntPoint::new(8, 6),
    IntPoint::new(0, 6),
]];
let clip: Vec<Vec<IntPoint<i32>>> = Vec::new();

let contours = Overlay::<i32>::with_contours(&subject, &clip)
    .overlay_contours(FillRule::NonZero, OverlayRule::Subject);
```

The returned `Vec<IntContour<I>>` keeps outer boundaries and holes separate; their winding direction distinguishes them.

&nbsp;
## Integer Types

xOverlay supports `i16`, `i32`, and `i64`. The coordinate type is explicit on `Overlay` and is preserved in the result:

```rust
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;

let subject = vec![vec![
    IntPoint::<i64>::new(5_000_000_000, 0),
    IntPoint::new(5_000_000_010, 0),
    IntPoint::new(5_000_000_010, 10),
    IntPoint::new(5_000_000_000, 10),
]];

let result = Overlay::<i64>::with_contours(&subject, &[])
    .overlay(FillRule::NonZero, OverlayRule::Subject);

assert_eq!(result.len(), 1);
```

### Winding Count Type

The coordinate type and winding-count type are independent:

```rust
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_float::int::point::IntPoint;

let subject = vec![vec![
    IntPoint::new(0_i64, 0),
    IntPoint::new(10, 0),
    IntPoint::new(10, 10),
    IntPoint::new(0, 10),
]];

// i64 coordinates with i32 winding counts.
let result = Overlay::<i64, i32>::with_contours(&subject, &[])
    .overlay(FillRule::NonZero, OverlayRule::Subject);

assert_eq!(result.len(), 1);
```

`Overlay<I>` uses `i16` winding counts by default. This is the most compact option and is
appropriate when every intermediate subject and clip winding count remains within
`-32768..=32767`. Choose `Overlay<I, i32>` or `Overlay<I, i64>` for inputs with deeper nesting or
more overlapping contours. Winding arithmetic follows the overflow behavior of the selected Rust
integer type.

&nbsp;
## Multithreading

The `allow_multithreading` feature is enabled by default. `Overlay::with_contours` automatically chooses the available parallelism. Use the custom constructor when execution must be controlled explicitly:

```rust
use x_overlay::core::cpu_count::CPUCount;
use x_overlay::core::overlay::Overlay;
use x_overlay::i_float::int::point::IntPoint;

let subject = vec![vec![
    IntPoint::new(0_i32, 0),
    IntPoint::new(8, 0),
    IntPoint::new(8, 6),
    IntPoint::new(0, 6),
]];
let clip: Vec<Vec<IntPoint<i32>>> = Vec::new();

let overlay = Overlay::<i32>::with_contours_custom(
    &subject,
    &clip,
    Default::default(),
    CPUCount::Single,
);
```

Disable default features for a serial `no_std` build:

```toml
[dependencies]
x_overlay = { git = "https://github.com/iShape-Rust/xOverlay.git", branch = "main", default-features = false }
```

See the published [interactive performance report](https://ishape-rust.github.io/xOverlay/performance/)
for deterministic iOverlay, xOverlay, and Boost Polygon 90 comparisons. The generators,
raw JSON, scripts, and usage guide live in the repository's
[`performance/benchmark`](../performance/benchmark) directory.

&nbsp;
## xOverlay and iOverlay

| | xOverlay | [iOverlay](https://github.com/iShape-Rust/iOverlay) |
|---|---|---|
| Edge directions | Horizontal and vertical only | Arbitrary |
| Coordinate APIs | `i16`, `i32`, `i64` | Integer and floating point |
| Primary goal | Maximum throughput on orthogonal geometry | General-purpose robust polygon overlays |
| Project maturity | Prototype | Production-ready |

If geometry may contain diagonal edges, use iOverlay.

&nbsp;
## Project Status

xOverlay is under active development and is not yet API-stable. Before adopting it in production, pin a tested revision and validate the output against your workloads.

&nbsp;
## License

xOverlay is distributed under the [MIT License](../LICENSE).
