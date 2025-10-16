use crate::gear::segment::Segment;
use alloc::vec::Vec;

pub(super) struct GeometrySource {
    pub(super) vr_list: Vec<Segment>,
    pub(super) hz_list: Vec<Segment>,
    pub(super) dp_list: Vec<Segment>,
    pub(super) dn_list: Vec<Segment>,
}