use i_float::int::point::IntPoint;
use crate::ortho::error::OrthoError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Orientation {
    Vertical,
    Horizontal,
}

impl Orientation {
    #[inline(always)]
    pub(crate) fn new(segment: [IntPoint; 2]) -> Result<Orientation, OrthoError> {
        let x = segment[0].x == segment[1].x;
        let y = segment[0].y == segment[1].y;
        match (x, y) {
            (true, false) => Ok(Orientation::Vertical),
            (false, true) => Ok(Orientation::Horizontal),
            _ => Err(OrthoError::NotValidPath),
        }
    }

    #[inline(always)]
    pub(crate) fn invert(&self) -> Self {
        match self {
            Orientation::Vertical => Orientation::Horizontal,
            Orientation::Horizontal => Orientation::Vertical,
        }
    }
}
