use crate::core::winding::WindingCount;
use crate::sub::merge::CountMergeable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct OrthoSegment<C> {
    pub(super) pos: i32,
    pub(super) min: i32,
    pub(super) max: i32,
    pub(super) count: C,
}

impl<C: Clone> OrthoSegment<C> {
    #[inline(always)]
    pub(super) fn is_inside(&self, val: i32) -> bool {
        self.min < val && val < self.max
    }

    #[inline(always)]
    pub(super) fn cut_tail(&mut self, mid: i32) -> Self {
        let tail = Self {
            pos: self.pos,
            min: mid,
            max: self.max,
            count: self.count.clone(),
        };

        self.max = mid;

        tail
    }

    #[inline(always)]
    pub(super) fn cut_head(&mut self, mid: i32) -> Self {
        let tail = Self {
            pos: self.pos,
            min: self.min,
            max: mid,
            count: self.count.clone(),
        };

        self.min = mid;

        tail
    }
}

impl<C: WindingCount> CountMergeable<C> for OrthoSegment<C> {
    #[inline(always)]
    fn is_same_geometry(&self, other: &Self) -> bool {
        self.pos == other.pos && self.min == other.min && self.max == other.max
    }

    #[inline(always)]
    fn count(&self) -> C {
        self.count
    }

    #[inline(always)]
    fn update(&mut self, count: C) {
        self.count = count;
    }
}