//! # xOverlay
//!
//! `xOverlay` provides high-performance Boolean operations for orthogonal (Manhattan) polygons.
#![no_std]
extern crate alloc;

pub mod core;
mod definition;
mod deg_90;
mod geom;
mod partition;
#[cfg(test)]
mod test_utils;
pub(crate) mod util;

pub use i_float;
pub use i_shape;
