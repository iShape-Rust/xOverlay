use crate::core::fill_rule::FillRule;
use crate::core::integer::OverlayInt;
use crate::core::overlay_rule::OverlayRule;
use crate::core::winding::WindingCount;
use crate::deg_90::column;
use crate::deg_90::column::SolverBuffer;
use crate::deg_90::column_map::Column;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_float::int::number::wide_int::WideIntNumber;
use i_shape::int::area::Area;
use i_shape::int::shape::{IntContour, IntShapes};
#[cfg(feature = "allow_multithreading")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

pub(super) struct ContourChunk<I: OverlayInt> {
    pub(super) x_range: LineRange<I>,
    pub(super) contours: Vec<IntContour<I>>,
    pub(super) is_leaf: bool,
}

#[derive(Default)]
pub(super) struct ContourChunks<I: OverlayInt> {
    pub(super) left: Vec<IntContour<I>>,
    pub(super) middle: Vec<ContourChunk<I>>,
    pub(super) right: Vec<IntContour<I>>,
    pub(super) both: Vec<IntContour<I>>,
}

pub(super) struct SubGraph<I: OverlayInt> {
    pub(super) range: LineRange<I>,
    pub(super) chunks: ContourChunks<I>,
}

impl<I: OverlayInt> SubGraph<I> {
    #[cfg(test)]
    pub(super) fn with_column<W: WindingCount>(
        column: Column<I, W>,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
    ) -> Self {
        Self::with_column_buffer(
            column,
            fill_rule,
            overlay_rule,
            &mut SolverBuffer::default(),
        )
    }

    pub(super) fn with_column_buffer<W: WindingCount>(
        column: Column<I, W>,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
        buffer: &mut SolverBuffer<I, W>,
    ) -> Self {
        let range = column.range;
        let contours = column.extract_contours(fill_rule, overlay_rule, buffer);

        Self::with_contours(range, contours)
    }

    pub(super) fn with_contours(range: LineRange<I>, contours: Vec<IntContour<I>>) -> Self {
        let mut result = Self {
            range,
            chunks: ContourChunks::default(),
        };
        result.append_classified(contours);
        result
    }

    pub(super) fn append_classified(&mut self, mut contours: Vec<IntContour<I>>) {
        debug_assert!(self.chunks.both.is_empty());

        let mut middle = Vec::new();
        let mut index = 0;
        while index < contours.len() {
            let contour = &contours[index];
            let (left, right) = border_sides(contour, self.range);

            match (left, right) {
                (true, true) => index += 1,
                (true, false) => self.chunks.left.push(contours.swap_remove(index)),
                (false, true) => self.chunks.right.push(contours.swap_remove(index)),
                (false, false) => middle.push(contours.swap_remove(index)),
            }
        }

        if !middle.is_empty() {
            let is_leaf = self.chunks.middle.is_empty();
            self.chunks.middle.push(ContourChunk {
                x_range: self.range,
                contours: middle,
                is_leaf,
            });
        }
        self.chunks.both = contours;
    }

    pub(super) fn into_contours(self) -> Vec<IntContour<I>> {
        let ContourChunks {
            mut left,
            middle,
            mut right,
            mut both,
        } = self.chunks;

        let count = middle
            .iter()
            .map(|chunk| chunk.contours.len())
            .sum::<usize>()
            + left.len()
            + right.len()
            + both.len();
        let mut contours = Vec::with_capacity(count);
        for chunk in middle {
            contours.extend(chunk.contours);
        }
        contours.append(&mut left);
        contours.append(&mut right);
        contours.append(&mut both);
        contours
    }

    pub(super) fn into_shapes(self) -> IntShapes<I> {
        self.build_shapes(false)
    }

    #[cfg(feature = "allow_multithreading")]
    pub(super) fn parallel_into_shapes(self) -> IntShapes<I> {
        self.build_shapes(true)
    }

    fn build_shapes(self, parallel: bool) -> IntShapes<I> {
        let ContourChunks {
            mut left,
            middle,
            mut right,
            mut both,
        } = self.chunks;

        let count = both.len()
            + left.len()
            + right.len()
            + middle
                .iter()
                .filter(|chunk| !chunk.is_leaf)
                .map(|chunk| chunk.contours.len())
                .sum::<usize>();
        let mut base_contours = Vec::with_capacity(count);
        base_contours.append(&mut both);
        base_contours.append(&mut left);
        base_contours.append(&mut right);
        let mut leaf_chunks = Vec::new();
        for mut chunk in middle {
            if chunk.is_leaf {
                leaf_chunks.push(chunk);
            } else {
                base_contours.append(&mut chunk.contours);
            }
        }

        if has_single_outer_contour(&base_contours, &leaf_chunks) {
            return build_single_shape(base_contours, leaf_chunks);
        }

        let (mut shapes, base_info) = column::build_base_shapes(base_contours);

        #[cfg(feature = "allow_multithreading")]
        if parallel && leaf_chunks.len() > 1 {
            let chunk_results: Vec<_> = leaf_chunks
                .into_par_iter()
                .map(|chunk| column::build_shapes(chunk.contours, chunk.x_range, &base_info))
                .collect();
            for (mut chunk_shapes, base_holes) in chunk_results {
                for hole in base_holes {
                    shapes[hole.shape_index].push(hole.contour);
                }
                shapes.append(&mut chunk_shapes);
            }
            return shapes;
        }
        #[cfg(not(feature = "allow_multithreading"))]
        let _ = parallel;

        for chunk in leaf_chunks {
            let (mut chunk_shapes, base_holes) =
                column::build_shapes(chunk.contours, chunk.x_range, &base_info);
            for hole in base_holes {
                shapes[hole.shape_index].push(hole.contour);
            }
            shapes.append(&mut chunk_shapes);
        }
        shapes
    }
}

fn has_single_outer_contour<I: OverlayInt>(
    base_contours: &[IntContour<I>],
    leaf_chunks: &[ContourChunk<I>],
) -> bool {
    let mut outer_count = 0;
    let contours = base_contours
        .iter()
        .chain(leaf_chunks.iter().flat_map(|chunk| chunk.contours.iter()));

    for contour in contours {
        if contour.area_two() > I::Wide::ZERO {
            outer_count += 1;
            if outer_count > 1 {
                return false;
            }
        }
    }

    outer_count == 1
}

fn build_single_shape<I: OverlayInt>(
    base_contours: Vec<IntContour<I>>,
    leaf_chunks: Vec<ContourChunk<I>>,
) -> IntShapes<I> {
    let contour_count = base_contours.len()
        + leaf_chunks
            .iter()
            .map(|chunk| chunk.contours.len())
            .sum::<usize>();
    let mut shape = Vec::with_capacity(contour_count);
    shape.push(IntContour::new());

    let contours = base_contours
        .into_iter()
        .chain(leaf_chunks.into_iter().flat_map(|chunk| chunk.contours));
    for contour in contours {
        let area = contour.area_two();
        if area > I::Wide::ZERO {
            debug_assert!(shape[0].is_empty(), "only one outer contour is expected");
            shape[0] = contour;
        } else if area < I::Wide::ZERO {
            shape.push(contour);
        }
    }

    debug_assert!(!shape[0].is_empty(), "the outer contour must be present");
    alloc::vec![shape]
}

fn border_sides<I: OverlayInt>(contour: &IntContour<I>, range: LineRange<I>) -> (bool, bool) {
    let count = contour.len();
    if count < 2 {
        return (false, false);
    }

    let mut left = false;
    let mut right = false;

    for index in 0..count {
        let a = contour[index];
        let b = contour[(index + 1) % count];
        if a.x != b.x || a.y == b.y {
            continue;
        }

        left |= a.x == range.min;
        right |= a.x == range.max;
        if left && right {
            break;
        }
    }

    (left, right)
}
