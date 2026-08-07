pub use crate::deg_90::config::ColumnConfig90;

/// Configuration options for polygon Boolean operations using
/// [`Overlay`](crate::core::overlay::Overlay).
#[derive(Debug, Clone, Copy, Default)]
pub struct IntOverlayOptions {
    /// Configuration of the orthogonal column solver.
    pub columns_config: ColumnConfig90,
}
