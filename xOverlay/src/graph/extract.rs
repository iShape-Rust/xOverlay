use crate::core::overlay_rule::OverlayRule;
use crate::graph::data::OverlayGraph;
use alloc::vec::Vec;
use i_shape::flat::buffer::FlatContoursBuffer;
use i_shape::int::shape::IntShapes;

impl OverlayGraph {
    /// Extracts shapes from the overlay graph.
    ///
    /// Graph extraction is not connected to the new solver yet.
    #[inline]
    pub fn extract_shapes(&mut self, _overlay_rule: OverlayRule) -> IntShapes<i32> {
        Vec::new()
    }

    /// Writes flat contours extracted from the overlay graph.
    ///
    /// Graph extraction is not connected to the new solver yet.
    #[inline]
    pub fn extract_contours_into(
        &mut self,
        _overlay_rule: OverlayRule,
        _output: &mut FlatContoursBuffer<i32>,
    ) {
    }
}
