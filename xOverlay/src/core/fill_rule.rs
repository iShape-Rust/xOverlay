//! Rules that determine which regions of a contour set are filled.

use core::fmt;

/// Represents the rule used to determine the "bind" of a shape, affecting how shapes are filled. For a visual description, see [Fill Rules](https://ishape-rust.github.io/iShape-js/overlay/filling_rules/filling_rules.html).
/// - `EvenOdd`: Only odd-numbered sub-regions are filled.
/// - `NonZero`: Only non-zero sub-regions are filled.
/// - `Positive`: Fills regions where the winding number is positive.
/// - `Negative`: Fills regions where the winding number is negative.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FillRule {
    /// Fills regions whose winding count is odd.
    EvenOdd,
    /// Fills regions whose winding count is not zero.
    #[default]
    NonZero,
    /// Fills regions whose winding count is positive.
    Positive,
    /// Fills regions whose winding count is negative.
    Negative,
}

impl fmt::Display for FillRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            FillRule::EvenOdd => "EvenOdd",
            FillRule::NonZero => "NonZero",
            FillRule::Positive => "Positive",
            FillRule::Negative => "Negative",
        };

        write!(f, "{}", text)
    }
}
