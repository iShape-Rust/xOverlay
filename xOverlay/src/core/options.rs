pub(crate) use crate::deg_90::config::ColumnConfig90;

/// Configuration options for polygon Boolean operations using
/// [`Overlay`](crate::core::overlay::Overlay).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct IntOverlayOptions {
    /// Configuration of the orthogonal column solver.
    pub(crate) columns_config: ColumnConfig90,
}
