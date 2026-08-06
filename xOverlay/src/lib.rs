//! # xOverlay
//!
//! The `xOverlay` provides Boolean Operations for 45 degrees geometry.
#![no_std]
extern crate alloc;

pub mod core;
mod definition;
mod deg_90;
mod geom;
mod partition;
pub(crate) mod util;

pub use i_float;
pub use i_shape;
