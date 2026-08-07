use x_overlay::core::validation::{Contour, ContourValidationError, ContourWarning};
use x_overlay::i_float::int::point::IntPoint;

#[test]
fn rectangle_is_valid_without_warnings() {
    let contour = [
        IntPoint::new(0_i32, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
        IntPoint::new(0, 10),
    ];

    assert_eq!(contour.as_slice().validate(), Ok(vec![]));
}

#[test]
fn repeated_closing_point_produces_a_zero_length_warning() {
    let contour = [
        IntPoint::new(0_i32, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
        IntPoint::new(0, 10),
        IntPoint::new(0, 0),
    ];

    assert_eq!(
        contour.as_slice().validate(),
        Ok(vec![ContourWarning::ZeroLengthEdge { start: 4, end: 0 }])
    );
}

#[test]
fn too_few_vertices_are_rejected() {
    let contour = [
        IntPoint::new(0_i32, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
    ];

    assert_eq!(
        contour.as_slice().validate(),
        Err(ContourValidationError::TooFewVertices { count: 3 })
    );
}

#[test]
fn diagonal_closing_edge_is_rejected() {
    let contour = [
        IntPoint::new(0_i32, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
        IntPoint::new(5, 10),
    ];

    assert_eq!(
        contour.as_slice().validate(),
        Err(ContourValidationError::NonOrthogonalEdge { start: 3, end: 0 })
    );
}

#[test]
fn zero_length_edge_produces_a_warning() {
    let contour = [
        IntPoint::new(0_i32, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
        IntPoint::new(0, 10),
    ];

    assert_eq!(
        contour.as_slice().validate(),
        Ok(vec![ContourWarning::ZeroLengthEdge { start: 1, end: 2 }])
    );
}

#[test]
fn collinear_vertex_is_reported_as_a_warning() {
    let contour = [
        IntPoint::new(0_i32, 0),
        IntPoint::new(5, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
        IntPoint::new(0, 10),
    ];

    assert_eq!(
        contour.as_slice().validate(),
        Ok(vec![ContourWarning::CollinearVertex { index: 1 }])
    );
}

#[test]
fn custom_contour_can_reuse_the_validator() {
    struct CustomContour([IntPoint<i32>; 4]);

    impl Contour<i32> for CustomContour {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn point(&self, index: usize) -> IntPoint<i32> {
            self.0[index]
        }
    }

    let contour = CustomContour([
        IntPoint::new(0, 0),
        IntPoint::new(10, 0),
        IntPoint::new(10, 10),
        IntPoint::new(0, 10),
    ]);

    assert_eq!(contour.validate(), Ok(vec![]));
}
