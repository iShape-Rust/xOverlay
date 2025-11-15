use alloc::vec::Vec;
use crate::definition::segment::SegmentFill;

#[derive(Clone)]
pub(super) struct FillSource {
    pub(super) vr: Vec<SegmentFill>,
    pub(super) hz: Vec<SegmentFill>,
    pub(super) dp: Vec<SegmentFill>,
    pub(super) dn: Vec<SegmentFill>,
}