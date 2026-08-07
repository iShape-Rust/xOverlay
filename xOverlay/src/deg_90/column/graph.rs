use crate::core::integer::OverlayInt;
use crate::deg_90::column::link::Link;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;

pub(super) struct Node<I: OverlayInt> {
    pub(super) point: IntPoint<I>,
    pub(super) links: [Link; 4],
}

pub(super) struct ColumnGraph<I: OverlayInt> {
    pub(super) nodes: Vec<Node<I>>,
}

#[cfg(test)]
mod tests {
    use crate::deg_90::column::link::LinkIndex;

    #[test]
    fn test_opposite() {
        assert_eq!(LinkIndex::Left.opposite(), LinkIndex::Right);
        assert_eq!(LinkIndex::Right.opposite(), LinkIndex::Left);
        assert_eq!(LinkIndex::Up.opposite(), LinkIndex::Down);
        assert_eq!(LinkIndex::Down.opposite(), LinkIndex::Up);
    }
}
