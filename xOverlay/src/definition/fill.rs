use crate::definition::segment::SegmentFill;
use crate::definition::winding_count::ShapeCountBoolean;

pub(crate) trait FillStrategy<C> {
    fn add_and_fill(this: C, bot: C) -> (C, SegmentFill);
    fn fill(top: C, bot: C) -> SegmentFill;
}

pub(crate) struct EvenOddStrategy;
pub(crate) struct NonZeroStrategy;
pub(crate) struct PositiveStrategy;
pub(crate) struct NegativeStrategy;

impl FillStrategy<ShapeCountBoolean> for EvenOddStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot + this;
        let fill = Self::fill(top, bot);

        (top, fill)
    }

    #[inline(always)]
    fn fill(top: ShapeCountBoolean, bot: ShapeCountBoolean) -> SegmentFill {
        let subj_top = 1 & top.subj as SegmentFill;
        let subj_bot = 1 & bot.subj as SegmentFill;
        let clip_top = 1 & top.clip as SegmentFill;
        let clip_bot = 1 & bot.clip as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}

impl FillStrategy<ShapeCountBoolean> for NonZeroStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot + this;
        let fill = Self::fill(top, bot);

        (top, fill)
    }

    #[inline(always)]
    fn fill(top: ShapeCountBoolean, bot: ShapeCountBoolean) -> SegmentFill {
        let subj_top = (top.subj != 0) as SegmentFill;
        let subj_bot = (bot.subj != 0) as SegmentFill;
        let clip_top = (top.clip != 0) as SegmentFill;
        let clip_bot = (bot.clip != 0) as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}

impl FillStrategy<ShapeCountBoolean> for PositiveStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot + this;
        let fill = Self::fill(top, bot);
        (top, fill)
    }

    #[inline(always)]
    fn fill(top: ShapeCountBoolean, bot: ShapeCountBoolean) -> SegmentFill {
        let subj_top = (top.subj > 0) as SegmentFill;
        let subj_bot = (bot.subj > 0) as SegmentFill;
        let clip_top = (top.clip > 0) as SegmentFill;
        let clip_bot = (bot.clip > 0) as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}

impl FillStrategy<ShapeCountBoolean> for NegativeStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot + this;
        let fill = Self::fill(top, bot);
        (top, fill)
    }

    #[inline(always)]
    fn fill(top: ShapeCountBoolean, bot: ShapeCountBoolean) -> SegmentFill {
        let subj_top = (top.subj < 0) as SegmentFill;
        let subj_bot = (bot.subj < 0) as SegmentFill;
        let clip_top = (top.clip < 0) as SegmentFill;
        let clip_bot = (bot.clip < 0) as SegmentFill;

        subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3)
    }
}
