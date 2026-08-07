//! Validation for contour representations accepted by the orthogonal solver.

use crate::core::integer::OverlayInt;
use alloc::vec::Vec;
use core::fmt;
use i_float::int::point::IntPoint;

/// A contour whose points can be inspected without converting or copying it.
///
/// Implement this trait for custom contour representations to reuse
/// [`Contour::validate`]. [`IntContour`](i_shape::int::shape::IntContour) implements it through
/// its underlying point slice.
pub trait Contour<I: OverlayInt> {
    /// Returns the number of stored points.
    fn len(&self) -> usize;

    /// Returns the point at `index`.
    ///
    /// The validator only calls this method with indices smaller than [`Self::len`].
    fn point(&self, index: usize) -> IntPoint<I>;

    /// Returns `true` when the contour contains no points.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Validates the structural requirements of the orthogonal solver.
    ///
    /// The closing edge from the last point to the first is validated automatically; the first
    /// point does not need to be stored again at the end. Zero-length edges and collinear vertices
    /// are reported as warnings because they are redundant but valid. Self-intersection, winding
    /// direction, nesting, and relationships between contours are intentionally not checked.
    fn validate(&self) -> Result<Vec<ContourWarning>, ContourValidationError> {
        let len = self.len();

        if len < 4 {
            return Err(ContourValidationError::TooFewVertices { count: len });
        }

        let mut warnings = Vec::new();

        for end in 0..len {
            let start = if end == 0 { len - 1 } else { end - 1 };
            let a = self.point(start);
            let b = self.point(end);

            if a == b {
                warnings.push(ContourWarning::ZeroLengthEdge { start, end });
                continue;
            }

            if a.x != b.x && a.y != b.y {
                return Err(ContourValidationError::NonOrthogonalEdge { start, end });
            }
        }

        for index in 0..len {
            let prev = self.point(if index == 0 { len - 1 } else { index - 1 });
            let point = self.point(index);
            let next = self.point(if index + 1 == len { 0 } else { index + 1 });

            if prev != point
                && point != next
                && ((prev.x == point.x && point.x == next.x)
                    || (prev.y == point.y && point.y == next.y))
            {
                warnings.push(ContourWarning::CollinearVertex { index });
            }
        }

        Ok(warnings)
    }
}

impl<I: OverlayInt> Contour<I> for [IntPoint<I>] {
    #[inline]
    fn len(&self) -> usize {
        <[IntPoint<I>]>::len(self)
    }

    #[inline]
    fn point(&self, index: usize) -> IntPoint<I> {
        self[index]
    }
}

/// A structural error that makes a contour unsuitable for the orthogonal solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourValidationError {
    /// The contour contains fewer than four vertices.
    TooFewVertices { count: usize },
    /// An edge changes both coordinates and is therefore neither horizontal nor vertical.
    NonOrthogonalEdge { start: usize, end: usize },
}

impl fmt::Display for ContourValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewVertices { count } => {
                write!(f, "a contour requires at least 4 vertices, found {count}")
            }
            Self::NonOrthogonalEdge { start, end } => {
                write!(f, "edge {start}..{end} is not orthogonal")
            }
        }
    }
}

impl core::error::Error for ContourValidationError {}

/// A non-fatal contour property that may be useful to normalize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourWarning {
    /// Two adjacent vertices describe an edge with no length.
    ZeroLengthEdge { start: usize, end: usize },
    /// The vertex lies between two collinear edges and is redundant for the solver.
    CollinearVertex { index: usize },
}

impl fmt::Display for ContourWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLengthEdge { start, end } => {
                write!(f, "edge {start}..{end} has zero length")
            }
            Self::CollinearVertex { index } => {
                write!(f, "vertex {index} joins two collinear edges")
            }
        }
    }
}
