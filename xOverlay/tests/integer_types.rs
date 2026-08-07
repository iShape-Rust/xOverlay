use i_float::int::point::IntPoint;
use i_shape::int::area::Area;
use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;

#[test]
fn overlay_supports_i16_coordinates() {
    let subject = vec![vec![
        IntPoint::<i16>::new(-8, -4),
        IntPoint::<i16>::new(8, -4),
        IntPoint::<i16>::new(8, 4),
        IntPoint::<i16>::new(-8, 4),
    ]];

    let shapes = Overlay::<i16>::with_contours(&subject, &[])
        .overlay(FillRule::NonZero, OverlayRule::Subject);

    assert_eq!(shapes.area_two(), 256i32);
}

#[test]
fn overlay_preserves_i64_coordinates_outside_i32_range() {
    const BASE: i64 = 5_000_000_000;
    let subject = vec![vec![
        IntPoint::new(BASE, BASE),
        IntPoint::new(BASE + 20, BASE),
        IntPoint::new(BASE + 20, BASE + 20),
        IntPoint::new(BASE, BASE + 20),
    ]];
    let clip = vec![vec![
        IntPoint::new(BASE + 10, BASE + 5),
        IntPoint::new(BASE + 30, BASE + 5),
        IntPoint::new(BASE + 30, BASE + 15),
        IntPoint::new(BASE + 10, BASE + 15),
    ]];

    let shapes = Overlay::<i64>::with_contours(&subject, &clip)
        .overlay(FillRule::NonZero, OverlayRule::Intersect);

    assert_eq!(shapes.area_two(), 200i128);
    assert!(
        shapes
            .iter()
            .flatten()
            .flatten()
            .all(|point| point.x > i32::MAX as i64)
    );
}

#[test]
fn overlay_accepts_empty_i64_input() {
    let empty: [Vec<IntPoint<i64>>; 0] = [];
    let shapes = Overlay::<i64>::with_contours(&empty, &empty)
        .overlay(FillRule::NonZero, OverlayRule::Union);

    assert!(shapes.is_empty());
}

#[test]
fn overlay_supports_i32_winding_counts() {
    let subject = vec![vec![
        IntPoint::new(0_i32, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
        IntPoint::new(0, 10),
    ]];

    let shapes = Overlay::<i32, i32>::with_contours(&subject, &[])
        .overlay(FillRule::NonZero, OverlayRule::Subject);

    assert_eq!(shapes.area_two(), 200i64);
}
