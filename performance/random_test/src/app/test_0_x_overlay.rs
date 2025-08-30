use x_overlay::core::fill_rule::FillRule;
use x_overlay::core::overlay::Overlay;
use x_overlay::core::overlay_rule::OverlayRule;
use x_overlay::i_shape::int::shape::{IntContour, IntShapes};

pub struct RandomTestX1;

impl RandomTestX1 {

    pub fn run(subj: &[IntContour]) -> IntShapes {
        let mut overlay = Overlay::with_contours(subj, &[]).expect("valid");
        overlay.overlay(FillRule::NonZero, OverlayRule::Subject)
    }
}