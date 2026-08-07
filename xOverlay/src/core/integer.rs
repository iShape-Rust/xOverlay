//! Integer types supported by the orthogonal overlay engine.

use i_float::int::number::int::IntNumber;
use i_key_sort::sort::key::SortKey;

mod private {
    use super::{IntNumber, SortKey};

    pub trait OverlayIntSealed: IntNumber + SortKey {}

    impl OverlayIntSealed for i16 {}
    impl OverlayIntSealed for i32 {}
    impl OverlayIntSealed for i64 {}
}

/// An integer coordinate type supported by [`crate::core::overlay::Overlay`].
///
/// The supported types match `iOverlay`: [`i16`], [`i32`], and [`i64`].
pub trait OverlayInt: private::OverlayIntSealed {}

impl OverlayInt for i16 {}
impl OverlayInt for i32 {}
impl OverlayInt for i64 {}
