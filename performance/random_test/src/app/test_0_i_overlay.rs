use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::Overlay;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::i_shape::int::count::IntShapes;
use i_overlay::i_shape::int::shape::IntContour;

pub struct RandomTestI1;

impl RandomTestI1 {

    pub fn run(subj: &[IntContour]) -> IntShapes {
        let mut overlay = Overlay::with_contours(subj, &[]);
        overlay.overlay(OverlayRule::Subject, FillRule::NonZero)
    }
}