use crate::core::fill_rule::FillRule;
use crate::core::options::IntOverlayOptions;
use crate::core::overlay_rule::OverlayRule;
use crate::core::solver::Solver;
use crate::gear::section::Section;
use crate::gear::x_layout::XLayout;
use alloc::vec::Vec;
use i_shape::flat::buffer::FlatContoursBuffer;
use i_shape::int::count::IntShapes;
use i_shape::int::shape::IntContour;

#[derive(Debug)]
pub enum OverlayError {
    NotValidPath,
    EmptyPath,
}

/// This struct is essential for describing and uploading the geometry or shapes required to construct an `OverlayGraph`. It prepares the necessary data for boolean operations.
pub struct Overlay {
    pub options: IntOverlayOptions,
    pub solver: Solver,
    pub(crate) layout: XLayout,
    pub(crate) sections: Vec<Section>,
}

impl Overlay {
    #[inline]
    pub fn with_contours(subj: &[IntContour], clip: &[IntContour]) -> Result<Self, OverlayError> {
        Self::with_contours_custom(subj, clip, Default::default(), Default::default())
    }

    #[inline]
    pub fn with_contours_custom(
        subj: &[IntContour],
        clip: &[IntContour],
        options: IntOverlayOptions,
        solver: Solver,
    ) -> Result<Self, OverlayError> {
        Self::init_contours_custom(subj, clip, options, solver)
    }

    #[inline]
    pub fn overlay(&mut self, fill_rule: FillRule, overlay_rule: OverlayRule) -> IntShapes {
        self.process_overlay(fill_rule, overlay_rule)
            .extract_shapes(overlay_rule)
    }

    #[inline]
    pub fn overlay_into(
        &mut self,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
        output: &mut FlatContoursBuffer,
    ) {
        self.process_overlay(fill_rule, overlay_rule)
            .extract_contours_into(overlay_rule, output);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::core::fill_rule::FillRule;
    use crate::core::overlay::Overlay;
    use crate::core::overlay_rule::OverlayRule;
    use alloc::vec;
    use i_float::int::point::IntPoint;
    use i_shape::int::area::Area;

    #[test]
    fn test_0() {
        let subj = [[
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]
        .to_vec()];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::EvenOdd, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].len(), 4);
        assert_eq!(shape[0].area(), -100);
    }

    #[test]
    fn test_1() {
        let subj = [
            vec![
                IntPoint::new(0, 0),
                IntPoint::new(5, 0),
                IntPoint::new(5, 10),
                IntPoint::new(0, 10),
            ],
            vec![
                IntPoint::new(5, 0),
                IntPoint::new(10, 0),
                IntPoint::new(10, 10),
                IntPoint::new(5, 10),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::EvenOdd, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].len(), 4);
        assert_eq!(shape[0].area(), -100);
    }

    #[test]
    fn test_2() {
        let subj = [
            vec![
                IntPoint::new(1, 1),
                IntPoint::new(1, 3),
                IntPoint::new(3, 3),
                IntPoint::new(3, 1),
            ],
            vec![
                IntPoint::new(0, 0),
                IntPoint::new(4, 0),
                IntPoint::new(4, 4),
                IntPoint::new(0, 4),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::EvenOdd, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 2);
        assert_eq!(shape[0].len(), 4);
        assert_eq!(shape[0].area(), -16);
        assert_eq!(shape[1].len(), 4);
        assert_eq!(shape[1].area(), 4);
    }

    #[test]
    fn test_3() {
        let subj = [
            vec![
                IntPoint::new(0, 5),
                IntPoint::new(3, 5),
                IntPoint::new(3, 7),
                IntPoint::new(0, 7),
            ],
            vec![
                IntPoint::new(0, 1),
                IntPoint::new(1, 1),
                IntPoint::new(1, 6),
                IntPoint::new(0, 6),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::NonZero, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].area(), -10);
    }

    #[test]
    fn test_4() {
        let subj = [
            vec![
                IntPoint::new(5, 5),
                IntPoint::new(6, 5),
                IntPoint::new(6, 7),
                IntPoint::new(5, 7),
            ],
            vec![
                IntPoint::new(3, 1),
                IntPoint::new(6, 1),
                IntPoint::new(6, 6),
                IntPoint::new(3, 6),
            ],
            vec![
                IntPoint::new(6, 6),
                IntPoint::new(7, 6),
                IntPoint::new(7, 7),
                IntPoint::new(6, 7),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::NonZero, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].area(), -17);
    }

    #[test]
    fn test_5() {
        let subj = [
            vec![
                IntPoint::new(0, 1),
                IntPoint::new(4, 1),
                IntPoint::new(4, 3),
                IntPoint::new(0, 3),
            ],
            vec![
                IntPoint::new(4, 0),
                IntPoint::new(6, 0),
                IntPoint::new(6, 4),
                IntPoint::new(4, 4),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::NonZero, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].area(), -16);
    }

    #[test]
    fn test_6() {
        let subj = [
            vec![
                IntPoint::new(6, 0),
                IntPoint::new(7, 0),
                IntPoint::new(7, 6),
                IntPoint::new(6, 6),
            ],
            vec![
                IntPoint::new(3, 2),
                IntPoint::new(6, 2),
                IntPoint::new(6, 4),
                IntPoint::new(3, 4),
            ],
            vec![
                IntPoint::new(0, 0),
                IntPoint::new(6, 0),
                IntPoint::new(6, 3),
                IntPoint::new(0, 3),
            ],
            vec![
                IntPoint::new(1, 0),
                IntPoint::new(3, 0),
                IntPoint::new(3, 2),
                IntPoint::new(1, 2),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::NonZero, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].area(), -27);
    }

    #[test]
    fn test_7() {
        let subj = [
            vec![
                IntPoint::new(5, 1),
                IntPoint::new(7, 1),
                IntPoint::new(7, 7),
                IntPoint::new(5, 7),
            ],
            vec![
                IntPoint::new(5, 3),
                IntPoint::new(7, 3),
                IntPoint::new(7, 5),
                IntPoint::new(5, 5),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::NonZero, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].area(), -12);
    }

    #[test]
    fn test_8() {
        let subj = [
            vec![
                IntPoint::new(6, 4),
                IntPoint::new(7, 4),
                IntPoint::new(7, 5),
                IntPoint::new(6, 5),
            ],
            vec![
                IntPoint::new(3, 4),
                IntPoint::new(7, 4),
                IntPoint::new(7, 5),
                IntPoint::new(3, 5),
            ],
        ];

        let mut overlay = Overlay::with_contours(&subj, &[]).expect("create");
        let result = overlay.overlay(FillRule::NonZero, OverlayRule::Subject);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].area(), -4);
    }
}
