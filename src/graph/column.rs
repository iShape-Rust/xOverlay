use crate::graph::end::End;
use crate::graph::link::OverlayLink;
use crate::ortho::column::Column;

impl<C> Column<C> {

    pub(crate) fn prepare_links_and_ends(
        &self,
        links: &mut [OverlayLink],
        ends: &mut [End],
    ) {
        debug_assert_eq!(links.len(), ends.len(), "links/ends window size must match");
        links.sort_unstable_by(|link_0, link_1| {
            link_0
                .a
                .point
                .cmp(&link_1.a.point)
                .then(link_0.b.point.cmp(&link_1.b.point))
        });

        for ((index, link), end) in links.iter().enumerate().zip(ends.iter_mut()) {
            *end = End { index: self.links_start + index, point: link.b.point };
        }
        ends.sort_unstable_by(|end_0, end_1| end_0.point.cmp(&end_1.point));
    }
}