use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_shape::int::shape::{IntContour, IntShapes};

pub struct RandomTestX1;

impl RandomTestX1 {

    pub fn run(subj: &[IntContour<i32>]) -> IntShapes<i32> {
        let overlay = Overlay::with_contours(subj, &[]);
        overlay.overlay(OverlayRule::Subject, FillRule::NonZero)
    }
}
