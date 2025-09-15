use crate::core::fill::{InclusionFilterStrategy, SegmentFill};
use crate::core::overlay_rule::OverlayRule;
use crate::gear::fill_source::FillSource;
use crate::gear::filter::{
    ClipFilter, DifferenceFilter, IntersectFilter, InverseDifferenceFilter, SubjectFilter,
    UnionFilter, XorFilter,
};
use crate::gear::section::Section;
use crate::geom::diagonal::{Diagonal, NegativeDiagonal, PositiveDiagonal};
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::sort::two_keys::TwoKeysSort;

#[derive(Clone, Copy)]
pub(super) struct FilledSegment {
    pub(super) a: IntPoint,
    pub(super) b: IntPoint,
    pub(super) fill: SegmentFill,
}

impl FilledSegment {
    #[inline(always)]
    fn new(a: IntPoint, b: IntPoint, fill: SegmentFill) -> Self {
        Self { a, b, fill }
    }
}

impl Section {
    pub(super) fn filled_segments(
        &self,
        overlay_rule: OverlayRule,
        fill_source: FillSource,
    ) -> Vec<FilledSegment> {
        match overlay_rule {
            OverlayRule::Subject => self.sorted_links_with_strategy::<SubjectFilter>(fill_source),
            OverlayRule::Clip => self.sorted_links_with_strategy::<ClipFilter>(fill_source),
            OverlayRule::Intersect => {
                self.sorted_links_with_strategy::<IntersectFilter>(fill_source)
            }
            OverlayRule::Union => self.sorted_links_with_strategy::<UnionFilter>(fill_source),
            OverlayRule::Difference => {
                self.sorted_links_with_strategy::<DifferenceFilter>(fill_source)
            }
            OverlayRule::Xor => self.sorted_links_with_strategy::<XorFilter>(fill_source),
            OverlayRule::InverseDifference => {
                self.sorted_links_with_strategy::<InverseDifferenceFilter>(fill_source)
            }
        }
    }

    fn sorted_links_with_strategy<F: InclusionFilterStrategy>(
        &self,
        fill_source: FillSource,
    ) -> Vec<FilledSegment> {
        let mut segments = Vec::with_capacity(self.source.count());

        for (vr, &fill) in self.source.vr_list.iter().zip(fill_source.vr.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let x = vr.pos;
            let a = IntPoint::new(x, vr.range.min);
            let b = IntPoint::new(x, vr.range.max);

            segments.push(FilledSegment::new(a, b, fill));
        }

        for (hz, &fill) in self.source.hz_list.iter().zip(fill_source.hz.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let y = hz.pos;
            let a = IntPoint::new(hz.range.min, y);
            let b = IntPoint::new(hz.range.max, y);

            segments.push(FilledSegment::new(a, b, fill));
        }

        for (dp, &fill) in self.source.dp_list.iter().zip(fill_source.dp.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let ay = dp.pos;
            let by = PositiveDiagonal::new(dp.range, dp.pos).find_y(dp.range.max);
            let a = IntPoint::new(dp.range.min, ay);
            let b = IntPoint::new(dp.range.max, by);

            segments.push(FilledSegment::new(a, b, fill));
        }

        for (dn, &fill) in self.source.dn_list.iter().zip(fill_source.dn.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let ay = NegativeDiagonal::new(dn.range, dn.pos).find_y(dn.range.min);
            let by = dn.pos;
            let a = IntPoint::new(dn.range.min, ay);
            let b = IntPoint::new(dn.range.max, by);

            segments.push(FilledSegment::new(a, b, fill));
        }

        segments.sort_by_two_keys(false, |s|s.a.x, |s|s.a.y);

        segments
    }
}
