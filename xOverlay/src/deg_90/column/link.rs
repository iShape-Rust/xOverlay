use crate::definition::segment::SegmentFill;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
#[derive(PartialEq)]
pub(super) enum LinkIndex {
    Left = 0,
    Up = 1,
    Right = 2,
    Down = 3,
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy)]
pub(super) struct Link {
    index: u32,
    fill: SegmentFill,
}

impl LinkIndex {
    #[inline(always)]
    pub(super) fn opposite(self) -> Self {
        // safe because `self as u8 ^ 2` is still 0..=3
        unsafe { core::mem::transmute::<u8, LinkIndex>(self as u8 ^ 0b10) }
    }

    #[inline(always)]
    pub(super) fn with_order(order: usize) -> Self {
        debug_assert!(order < 4);
        unsafe { core::mem::transmute::<u8, LinkIndex>(order as u8) }
    }

    #[inline(always)]
    pub(super) fn opposite_order(order: usize) -> usize {
        debug_assert!(order < 4);
        order ^ 0b10
    }

    #[inline(always)]
    pub(super) fn order(&self) -> usize {
        *self as usize
    }
}

#[cfg(not(debug_assertions))]
#[derive(Debug, Clone, Copy)]
pub(super) struct Link {
    data: u32,
}

#[cfg(debug_assertions)]
impl Link {
    #[inline(always)]
    pub(super) fn index(&self) -> usize {
        self.index as usize
    }

    #[inline(always)]
    pub(super) fn fill(&self) -> SegmentFill {
        self.fill
    }

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

#[cfg(not(debug_assertions))]
impl Link {
    const INDEX_BITS: u32 = 28;
    const INDEX_MASK: u32 = (1 << Self::INDEX_BITS) - 1;

    #[inline(always)]
    pub(super) fn index(&self) -> usize {
        (self.data & Self::INDEX_MASK) as usize
    }

    #[inline(always)]
    pub(super) fn fill(&self) -> SegmentFill {
        (self.data >> Self::INDEX_BITS) as SegmentFill
    }

    #[inline(always)]
    pub(super) fn is_empty(&self) -> bool {
        self.data == 0
    }

    #[inline(always)]
    pub(super) fn is_not_empty(&self) -> bool {
        self.data != 0
    }

    #[inline(always)]
    pub(super) fn empty() -> Self {
        Self { data: 0 }
    }
    #[inline(always)]
    pub(super) fn new(node: u32, fill: SegmentFill) -> Self {
        debug_assert!(node <= Self::INDEX_MASK);
        Self {
            data: node | ((fill as u32) << Self::INDEX_BITS),
        }
    }
}
