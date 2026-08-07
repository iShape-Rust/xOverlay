use i_float::int::number::int::IntNumber;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct LineRange<I: IntNumber> {
    pub(crate) min: I,
    pub(crate) max: I,
}

impl<I: IntNumber> LineRange<I> {
    #[inline(always)]
    pub(crate) fn with_min_max(min: I, max: I) -> Self {
        Self { min, max }
    }
}
