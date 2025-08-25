use alloc::vec::Vec;
use crate::core::options::IntOverlayOptions;
use crate::core::solver::Solver;
use crate::gear::x_layout::XLayout;
use crate::gear::section::Section;

/// This struct is essential for describing and uploading the geometry or shapes required to construct an `OverlayGraph`. It prepares the necessary data for boolean operations.
pub struct Overlay {
    pub options: IntOverlayOptions,
    pub solver: Solver,
    pub(crate) layout: XLayout,
    pub(crate) sections: Vec<Section>,
}
