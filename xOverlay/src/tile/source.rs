use alloc::vec::Vec;
use crate::gear::segment::Segment;

pub(crate) struct GeometrySource {
    pub(crate) vr_list: Vec<Segment>,
    pub(crate) hz_list: Vec<Segment>,
    pub(crate) dp_list: Vec<Segment>,
    pub(crate) dn_list: Vec<Segment>,
}