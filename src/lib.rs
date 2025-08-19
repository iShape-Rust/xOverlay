//! # xOverlay
//!
//! The `xOverlay` provides Boolean Operations for 45 degrees geometry.

#![cfg_attr(not(test), no_std)]
extern crate alloc;

pub mod core;
pub(crate) mod build;
mod ortho;
mod sub;
mod geom;

pub use i_float;
pub use i_shape;