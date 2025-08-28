// use crate::core::fill::{FillStrategy, SegmentFill};
// use crate::core::winding::WindingCount;
// use crate::graph::boolean::winding_count::ShapeCountBoolean;
//
// struct EvenOddStrategy;
// struct NonZeroStrategy;
// struct PositiveStrategy;
// struct NegativeStrategy;
//
// impl FillStrategy<ShapeCountBoolean> for EvenOddStrategy {
//     #[inline(always)]
//     fn add_and_fill(
//         this: ShapeCountBoolean,
//         bot: ShapeCountBoolean,
//     ) -> (ShapeCountBoolean, SegmentFill) {
//         let top = bot.add(this);
//         let subj_top = 1 & top.subj as SegmentFill;
//         let subj_bot = 1 & bot.subj as SegmentFill;
//         let clip_top = 1 & top.clip as SegmentFill;
//         let clip_bot = 1 & bot.clip as SegmentFill;
//
//         let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);
//
//         (top, fill)
//     }
// }
//
// impl FillStrategy<ShapeCountBoolean> for NonZeroStrategy {
//     #[inline(always)]
//     fn add_and_fill(
//         this: ShapeCountBoolean,
//         bot: ShapeCountBoolean,
//     ) -> (ShapeCountBoolean, SegmentFill) {
//         let top = bot.add(this);
//         let subj_top = (top.subj != 0) as SegmentFill;
//         let subj_bot = (bot.subj != 0) as SegmentFill;
//         let clip_top = (top.clip != 0) as SegmentFill;
//         let clip_bot = (bot.clip != 0) as SegmentFill;
//
//         let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);
//
//         (top, fill)
//     }
// }
//
// impl FillStrategy<ShapeCountBoolean> for PositiveStrategy {
//     #[inline(always)]
//     fn add_and_fill(
//         this: ShapeCountBoolean,
//         bot: ShapeCountBoolean,
//     ) -> (ShapeCountBoolean, SegmentFill) {
//         let top = bot.add(this);
//         let subj_top = (top.subj > 0) as SegmentFill;
//         let subj_bot = (bot.subj > 0) as SegmentFill;
//         let clip_top = (top.clip > 0) as SegmentFill;
//         let clip_bot = (bot.clip > 0) as SegmentFill;
//
//         let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);
//
//         (top, fill)
//     }
// }
//
// impl FillStrategy<ShapeCountBoolean> for NegativeStrategy {
//     #[inline(always)]
//     fn add_and_fill(
//         this: ShapeCountBoolean,
//         bot: ShapeCountBoolean,
//     ) -> (ShapeCountBoolean, SegmentFill) {
//         let top = bot.add(this);
//         let subj_top = (top.subj < 0) as SegmentFill;
//         let subj_bot = (bot.subj < 0) as SegmentFill;
//         let clip_top = (top.clip < 0) as SegmentFill;
//         let clip_bot = (bot.clip < 0) as SegmentFill;
//
//         let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);
//
//         (top, fill)
//     }
// }
