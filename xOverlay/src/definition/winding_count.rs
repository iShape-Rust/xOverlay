use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;
use core::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ShapeCountBoolean<W: WindingCount = i16> {
    pub subj: W,
    pub clip: W,
}

impl<W: WindingCount> ShapeCountBoolean<W> {
    const SUBJ_DIRECT: Self = Self {
        subj: W::ONE,
        clip: W::ZERO,
    };
    const SUBJ_INVERT: Self = Self {
        subj: W::NEG_ONE,
        clip: W::ZERO,
    };
    const CLIP_DIRECT: Self = Self {
        subj: W::ZERO,
        clip: W::ONE,
    };
    const CLIP_INVERT: Self = Self {
        subj: W::ZERO,
        clip: W::NEG_ONE,
    };

    #[inline(always)]
    pub(crate) fn is_empty(&self) -> bool {
        self.subj == W::ZERO && self.clip == W::ZERO
    }

    #[inline(always)]
    pub(crate) fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }

    #[inline(always)]
    pub(crate) fn empty() -> Self {
        Self::new(W::ZERO, W::ZERO)
    }

    #[inline(always)]
    pub(crate) fn new(subj: W, clip: W) -> Self {
        Self { subj, clip }
    }

    #[cfg(test)]
    #[inline(always)]
    pub(crate) fn subj(subj: W) -> Self {
        Self {
            subj,
            clip: W::ZERO,
        }
    }

    #[inline(always)]
    pub(crate) fn with_shape_type(shape_type: ShapeType) -> (Self, Self) {
        match shape_type {
            ShapeType::Subject => (Self::SUBJ_DIRECT, Self::SUBJ_INVERT),
            ShapeType::Clip => (Self::CLIP_DIRECT, Self::CLIP_INVERT),
        }
    }

    #[inline(always)]
    pub(crate) fn add(self, count: Self) -> Self {
        Self::new(self.subj + count.subj, self.clip + count.clip)
    }

    #[inline(always)]
    pub(crate) fn sub(self, count: Self) -> Self {
        Self::new(self.subj - count.subj, self.clip - count.clip)
    }
}

impl<W: WindingCount> ops::Add for ShapeCountBoolean<W> {
    type Output = Self;

    #[inline(always)]
    fn add(self, other: Self) -> Self {
        Self::new(self.subj + other.subj, self.clip + other.clip)
    }
}

impl<W: WindingCount> ops::Sub for ShapeCountBoolean<W> {
    type Output = Self;

    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        Self::new(self.subj - other.subj, self.clip - other.clip)
    }
}

#[cfg(test)]
mod tests {
    use super::ShapeCountBoolean;

    #[test]
    fn i32_winding_count_can_exceed_i16_range() {
        let count =
            ShapeCountBoolean::<i32>::new(i16::MAX as i32, 0).add(ShapeCountBoolean::new(1, 0));

        assert_eq!(count.subj, i16::MAX as i32 + 1);
    }
}
