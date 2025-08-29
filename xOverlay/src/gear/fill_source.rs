use alloc::vec::Vec;
use crate::core::fill::SegmentFill;

pub(super) struct FillSource {
    pub(super) vr: Vec<SegmentFill>,
    pub(super) hz: Vec<SegmentFill>,
    pub(super) dp: Vec<SegmentFill>,
    pub(super) dn: Vec<SegmentFill>,
}