use crate::core::fill::SegmentFill;
use crate::core::overlay_rule::OverlayRule;
use crate::geom::id_point::IdPoint;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OverlayLink {
    pub(crate) a: IdPoint,
    pub(crate) b: IdPoint,
    pub(crate) fill: SegmentFill,
}

impl OverlayLink {
    #[inline(always)]
    pub(crate) fn new(a: IdPoint, b: IdPoint, fill: SegmentFill) -> OverlayLink {
        OverlayLink { a, b, fill }
    }

    #[inline(always)]
    pub(crate) fn other(&self, node_id: usize) -> IdPoint {
        if self.a.id == node_id { self.b } else { self.a }
    }

    #[inline(always)]
    pub(crate) fn is_direct(&self) -> bool {
        self.a.point < self.b.point
    }
}

pub(crate) trait OverlayLinkFilter {
    fn filter_by_overlay(&self, fill_rule: OverlayRule) -> Vec<bool>;
    fn filter_by_overlay_into(&self, overlay_rule: OverlayRule, buffer: &mut Vec<bool>);
}

impl OverlayLink {
    #[inline(always)]
    pub(crate) fn with_vr(x: i32, min_y: i32, max_y: i32, fill: SegmentFill) -> Self {
        Self {
            a: IdPoint {
                id: 0,
                point: IntPoint::new(x, min_y),
            },
            b: IdPoint {
                id: 0,
                point: IntPoint::new(x, max_y),
            },
            fill,
        }
    }

    #[inline(always)]
    pub(crate) fn with_hz(y: i32, min_x: i32, max_x: i32, fill: SegmentFill) -> Self {
        Self {
            a: IdPoint {
                id: 0,
                point: IntPoint::new(min_x, y),
            },
            b: IdPoint {
                id: 0,
                point: IntPoint::new(max_x, y),
            },
            fill,
        }
    }
}
