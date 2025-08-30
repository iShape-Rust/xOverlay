//! # xOverlay
//!
//! The `xOverlay` provides Boolean Operations for 45 degrees geometry.
#![no_std]
extern crate alloc;

pub mod core;
pub(crate) mod graph;
mod geom;
pub(crate) mod bind;
mod gear;
pub(crate) mod util;

pub use i_float;
pub use i_shape;