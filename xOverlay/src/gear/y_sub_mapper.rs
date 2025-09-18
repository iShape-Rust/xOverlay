use crate::gear::y_layout::YLayout;
use alloc::vec;
use alloc::vec::Vec;

pub(super) struct YSubMapper {
    pub(super) parts_layout: YLayout,
    pub(super) parts_count: Vec<usize>,
    pub(super) parts_start: Vec<usize>,
}

impl YSubMapper {
    #[inline]
    pub(super) fn new(layout: YLayout) -> Self {
        let n = layout.count();
        Self {
            parts_layout: layout,
            parts_count: vec![0; n],
            parts_start: vec![0; n],
        }
    }
}