// use crate::core::fill::InclusionFilterStrategy;
// use crate::graph::link::OverlayLink;
// use crate::ortho::section::OthoSection;
// 
// gear<C> OthoSection<C> {
//     pub(crate) fn count_included_links<F: InclusionFilterStrategy>(&self) -> usize {
//         let mut count = 0;
//         for &fill in self.vr_fills.iter() {
//             if F::is_included(fill) {
//                 count += 1;
//             }
//         }
//         for &fill in self.hz_fills.iter() {
//             if F::is_included(fill) {
//                 count += 1;
//             }
//         }
//         count
//     }
// 
//     pub(crate) fn copy_links_into_with_filter<F: InclusionFilterStrategy>(&self, target: &mut [OverlayLink]) {
//         debug_assert_eq!(target.len(), self.count_included_links::<F>());
//         let mut it = target.iter_mut();
// 
//         for (vr, &fill) in self.vr_segments.iter().zip(&self.vr_fills) {
//             if F::is_included(fill) {
//                 if let Some(slot) = it.next() {
//                     *slot = OverlayLink::with_vr(vr.pos, vr.min, vr.max, fill);
//                 } else {
//                     debug_assert!(false, "iterator underrun");
//                     break;
//                 }
//             }
//         }
// 
//         for (hz, &fill) in self.hz_segments.iter().zip(&self.hz_fills) {
//             if F::is_included(fill) {
//                 if let Some(slot) = it.next() {
//                     *slot = OverlayLink::with_hz(hz.pos, hz.min, hz.max, fill);
//                 } else {
//                     debug_assert!(false, "iterator underrun");
//                     break;
//                 }
//             }
//         }
// 
//         debug_assert!(it.next().is_none(), "iterator overrun");
//     }
// }