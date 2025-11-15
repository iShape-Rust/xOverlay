#[derive(Debug, Clone, Copy)]
pub struct ColumnConfig90 {
    // minimal number of columns we allow to produce (for tests or very small inputs)
    // invariant: >= 1
    pub(super) min_columns_count: usize,

    // minimal column_graph width in power-of-two form
    // production: 7..8 (width 128..256)
    // this is a *hard* lower bound for X-size of a column_graph
    pub(super) min_column_width_power: usize,

    // how many segments we *want* to have per column_graph, on average, max limit
    // used to *propose* number of columns from total segments
    // production: ~ 100_000
    pub(super) max_allow_segments_per_column: usize,

    // how many segments we *want* to have per column_graph, on average, min limit
    // used to *propose* number of columns from total segments
    // production: ~ 10_000
    pub(super) min_allowed_segments_per_column: usize,

    pub(super) max_allowed_segments_per_line: usize
}

impl ColumnConfig90 {
    pub(super) fn dev(min_columns_count: usize) -> Self {
        Self {
            min_columns_count,
            min_column_width_power: 1,
            min_allowed_segments_per_column: 1_000_000,
            max_allow_segments_per_column: 1000_000_000,
            max_allowed_segments_per_line: 128,
        }
    }

    pub(super) fn new(
        avg_min_segments_in_column: usize,
        avg_max_segments_in_column: usize,
    ) -> Self {
        Self {
            min_columns_count: 1,
            min_column_width_power: 8,
            min_allowed_segments_per_column: avg_min_segments_in_column,
            max_allow_segments_per_column: avg_max_segments_in_column,
            max_allowed_segments_per_line: 128,
        }
    }
}

impl Default for ColumnConfig90 {
    fn default() -> Self {
        Self {
            min_columns_count: 1,
            min_column_width_power: 8,
            max_allow_segments_per_column: 2 ^ 20,
            min_allowed_segments_per_column: 2 ^ 16,
            max_allowed_segments_per_line: 7,
        }
    }
}
