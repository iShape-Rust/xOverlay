use crate::core::fill_rule::FillRule;
use crate::core::overlay_rule::OverlayRule;
use i_overlay::core::fill_rule::FillRule as IFillRule;
use i_overlay::core::overlay::Overlay as IOverlay;
use i_overlay::core::overlay_rule::OverlayRule as IOverlayRule;
use i_shape::int::shape::{IntContour, IntShapes};

impl From<FillRule> for IFillRule {
    fn from(value: FillRule) -> Self {
        match value {
            FillRule::EvenOdd => Self::EvenOdd,
            FillRule::NonZero => Self::NonZero,
            FillRule::Positive => Self::Positive,
            FillRule::Negative => Self::Negative,
        }
    }
}

impl From<OverlayRule> for IOverlayRule {
    fn from(value: OverlayRule) -> Self {
        match value {
            OverlayRule::Subject => Self::Subject,
            OverlayRule::Clip => Self::Clip,
            OverlayRule::Intersect => Self::Intersect,
            OverlayRule::Union => Self::Union,
            OverlayRule::Difference => Self::Difference,
            OverlayRule::InverseDifference => Self::InverseDifference,
            OverlayRule::Xor => Self::Xor,
        }
    }
}

pub(crate) fn i_overlay_shapes(
    subject: &[IntContour<i32>],
    clip: &[IntContour<i32>],
    fill_rule: FillRule,
    overlay_rule: OverlayRule,
) -> IntShapes<i32> {
    let mut overlay = IOverlay::with_contours(subject, clip);
    overlay.overlay(overlay_rule.into(), fill_rule.into())
}
