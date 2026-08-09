use crate::definition::segment::SegmentFill;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LinkIndex(u8);

#[derive(Debug, Clone, Copy)]
pub(super) struct Link {
    index: u32,
    fill: SegmentFill,
}

#[allow(non_upper_case_globals)]
impl LinkIndex {
    pub(super) const Left: Self = Self(0);
    pub(super) const Up: Self = Self(1);
    pub(super) const Right: Self = Self(2);
    pub(super) const Down: Self = Self(3);

    #[inline(always)]
    pub(super) fn opposite(self) -> Self {
        Self(self.0 ^ 0b10)
    }

    #[cfg(test)]
    #[inline(always)]
    pub(super) fn opposite_order(order: usize) -> usize {
        debug_assert!(order < 4);
        order ^ 0b10
    }

    #[inline(always)]
    pub(super) fn order(&self) -> usize {
        self.0 as usize
    }
}

impl Link {
    #[inline(always)]
    pub(super) fn index(&self) -> usize {
        self.index as usize
    }

    #[inline(always)]
    pub(super) fn fill(&self) -> SegmentFill {
        self.fill
    }

    #[cfg(test)]
    #[inline(always)]
    pub(super) fn is_empty(&self) -> bool {
        self.index == 0 && self.fill == 0
    }

    #[inline(always)]
    pub(super) fn is_not_empty(&self) -> bool {
        self.index != 0 || self.fill != 0
    }
    #[inline(always)]
    pub(super) fn empty() -> Self {
        Self { index: 0, fill: 0 }
    }

    #[inline(always)]
    pub(super) fn new(node: u32, fill: SegmentFill) -> Self {
        Self { index: node, fill }
    }
}

#[cfg(test)]
mod tests {
    use super::Link;
    use core::mem::size_of;

    #[test]
    fn link_stores_a_u32_index_and_u8_fill_in_eight_bytes() {
        assert_eq!(size_of::<Link>(), 8);
    }
}
