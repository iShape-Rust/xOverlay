# xOverlay
Polygon boolean library for 45 degrees geometry.

Currently in prototype stage.

Integer contours are supported for `i16`, `i32`, and `i64` coordinates. The coordinate type is
explicit on the overlay engine and is preserved in the result:

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
