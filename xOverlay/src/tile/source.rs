use crate::gear::segment::Segment;
use crate::tile::mapper::TilePart;
use alloc::vec::Vec;

pub(super) struct GeometrySource {
    pub(super) vr_list: Vec<Segment>,
    pub(super) hz_list: Vec<Segment>,
    pub(super) dp_list: Vec<Segment>,
    pub(super) dn_list: Vec<Segment>,
}

impl GeometrySource {
    #[inline]
    pub(super) fn with_part(part: TilePart) -> Self {
        Self {
            vr_list: Vec::with_capacity(part.count_vr),
            hz_list: Vec::with_capacity(part.count_hz),
            dp_list: Vec::with_capacity(part.count_dp),
            dn_list: Vec::with_capacity(part.count_dn),
        }
    }
}