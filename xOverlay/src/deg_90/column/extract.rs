use crate::deg_90::column::graph::{ColumnGraph, Node};
use alloc::vec::Vec;
use crate::deg_90::column::link::LinkIndex;

pub(super) struct NodeVisitor {
    data: u8,
}

impl LinkIndex {
    #[inline(always)]
    fn bit(self) -> u8 {
        1 << self as u8
    }
}

impl NodeVisitor {
    #[inline(always)]
    pub(super) fn new(node: &Node) -> Self {
        let mut data = 0;
        for (order, link) in node.links.iter().enumerate() {
            let has_bit = link.is_not_empty() as u8;
            data |= has_bit << order;
        }

        Self { data }
    }
    #[inline(always)]
    pub(super) fn visit_if_not_yet(&mut self, index: usize) -> bool {
        let bit = 1 << index;
        let val = self.data & bit;
        self.data &= !val;
        val != 0
    }

    #[inline(always)]
    pub(super) fn is_visited(&self, index: usize) -> bool {
        self.data & (1 << index) == 0
    }
}

impl ColumnGraph {
    #[inline(always)]
    pub(super) fn visitors(&self) -> Vec<NodeVisitor> {
        self.nodes.iter().map(|n| NodeVisitor::new(n)).collect()
    }
}

#[cfg(test)]
mod tests {

}
