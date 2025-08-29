use crate::core::fill::InclusionFilterStrategy;
use crate::core::overlay_rule::OverlayRule;
use crate::gear::fill_source::FillSource;
use crate::gear::filter::{
    ClipFilter, DifferenceFilter, IntersectFilter, InverseDifferenceFilter, SubjectFilter,
    UnionFilter, XorFilter,
};
use crate::gear::section::Section;
use crate::geom::diagonal::{Diagonal, NegativeDiagonal, PositiveDiagonal};
use crate::geom::id_point::IdPoint;
use crate::graph::end::End;
use crate::graph::link::OverlayLink;
use alloc::vec;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::sort::layout::BinStore;
use crate::geom::range::LineRange;

pub(super) struct SubGraph {
    pub(super) links: Vec<OverlayLink>,
    pub(super) ends: Vec<End>,
}

impl Section {

    pub(super) fn sub_graph(
        &self,
        overlay_rule: OverlayRule,
        fill_source: FillSource,
        x_range: LineRange,
    ) -> SubGraph {
        match overlay_rule {
            OverlayRule::Subject => {
                self.sorted_links_with_strategy::<SubjectFilter>(fill_source, x_range)
            }
            OverlayRule::Clip => {
                self.sorted_links_with_strategy::<ClipFilter>(fill_source, x_range)
            }
            OverlayRule::Intersect => {
                self.sorted_links_with_strategy::<IntersectFilter>(fill_source, x_range)
            }
            OverlayRule::Union => {
                self.sorted_links_with_strategy::<UnionFilter>(fill_source, x_range)
            }
            OverlayRule::Difference => {
                self.sorted_links_with_strategy::<DifferenceFilter>(fill_source, x_range)
            }
            OverlayRule::Xor => {
                self.sorted_links_with_strategy::<XorFilter>(fill_source, x_range)
            }
            OverlayRule::InverseDifference => self
                .sorted_links_with_strategy::<InverseDifferenceFilter>(fill_source, x_range),
        }
    }

    fn sorted_links_with_strategy<F: InclusionFilterStrategy>(
        &self,
        fill_source: FillSource,
        x_range: LineRange,
    ) -> SubGraph {

        let mut bin_store = BinStore::new_anyway(x_range.min, x_range.max, self.source.count());

        // calculate bin capacity
        for (vr, &fill) in self.source.vr_list.iter().zip(fill_source.vr.iter()) {
            if F::is_included(fill) {
                bin_store.reserve_bins_for_key(vr.pos);
            }
        }

        for (hz, &fill) in self.source.hz_list.iter().zip(fill_source.hz.iter()) {
            if F::is_included(fill) {
                bin_store.reserve_bins_for_key(hz.range.min);
            }
        }

        for (dp, &fill) in self.source.dp_list.iter().zip(fill_source.dp.iter()) {
            if F::is_included(fill) {
                bin_store.reserve_bins_for_key(dp.range.min);
            }
        }

        for (dn, &fill) in self.source.dn_list.iter().zip(fill_source.dn.iter()) {
            if F::is_included(fill) {
                bin_store.reserve_bins_for_key(dn.range.min);
            }
        }


        let links_count = bin_store.prepare_bins();

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;

        let mut links = vec![OverlayLink::default(); links_count];

        for (vr, &fill) in self.source.vr_list.iter().zip(fill_source.vr.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let x = vr.pos;
            let a = IdPoint::new(0, IntPoint::new(x, vr.range.min));
            let b = IdPoint::new(0, IntPoint::new(x, vr.range.max));

            min_x = min_x.min(b.point.x);
            max_x = max_x.max(b.point.x);

            let link = OverlayLink::new(a, b, fill);

            bin_store.feed_by_key(&mut links, link, x);
        }

        for (hz, &fill) in self.source.hz_list.iter().zip(fill_source.hz.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let y = hz.pos;
            let a = IdPoint::new(0, IntPoint::new(hz.range.min, y));
            let b = IdPoint::new(0, IntPoint::new(hz.range.max, y));

            min_x = min_x.min(b.point.x);
            max_x = max_x.max(b.point.x);

            let link = OverlayLink::new(a, b, fill);

            bin_store.feed_by_key(&mut links, link, hz.range.min);
        }

        for (dp, &fill) in self.source.dp_list.iter().zip(fill_source.dp.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let ay = dp.pos;
            let by = PositiveDiagonal::new(dp.range, dp.pos).find_y(dp.range.max);
            let a = IdPoint::new(0, IntPoint::new(dp.range.min, ay));
            let b = IdPoint::new(0, IntPoint::new(dp.range.max, by));

            min_x = min_x.min(b.point.x);
            max_x = max_x.max(b.point.x);

            let link = OverlayLink::new(a, b, fill);

            bin_store.feed_by_key(&mut links, link, dp.range.min);
        }

        for (dn, &fill) in self.source.dn_list.iter().zip(fill_source.dn.iter()) {
            if !F::is_included(fill) {
                continue;
            }

            let ay = NegativeDiagonal::new(dn.range, dn.pos).find_y(dn.range.min);
            let by = dn.pos;
            let a = IdPoint::new(0, IntPoint::new(dn.range.min, ay));
            let b = IdPoint::new(0, IntPoint::new(dn.range.max, by));

            min_x = min_x.min(b.point.x);
            max_x = max_x.max(b.point.x);

            let link = OverlayLink::new(a, b, fill);

            bin_store.feed_by_key(&mut links, link, dn.range.min);
        }

        bin_store.sort_by_bins(&mut links, |l0, l1| {
            l0.a.point
                .cmp(&l1.a.point)
                .then(l0.b.point.cmp(&l1.b.point))
        });

        // ends

        bin_store.resize(min_x, max_x, links.len());

        for link in links.iter() {
            bin_store.reserve_bins_for_key(link.b.point.x);
        }
        bin_store.prepare_bins();

        let mut ends = vec![End::default(); links.len()];

        for (index, link) in links.iter().enumerate() {
            let end = End { index, point: link.b.point };
            bin_store.feed_by_key(&mut ends, end, link.b.point.x );
        }

        bin_store.sort_by_bins(&mut ends, |e0, e1| e0.point.cmp(&e1.point));

        SubGraph { links, ends }
    }
}

