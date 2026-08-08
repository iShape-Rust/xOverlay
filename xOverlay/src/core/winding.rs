//! Winding-count storage used by the orthogonal overlay solver.

use core::fmt::Debug;
use core::hash::Hash;
use core::ops::{Add, BitAnd, Sub};

mod private {
    pub trait WindingCountSealed {}

    impl WindingCountSealed for i16 {}
    impl WindingCountSealed for i32 {}
    impl WindingCountSealed for i64 {}
}

/// Signed integer storage for subject and clip winding counts.
///
/// [`i16`] is the default used by [`Overlay`](crate::core::overlay::Overlay). Select [`i32`] or
/// [`i64`] when the input can contain more overlapping or nested contours than fit in `i16`.
/// Arithmetic follows the overflow behavior of the selected integer type.
pub trait WindingCount:
    private::WindingCountSealed
    + Clone
    + Copy
    + Debug
    + Default
    + Eq
    + Ord
    + Hash
    + Send
    + Sync
    + Add<Output = Self>
    + Sub<Output = Self>
    + BitAnd<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;
    const NEG_ONE: Self;

    #[inline(always)]
    fn is_odd(self) -> bool {
        self & Self::ONE != Self::ZERO
    }
}

macro_rules! impl_winding_count {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl WindingCount for $ty {
                const ZERO: Self = 0;
                const ONE: Self = 1;
                const NEG_ONE: Self = -1;
            }
        )+
    };
}

impl_winding_count!(i16, i32, i64);
