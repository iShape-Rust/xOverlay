#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct LineRange {
    pub(crate) min: i32,
    pub(crate) max: i32,
}

impl LineRange {
    #[inline(always)]
    pub(crate) fn with_min_max(min: i32, max: i32) -> Self {
        Self {
            min,
            max,
        }
    }

    #[inline(always)]
    pub(crate) fn not_contains(&self, val: i32) -> bool {
        val < self.min || self.max < val
    }

    #[inline(always)]
    pub(crate) fn strict_contains(&self, val: i32) -> bool {
        self.min < val && val < self.max
    }
}