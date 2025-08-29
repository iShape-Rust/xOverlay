pub(crate) mod end;
pub(crate) mod link;
mod node;
mod build;
mod extract;
mod nearest_vector;

use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use crate::core::options::IntOverlayOptions;
use crate::graph::end::End;
use crate::graph::link::OverlayLink;
use crate::graph::node::OverlayNode;

#[derive(Default)]
pub(crate) struct BooleanExtractionBuffer {
    pub(crate) points: Vec<IntPoint>,
    pub(crate) visited: Vec<bool>
}

pub struct OverlayGraph {
    pub(crate) options: IntOverlayOptions,
    pub(crate) nodes: Vec<OverlayNode>,
    pub(crate) links: Vec<OverlayLink>,
    pub(crate) ends: Vec<End>,
    pub(crate) buffer: Option<BooleanExtractionBuffer>,
}