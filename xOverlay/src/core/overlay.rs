use crate::core::cpu_count::CPUCount;
use crate::core::fill_rule::FillRule;
use crate::core::integer::OverlayInt;
use crate::core::options::IntOverlayOptions;
use crate::core::overlay_rule::OverlayRule;
use crate::core::winding::WindingCount;
use crate::deg_90::column_map::{Column, ColumnMap};
use alloc::vec::Vec;
use i_shape::int::shape::{IntContour, IntShapes};

/// Input geometry prepared for an orthogonal Boolean operation.
///
/// `I` is the coordinate type. `W` stores intermediate subject and clip winding counts and
/// defaults to [`i16`]. Use [`i32`] or [`i64`] when the winding depth may exceed the `i16` range.
pub struct Overlay<I: OverlayInt, W: WindingCount = i16> {
    pub(crate) options: IntOverlayOptions,
    #[cfg(feature = "allow_multithreading")]
    pub(crate) cpu_count: CPUCount,
    pub(crate) columns: Vec<Column<I, W>>,
}

impl<I: OverlayInt, W: WindingCount> Overlay<I, W> {
    /// Prepares subject and clip contours for an orthogonal Boolean operation.
    ///
    /// # Input validity
    ///
    /// This constructor assumes valid orthogonal contours and does not validate contour length,
    /// axis alignment, closing-edge alignment, or other structural invariants. Invalid input may
    /// produce invalid geometry. Use [`Contour::validate`](crate::core::validation::Contour::validate)
    /// before construction when input is not trusted.
    #[inline]
    pub fn with_contours(subj: &[IntContour<I>], clip: &[IntContour<I>]) -> Self {
        Self::with_contours_custom(subj, clip, Default::default(), CPUCount::Auto)
    }

    /// Prepares subject and clip contours with explicit CPU configuration.
    ///
    /// # Input validity
    ///
    /// This constructor has the same unchecked input contract as [`Self::with_contours`].
    #[inline]
    pub fn with_cpu_count(
        subj: &[IntContour<I>],
        clip: &[IntContour<I>],
        cpu_count: CPUCount,
    ) -> Self {
        Self::with_contours_custom(subj, clip, Default::default(), cpu_count)
    }

    #[inline]
    pub(crate) fn with_contours_custom(
        subj: &[IntContour<I>],
        clip: &[IntContour<I>],
        options: IntOverlayOptions,
        cpu_count: CPUCount,
    ) -> Self {
        let map = ColumnMap::with_subj_and_clip(subj, clip, cpu_count, options.columns_config);

        Self {
            options,
            #[cfg(feature = "allow_multithreading")]
            cpu_count,
            columns: map.columns,
        }
    }

    /// Executes one Boolean operation and returns shapes with holes grouped under their outer
    /// contours.
    ///
    /// This method consumes the prepared input geometry.
    #[inline]
    pub fn overlay(self, overlay_rule: OverlayRule, fill_rule: FillRule) -> IntShapes<I> {
        self.process_overlay(fill_rule, overlay_rule)
    }

    /// Executes the Boolean operation and returns its boundaries as a flat list of contours.
    ///
    /// Unlike [`Self::overlay`], this method does not group holes with their containing outer
    /// contours. Outer contours and holes are distinguished by their winding direction.
    /// This method consumes the prepared input geometry.
    #[inline]
    pub fn overlay_contours(
        self,
        overlay_rule: OverlayRule,
        fill_rule: FillRule,
    ) -> Vec<IntContour<I>> {
        self.process_overlay_contours(fill_rule, overlay_rule)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::cpu_count::CPUCount;
    use crate::core::fill_rule::FillRule;
    use crate::core::overlay::Overlay;
    use crate::core::overlay_rule::OverlayRule;
    use i_shape::int::area::Area;
    use i_shape::int_shape;

    #[test]
    fn overlay_returns_shapes_from_the_column_solver() {
        let subject = int_shape![[[-8, -4], [8, -4], [8, 4], [-8, 4]]];
        let shapes = Overlay::<i32>::with_cpu_count(&subject, &[], CPUCount::Single)
            .overlay(OverlayRule::Subject, FillRule::NonZero);

        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].len(), 1);
        assert_eq!(shapes.area_two(), 256i64);
    }

    #[test]
    fn overlay_contours_skips_hole_grouping() {
        let subject = int_shape![[[-8, -8], [8, -8], [8, 8], [-8, 8]]];
        let clip = int_shape![[[-4, -4], [4, -4], [4, 4], [-4, 4]]];
        let contours = Overlay::<i32>::with_contours(&subject, &clip)
            .overlay_contours(OverlayRule::Difference, FillRule::NonZero);

        assert_eq!(contours.len(), 2);
        assert_eq!(
            contours
                .iter()
                .map(|contour| contour.area_two())
                .sum::<i64>(),
            384
        );
        assert_eq!(
            contours
                .iter()
                .filter(|contour| contour.area_two() > 0i64)
                .count(),
            1
        );
        assert_eq!(
            contours
                .iter()
                .filter(|contour| contour.area_two() < 0i64)
                .count(),
            1
        );
    }
}
