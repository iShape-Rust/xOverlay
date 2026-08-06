#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct LineRange {
    pub(crate) min: i32,
    pub(crate) max: i32,
}

impl LineRange {
    #[inline(always)]
    pub(crate) fn with_min_max(min: i32, max: i32) -> Self {
        Self { min, max }
    }
}
