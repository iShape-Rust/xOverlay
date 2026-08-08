use crate::model::{Case, Contour};
use std::fmt::Write as _;

pub fn render(case: &Case<i64>) -> String {
    let (min_x, min_y, max_x, max_y) = bounds(case);
    let geometry_width = (max_x - min_x).max(1) as f64;
    let geometry_height = (max_y - min_y).max(1) as f64;
    let largest = geometry_width.max(geometry_height);
    let stroke = 5.0;
    let padding = largest * 0.09;
    let view_x = min_x as f64 - padding;
    let view_y = min_y as f64 - padding;
    let view_width = geometry_width + 2.0 * padding;
    let view_height = geometry_height + 2.0 * padding;

    let mut svg = String::with_capacity(16_384);
    writeln!(svg, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"720\" height=\"720\" viewBox=\"{view_x:.3} {view_y:.3} {view_width:.3} {view_height:.3}\" preserveAspectRatio=\"xMidYMid meet\" role=\"img\" aria-labelledby=\"title desc\">"
    )
    .unwrap();
    writeln!(svg, "  <title id=\"title\">{}</title>", case.label).unwrap();
    writeln!(
        svg,
        "  <desc id=\"desc\">Symmetric preview of the {} {} benchmark.</desc>",
        case.label, case.operation
    )
    .unwrap();
    writeln!(svg, "  <rect x=\"{view_x:.3}\" y=\"{view_y:.3}\" width=\"{view_width:.3}\" height=\"{view_height:.3}\" fill=\"#fbfaf7\"/>").unwrap();
    write_paths(
        &mut svg,
        &case.subject,
        "subject",
        "#ef4444",
        "#ef4444",
        stroke,
    );
    write_paths(&mut svg, &case.clip, "clip", "#3b82f6", "#3b82f6", stroke);
    writeln!(svg, "</svg>").unwrap();
    svg
}

fn bounds(case: &Case<i64>) -> (i64, i64, i64, i64) {
    let mut points = case.subject.iter().chain(&case.clip).flatten();
    let first = points.next().expect("preview geometry must not be empty");
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    (min_x, min_y, max_x, max_y)
}

fn write_paths(
    svg: &mut String,
    contours: &[Contour<i64>],
    class_name: &str,
    stroke_color: &str,
    fill_color: &str,
    stroke: f64,
) {
    write!(svg, "  <path class=\"{class_name}\" d=\"").unwrap();
    for contour in contours {
        if let Some(first) = contour.first() {
            write!(svg, "M {} {} ", first.x, first.y).unwrap();
            for point in &contour[1..] {
                write!(svg, "L {} {} ", point.x, point.y).unwrap();
            }
            write!(svg, "Z ").unwrap();
        }
    }
    writeln!(
        svg,
        "\" fill=\"{fill_color}\" fill-opacity=\"0.10\" stroke=\"{stroke_color}\" stroke-width=\"{stroke:.3}\" stroke-linejoin=\"round\" vector-effect=\"non-scaling-stroke\"/>",
    )
    .unwrap();
}
