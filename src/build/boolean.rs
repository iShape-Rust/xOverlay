use crate::core::fill::{
    ALL, BOTH_BOTTOM, BOTH_TOP, CLIP_BOTH, CLIP_BOTTOM, CLIP_TOP, FillStrategy,
    InclusionFilterStrategy, SUBJ_BOTH, SUBJ_BOTTOM, SUBJ_TOP, SegmentFill,
};
use crate::core::fill_rule::FillRule;
use crate::core::link::{OverlayLink, OverlayLinkFilter};
use crate::core::overlay_rule::OverlayRule;
use crate::core::shape_type::ShapeType;
use crate::core::winding::WindingCount;
use crate::ortho::column::Column;
use alloc::vec::Vec;
use i_shape::util::reserve::Reserve;

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

impl Column<ShapeCountBoolean> {
    pub(crate) fn fill_boolean(&mut self, fill_rule: FillRule) {
        match fill_rule {
            FillRule::EvenOdd => self.fill_with_strategy::<EvenOddStrategy>(),
            FillRule::NonZero => self.fill_with_strategy::<NonZeroStrategy>(),
            FillRule::Positive => self.fill_with_strategy::<PositiveStrategy>(),
            FillRule::Negative => self.fill_with_strategy::<NegativeStrategy>(),
        }
    }

    pub(crate) fn count_included_links_with_overlay_rule(
        &self,
        overlay_rule: OverlayRule,
    ) -> usize {
        match overlay_rule {
            OverlayRule::Subject => self.count_included_links::<SubjectFilter>(),
            OverlayRule::Clip => self.count_included_links::<ClipFilter>(),
            OverlayRule::Intersect => self.count_included_links::<IntersectFilter>(),
            OverlayRule::Union => self.count_included_links::<UnionFilter>(),
            OverlayRule::Difference => self.count_included_links::<DifferenceFilter>(),
            OverlayRule::Xor => self.count_included_links::<XorFilter>(),
            OverlayRule::InverseDifference => {
                self.count_included_links::<InverseDifferenceFilter>()
            }
        }
    }

    pub(crate) fn copy_and_sort_links_into(
        &self,
        overlay_rule: OverlayRule,
        target: &mut [OverlayLink],
    ) {
        match overlay_rule {
            OverlayRule::Subject => self.copy_links_into::<SubjectFilter>(target),
            OverlayRule::Clip => self.copy_links_into::<ClipFilter>(target),
            OverlayRule::Intersect => self.copy_links_into::<IntersectFilter>(target),
            OverlayRule::Union => self.copy_links_into::<UnionFilter>(target),
            OverlayRule::Difference => self.copy_links_into::<DifferenceFilter>(target),
            OverlayRule::Xor => self.copy_links_into::<XorFilter>(target),
            OverlayRule::InverseDifference => {
                self.copy_links_into::<InverseDifferenceFilter>(target)
            }
        }
        target.sort_unstable_by(|link_0, link_1| {
            link_0
                .a
                .point
                .cmp(&link_1.a.point)
                .then(link_0.b.point.cmp(&link_1.b.point))
        });
    }
}

struct EvenOddStrategy;
struct NonZeroStrategy;
struct PositiveStrategy;
struct NegativeStrategy;

impl FillStrategy<ShapeCountBoolean> for EvenOddStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = 1 & top.subj as SegmentFill;
        let subj_bot = 1 & bot.subj as SegmentFill;
        let clip_top = 1 & top.clip as SegmentFill;
        let clip_bot = 1 & bot.clip as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}

impl FillStrategy<ShapeCountBoolean> for NonZeroStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = (top.subj != 0) as SegmentFill;
        let subj_bot = (bot.subj != 0) as SegmentFill;
        let clip_top = (top.clip != 0) as SegmentFill;
        let clip_bot = (bot.clip != 0) as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}

impl FillStrategy<ShapeCountBoolean> for PositiveStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = (top.subj > 0) as SegmentFill;
        let subj_bot = (bot.subj > 0) as SegmentFill;
        let clip_top = (top.clip > 0) as SegmentFill;
        let clip_bot = (bot.clip > 0) as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}

impl FillStrategy<ShapeCountBoolean> for NegativeStrategy {
    #[inline(always)]
    fn add_and_fill(
        this: ShapeCountBoolean,
        bot: ShapeCountBoolean,
    ) -> (ShapeCountBoolean, SegmentFill) {
        let top = bot.add(this);
        let subj_top = (top.subj < 0) as SegmentFill;
        let subj_bot = (bot.subj < 0) as SegmentFill;
        let clip_top = (top.clip < 0) as SegmentFill;
        let clip_bot = (bot.clip < 0) as SegmentFill;

        let fill = subj_top | (subj_bot << 1) | (clip_top << 2) | (clip_bot << 3);

        (top, fill)
    }
}

struct SubjectFilter;
struct ClipFilter;
struct IntersectFilter;
struct UnionFilter;
struct DifferenceFilter;
struct InverseDifferenceFilter;
struct XorFilter;

impl InclusionFilterStrategy for SubjectFilter {
    #[inline(always)]
    fn is_included(fill: SegmentFill) -> bool {
        fill.is_subject()
    }
}

impl InclusionFilterStrategy for ClipFilter {
    #[inline(always)]
    fn is_included(fill: SegmentFill) -> bool {
        fill.is_clip()
    }
}

impl InclusionFilterStrategy for IntersectFilter {
    #[inline(always)]
    fn is_included(fill: SegmentFill) -> bool {
        fill.is_intersect()
    }
}

impl InclusionFilterStrategy for UnionFilter {
    #[inline(always)]
    fn is_included(fill: SegmentFill) -> bool {
        fill.is_union()
    }
}

impl InclusionFilterStrategy for DifferenceFilter {
    #[inline(always)]
    fn is_included(fill: SegmentFill) -> bool {
        fill.is_difference()
    }
}

impl InclusionFilterStrategy for InverseDifferenceFilter {
    #[inline(always)]
    fn is_included(fill: SegmentFill) -> bool {
        fill.is_inverse_difference()
    }
}

impl InclusionFilterStrategy for XorFilter {
    #[inline(always)]
    fn is_included(fill: SegmentFill) -> bool {
        fill.is_xor()
    }
}

trait BooleanFillFilter {
    fn is_subject(&self) -> bool;
    fn is_clip(&self) -> bool;
    fn is_intersect(&self) -> bool;
    fn is_union(&self) -> bool;
    fn is_difference(&self) -> bool;
    fn is_inverse_difference(&self) -> bool;
    fn is_xor(&self) -> bool;
}

impl BooleanFillFilter for SegmentFill {
    #[inline(always)]
    fn is_subject(&self) -> bool {
        let fill = *self;
        let subj = fill & SUBJ_BOTH;
        subj == SUBJ_TOP || subj == SUBJ_BOTTOM
    }

    #[inline(always)]
    fn is_clip(&self) -> bool {
        let fill = *self;
        let clip = fill & CLIP_BOTH;
        clip == CLIP_TOP || clip == CLIP_BOTTOM
    }

    #[inline(always)]
    fn is_intersect(&self) -> bool {
        let fill = *self;
        let top = fill & BOTH_TOP;
        let bottom = fill & BOTH_BOTTOM;

        (top == BOTH_TOP || bottom == BOTH_BOTTOM) && fill != ALL
    }

    #[inline(always)]
    fn is_union(&self) -> bool {
        let fill = *self;
        let top = fill & BOTH_TOP;
        let bottom = fill & BOTH_BOTTOM;
        (top == 0 || bottom == 0) && fill != 0
    }

    #[inline(always)]
    fn is_difference(&self) -> bool {
        let fill = *self;
        let top = fill & BOTH_TOP;
        let bottom = fill & BOTH_BOTTOM;
        let is_not_inner_subj = fill != SUBJ_BOTH;
        (top == SUBJ_TOP || bottom == SUBJ_BOTTOM) && is_not_inner_subj
    }

    #[inline(always)]
    fn is_inverse_difference(&self) -> bool {
        let fill = *self;
        let top = fill & BOTH_TOP;
        let bottom = fill & BOTH_BOTTOM;
        let is_not_inner_clip = fill != CLIP_BOTH;
        (top == CLIP_TOP || bottom == CLIP_BOTTOM) && is_not_inner_clip
    }

    #[inline(always)]
    fn is_xor(&self) -> bool {
        let fill = *self;
        let top = fill & BOTH_TOP;
        let bottom = fill & BOTH_BOTTOM;

        let is_any_top = top == SUBJ_TOP || top == CLIP_TOP;
        let is_any_bottom = bottom == SUBJ_BOTTOM || bottom == CLIP_BOTTOM;

        // only one of it must be true
        is_any_top != is_any_bottom
    }
}

impl OverlayLinkFilter for [OverlayLink] {
    #[inline]
    fn filter_by_overlay(&self, overlay_rule: OverlayRule) -> Vec<bool> {
        match overlay_rule {
            OverlayRule::Subject => filter_subject(self),
            OverlayRule::Clip => filter_clip(self),
            OverlayRule::Intersect => filter_intersect(self),
            OverlayRule::Union => filter_union(self),
            OverlayRule::Difference => filter_difference(self),
            OverlayRule::Xor => filter_xor(self),
            OverlayRule::InverseDifference => filter_inverse_difference(self),
        }
    }

    #[inline]
    fn filter_by_overlay_into(&self, overlay_rule: OverlayRule, buffer: &mut Vec<bool>) {
        match overlay_rule {
            OverlayRule::Subject => filter_subject_into(self, buffer),
            OverlayRule::Clip => filter_clip_into(self, buffer),
            OverlayRule::Intersect => filter_intersect_into(self, buffer),
            OverlayRule::Union => filter_union_into(self, buffer),
            OverlayRule::Difference => filter_difference_into(self, buffer),
            OverlayRule::Xor => filter_xor_into(self, buffer),
            OverlayRule::InverseDifference => filter_inverse_difference_into(self, buffer),
        }
    }
}

#[inline]
fn filter_subject(links: &[OverlayLink]) -> Vec<bool> {
    links.iter().map(|link| !link.fill.is_subject()).collect()
}

#[inline]
fn filter_clip(links: &[OverlayLink]) -> Vec<bool> {
    links.iter().map(|link| !link.fill.is_clip()).collect()
}

#[inline]
fn filter_intersect(links: &[OverlayLink]) -> Vec<bool> {
    links.iter().map(|link| !link.fill.is_intersect()).collect()
}

#[inline]
fn filter_union(links: &[OverlayLink]) -> Vec<bool> {
    links.iter().map(|link| !link.fill.is_union()).collect()
}

#[inline]
fn filter_difference(links: &[OverlayLink]) -> Vec<bool> {
    links
        .iter()
        .map(|link| !link.fill.is_difference())
        .collect()
}

#[inline]
fn filter_inverse_difference(links: &[OverlayLink]) -> Vec<bool> {
    links
        .iter()
        .map(|link| !link.fill.is_inverse_difference())
        .collect()
}

#[inline]
fn filter_xor(links: &[OverlayLink]) -> Vec<bool> {
    links.iter().map(|link| !link.fill.is_xor()).collect()
}

#[inline]
fn filter_subject_into(links: &[OverlayLink], buffer: &mut Vec<bool>) {
    buffer.clear();
    buffer.reserve_capacity(links.len());
    for link in links.iter() {
        buffer.push(!link.fill.is_subject());
    }
}

#[inline]
fn filter_clip_into(links: &[OverlayLink], buffer: &mut Vec<bool>) {
    buffer.clear();
    buffer.reserve_capacity(links.len());
    for link in links.iter() {
        buffer.push(!link.fill.is_clip());
    }
}

#[inline]
fn filter_intersect_into(links: &[OverlayLink], buffer: &mut Vec<bool>) {
    buffer.clear();
    buffer.reserve_capacity(links.len());
    for link in links.iter() {
        buffer.push(!link.fill.is_intersect());
    }
}

#[inline]
fn filter_union_into(links: &[OverlayLink], buffer: &mut Vec<bool>) {
    buffer.clear();
    buffer.reserve_capacity(links.len());
    for link in links.iter() {
        buffer.push(!link.fill.is_union());
    }
}

#[inline]
fn filter_difference_into(links: &[OverlayLink], buffer: &mut Vec<bool>) {
    buffer.clear();
    buffer.reserve_capacity(links.len());
    for link in links.iter() {
        buffer.push(!link.fill.is_difference());
    }
}

#[inline]
fn filter_inverse_difference_into(links: &[OverlayLink], buffer: &mut Vec<bool>) {
    buffer.clear();
    buffer.reserve_capacity(links.len());
    for link in links.iter() {
        buffer.push(!link.fill.is_inverse_difference());
    }
}

#[inline]
fn filter_xor_into(links: &[OverlayLink], buffer: &mut Vec<bool>) {
    buffer.clear();
    buffer.reserve_capacity(links.len());
    for link in links.iter() {
        buffer.push(!link.fill.is_xor());
    }
}
