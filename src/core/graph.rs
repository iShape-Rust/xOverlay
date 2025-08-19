use alloc::vec::Vec;
use crate::build::builder::GraphNode;
use crate::core::link::OverlayLink;

pub struct BooleanGraph {
    pub(crate) nodes: Vec<OverlayNode>,
    pub(crate) links: Vec<OverlayLink>,
}

#[derive(Debug)]
pub(crate) enum OverlayNode {
    Bridge([usize; 2]),
    Cross(Vec<usize>),
}

impl GraphNode for OverlayNode {
    #[inline]
    fn with_indices(indices: &[usize]) -> Self {
        if indices.len() == 2 {
            Self::Bridge(unsafe { [*indices.get_unchecked(0), *indices.get_unchecked(1)] })
        } else {
            Self::Cross(indices.to_vec())
        }
    }
}

impl BooleanGraph {
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

impl Default for BooleanGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            links: Vec::new(),
        }
    }
}