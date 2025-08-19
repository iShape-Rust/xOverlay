use crate::core::fill::InclusionFilterStrategy;
use crate::core::link::OverlayLink;
use crate::ortho::column::Column;

impl<C> Column<C> {
    pub(crate) fn count_included_links<F: InclusionFilterStrategy>(&self) -> usize {
        let mut count = 0;
        for &fill in self.vr_fills.iter() {
            if F::is_included(fill) {
                count += 1;
            }
        }
        for &fill in self.hz_fills.iter() {
            if F::is_included(fill) {
                count += 1;
            }
        }
        count
    }

    pub(crate) fn copy_links_into<F: InclusionFilterStrategy>(&self, target: &mut [OverlayLink]) {
        let mut index = 0;
        for (vr, &fill) in self.vr_segments.iter().zip(&self.vr_fills) {
            if !F::is_included(fill) {
                continue;
            }
            unsafe {
                *target.get_unchecked_mut(index) = OverlayLink::with_vr(vr.pos, vr.min, vr.max, fill);
            }
            index += 1;
        }
        for (hz, &fill) in self.hz_segments.iter().zip(&self.hz_fills) {
            if !F::is_included(fill) {
                continue;
            }
            unsafe {
                *target.get_unchecked_mut(index) = OverlayLink::with_hz(hz.pos, hz.min, hz.max, fill);
            }
            index += 1;
        }
    }
}