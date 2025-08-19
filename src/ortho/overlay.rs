use crate::core::layout::Layout;
use crate::ortho::column::Column;
use alloc::vec::Vec;
use crate::core::graph::BooleanGraph;
use crate::core::options::IntOverlayOptions;
use crate::core::solver::Solver;

/// This struct is essential for describing and uploading the geometry or shapes required to construct an `OverlayGraph`. It prepares the necessary data for boolean operations.
pub struct OrthoOverlay<C> {
    pub options: IntOverlayOptions,
    pub solver: Solver,
    pub(crate) layout: Layout,
    pub(crate) columns: Vec<Column<C>>,
    pub(crate) graph: Option<BooleanGraph>,
}

impl<C> Default for OrthoOverlay<C> {
    #[inline]
    fn default() -> Self {
        OrthoOverlay {
            options: Default::default(),
            solver: Default::default(),
            layout: Default::default(),
            columns: Vec::new(),
            graph: None,
        }
    }
}