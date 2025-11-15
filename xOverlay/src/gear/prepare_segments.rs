use crate::definition::segment::SegmentFill;
use crate::core::overlay_rule::OverlayRule;
use crate::gear::fill_source::FillSource;
use crate::definition::filter::{ClipFilter, DifferenceFilter, FilterStrategy, IntersectFilter, InverseDifferenceFilter, SubjectFilter, UnionFilter, XorFilter};
use crate::gear::section::Section;
use crate::geom::diagonal::{Diagonal, NegativeDiagonal, PositiveDiagonal};
use crate::geom::x_segment::XSegment;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use crate::gear::process::SegmentsPack;

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
    pub(super) fn prepare_segments(
        &self,
        overlay_rule: OverlayRule,
        fill_src: FillSource,
    ) -> SegmentsPack {
        match overlay_rule {
            OverlayRule::Subject => self.segments::<SubjectFilter>(fill_src),
            OverlayRule::Clip => self.segments::<ClipFilter>(fill_src),
            OverlayRule::Intersect => self.segments::<IntersectFilter>(fill_src),
            OverlayRule::Union => self.segments::<UnionFilter>(fill_src),
            OverlayRule::Difference => self.segments::<DifferenceFilter>(fill_src),
            OverlayRule::Xor => self.segments::<XorFilter>(fill_src),
            OverlayRule::InverseDifference => self.segments::<InverseDifferenceFilter>(fill_src),
        }
    }

    fn segments<F: FilterStrategy>(
        &self,
        fill_source: FillSource,
    ) -> SegmentsPack {
        let mut count = 0;

        for &fill in fill_source.vr.iter() {
            if F::is_included(fill) {
                count += 1;
            }
        }

        for &fill in fill_source.hz.iter() {
            if F::is_included(fill) {
                count += 1;
            }
        }

        for &fill in fill_source.dp.iter() {
            if F::is_included(fill) {
                count += 1;
            }
        }

        for &fill in fill_source.dn.iter() {
            if F::is_included(fill) {
                count += 1;
            }
        }

        let mut segments = Vec::with_capacity(count);
        let mut fills = Vec::with_capacity(count);

        for (vr, &fill) in self.source.vr_list.iter().zip(fill_source.vr.iter()) {
            if !F::is_included(fill) {
                continue;
            }
            let min_y = vr.range.min;
            let max_y = vr.range.max;
            let x = vr.pos;

            let a = IntPoint::new(x, min_y);
            let b = IntPoint::new(x, max_y);

            segments.push(XSegment { a, b });
            fills.push(fill);
        }

        for (hz, &fill) in self.source.hz_list.iter().zip(fill_source.hz.iter()) {
            if !F::is_included(fill) {
                continue;
            }
            let min_x = hz.range.min;
            let max_x = hz.range.max;
            let y = hz.pos;

            let a = IntPoint::new(min_x, y);
            let b = IntPoint::new(max_x, y);

            segments.push(XSegment { a, b });
            fills.push(fill);
        }

        for (dp, &fill) in self.source.dp_list.iter().zip(fill_source.dp.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let min_x = dp.range.min;
            let max_x = dp.range.max;

            let min_y = dp.pos;
            let max_y = PositiveDiagonal::new(dp.range, dp.pos).find_y(max_x);

            let a = IntPoint::new(min_x, min_y);
            let b = IntPoint::new(max_x, max_y);

            segments.push(XSegment { a, b });
            fills.push(fill);
        }

        for (dn, &fill) in self.source.dn_list.iter().zip(fill_source.dn.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let min_x = dn.range.min;
            let max_x = dn.range.max;

            let min_y = dn.pos;
            let max_y = NegativeDiagonal::new(dn.range, dn.pos).find_y(min_x);

            let a = IntPoint::new(min_x, max_y);
            let b = IntPoint::new(max_x, min_y);

            segments.push(XSegment { a, b });
            fills.push(fill);
        }

        SegmentsPack {
            segments,
            fills,
        }
    }
}
