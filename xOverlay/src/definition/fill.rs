use crate::core::winding::WindingCount;
use crate::definition::segment::SegmentFill;
use crate::definition::winding_count::ShapeCountBoolean;

pub(crate) trait FillStrategy<C> {
    fn fill(top: C, bot: C) -> SegmentFill;
}

pub(crate) struct EvenOddStrategy;
pub(crate) struct NonZeroStrategy;
pub(crate) struct PositiveStrategy;
pub(crate) struct NegativeStrategy;

impl<W: WindingCount> FillStrategy<ShapeCountBoolean<W>> for EvenOddStrategy {
    #[inline(always)]
    fn fill(top: ShapeCountBoolean<W>, bot: ShapeCountBoolean<W>) -> SegmentFill {
        let subj_top = top.subj.is_odd() as SegmentFill;
        let subj_bot = bot.subj.is_odd() as SegmentFill;
        let clip_top = top.clip.is_odd() as SegmentFill;
        let clip_bot = bot.clip.is_odd() as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}

impl<W: WindingCount> FillStrategy<ShapeCountBoolean<W>> for NonZeroStrategy {
    #[inline(always)]
    fn fill(top: ShapeCountBoolean<W>, bot: ShapeCountBoolean<W>) -> SegmentFill {
        let subj_top = (top.subj != W::ZERO) as SegmentFill;
        let subj_bot = (bot.subj != W::ZERO) as SegmentFill;
        let clip_top = (top.clip != W::ZERO) as SegmentFill;
        let clip_bot = (bot.clip != W::ZERO) as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}

impl<W: WindingCount> FillStrategy<ShapeCountBoolean<W>> for PositiveStrategy {
    #[inline(always)]
    fn fill(top: ShapeCountBoolean<W>, bot: ShapeCountBoolean<W>) -> SegmentFill {
        let subj_top = (top.subj > W::ZERO) as SegmentFill;
        let subj_bot = (bot.subj > W::ZERO) as SegmentFill;
        let clip_top = (top.clip > W::ZERO) as SegmentFill;
        let clip_bot = (bot.clip > W::ZERO) as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}

impl<W: WindingCount> FillStrategy<ShapeCountBoolean<W>> for NegativeStrategy {
    #[inline(always)]
    fn fill(top: ShapeCountBoolean<W>, bot: ShapeCountBoolean<W>) -> SegmentFill {
        let subj_top = (top.subj < W::ZERO) as SegmentFill;
        let subj_bot = (bot.subj < W::ZERO) as SegmentFill;
        let clip_top = (top.clip < W::ZERO) as SegmentFill;
        let clip_bot = (bot.clip < W::ZERO) as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}
