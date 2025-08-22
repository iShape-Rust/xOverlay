use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeCountBoolean {
    pub subj: i16,
    pub clip: i16,
}

impl ShapeCountBoolean {
    const SUBJ_DIRECT: ShapeCountBoolean = ShapeCountBoolean { subj: 1, clip: 0 };
    const SUBJ_INVERT: ShapeCountBoolean = ShapeCountBoolean { subj: -1, clip: 0 };
    const CLIP_DIRECT: ShapeCountBoolean = ShapeCountBoolean { subj: 0, clip: 1 };
    const CLIP_INVERT: ShapeCountBoolean = ShapeCountBoolean { subj: 0, clip: -1 };
}

impl WindingCount for ShapeCountBoolean {
    #[inline(always)]
    fn is_not_empty(&self) -> bool {
        self.subj != 0 || self.clip != 0
    }

    #[inline(always)]
    fn empty() -> Self {
        Self::new(0, 0)
    }

    #[inline(always)]
    fn new(subj: i16, clip: i16) -> Self {
        Self { subj, clip }
    }

    #[inline(always)]
    fn with_shape_type(shape_type: ShapeType) -> (Self, Self) {
        match shape_type {
            ShapeType::Subject => (
                ShapeCountBoolean::SUBJ_DIRECT,
                ShapeCountBoolean::SUBJ_INVERT,
            ),
            ShapeType::Clip => (
                ShapeCountBoolean::CLIP_DIRECT,
                ShapeCountBoolean::CLIP_INVERT,
            ),
        }
    }

    #[inline(always)]
    fn add(self, count: Self) -> Self {
        let subj = self.subj + count.subj;
        let clip = self.clip + count.clip;

        Self { subj, clip }
    }

    #[inline(always)]
    fn apply(&mut self, count: Self) {
        self.subj += count.subj;
        self.clip += count.clip;
    }

    #[inline(always)]
    fn invert(self) -> Self {
        Self {
            subj: -self.subj,
            clip: -self.clip,
        }
    }
}