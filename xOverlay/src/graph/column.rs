use crate::graph::link::OverlayLink;
use crate::ortho::column::Column;

impl<C> Column<C> {

    pub(crate) fn sort_links(
        &self,
        links: &mut [OverlayLink],
    ) {
        links.sort_unstable_by(|link_0, link_1| {
            link_0
                .a
                .point
                .cmp(&link_1.a.point)
                .then(link_0.b.point.cmp(&link_1.b.point))
        });
    }
}