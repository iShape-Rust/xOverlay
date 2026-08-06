use crate::core::fill_rule::FillRule;
use crate::core::overlay_rule::OverlayRule;
use crate::deg_90::column;
use crate::deg_90::column::SolverBuffer;
use crate::deg_90::column_map::Column;
use crate::geom::range::LineRange;
use alloc::vec::Vec;
use i_shape::int::shape::{IntContour, IntShapes};
#[cfg(feature = "allow_multithreading")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};

pub(super) struct ContourChunk {
    pub(super) x_range: LineRange,
    pub(super) contours: Vec<IntContour<i32>>,
    pub(super) is_leaf: bool,
}

#[derive(Default)]
pub(super) struct ContourChunks {
    pub(super) left: Vec<IntContour<i32>>,
    pub(super) middle: Vec<ContourChunk>,
    pub(super) right: Vec<IntContour<i32>>,
    pub(super) both: Vec<IntContour<i32>>,
}

pub(super) struct SubGraph {
    pub(super) range: LineRange,
    pub(super) chunks: ContourChunks,
}

impl SubGraph {
    #[cfg(test)]
    pub(super) fn with_column(
        column: Column,
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

    pub(super) fn with_column_buffer(
        column: Column,
        fill_rule: FillRule,
        overlay_rule: OverlayRule,
        buffer: &mut SolverBuffer,
    ) -> Self {
        let range = column.range;
        let contours = column.extract_contours(fill_rule, overlay_rule, buffer);

        Self::with_contours(range, contours)
    }

    pub(super) fn with_contours(range: LineRange, contours: Vec<IntContour<i32>>) -> Self {
        let mut result = Self {
            range,
            chunks: ContourChunks::default(),
        };
        result.append_classified(contours);
        result
    }

    pub(super) fn append_classified(&mut self, mut contours: Vec<IntContour<i32>>) {
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

    pub(super) fn into_contours(self) -> Vec<IntContour<i32>> {
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

    pub(super) fn into_shapes(self) -> IntShapes<i32> {
        self.build_shapes(false)
    }

    #[cfg(feature = "allow_multithreading")]
    pub(super) fn parallel_into_shapes(self) -> IntShapes<i32> {
        self.build_shapes(true)
    }

    fn build_shapes(self, parallel: bool) -> IntShapes<i32> {
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

fn border_sides(contour: &IntContour<i32>, range: LineRange) -> (bool, bool) {
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
