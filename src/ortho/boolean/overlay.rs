use alloc::vec;
use i_shape::int::shape::IntShapes;
use crate::core::fill_rule::FillRule;
use crate::core::overlay_rule::OverlayRule;
use crate::graph::boolean::winding_count::ShapeCountBoolean;
use crate::ortho::overlay::OrthoOverlay;

impl OrthoOverlay<ShapeCountBoolean> {
    pub fn overlay(&mut self, overlay_rule: OverlayRule, fill_rule: FillRule) -> IntShapes {
        self.build_custom_graph(fill_rule, overlay_rule);
        if let Some(graph) = &mut self.graph {
            graph.extract_shapes(overlay_rule)
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::ortho::overlay::OrthoOverlay;
    use i_float::int::point::IntPoint;
    use i_shape::int::area::Area;
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay_rule::OverlayRule;
    use crate::graph::boolean::winding_count::ShapeCountBoolean;

    #[test]
    fn test_0() {
        let mut overlay = OrthoOverlay::<ShapeCountBoolean>::default();

        let subj = [[
            IntPoint::new(0, 0),
            IntPoint::new(10, 0),
            IntPoint::new(10, 10),
            IntPoint::new(0, 10),
        ]
            .to_vec()];

        overlay.init_with_ortho_contours(&subj, &[]).expect("OK");
        let result = overlay.overlay(OverlayRule::Subject, FillRule::EvenOdd);

        assert_eq!(result.len(), 1);
        let shape = &result[0];
        assert_eq!(shape.len(), 1);
        assert_eq!(shape[0].len(), 4);
        assert_eq!(shape[0].area(), 100);
    }
}