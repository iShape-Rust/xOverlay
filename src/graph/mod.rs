pub(crate) mod end;
pub(crate) mod link;
mod node;
pub(crate) mod boolean;
pub(crate) mod column;
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
    pub(crate) buffer: Option<BooleanExtractionBuffer>
}

impl OverlayGraph {
    pub fn validate(&self) {
        for node in self.nodes.iter() {
            if let OverlayNode::Cross(indices) = node {
                debug_assert!(indices.len() > 1, "indices: {}", indices.len());
                debug_assert!(
                    self.nodes.len() <= self.links.len(),
                    "nodes is more then links"
                );
            }
        }
    }
}

impl Default for OverlayGraph {
    fn default() -> Self {
        Self {
            options: Default::default(),
            nodes: Vec::new(),
            links: Vec::new(),
            ends: Vec::new(),
            buffer: None,
        }
    }
}