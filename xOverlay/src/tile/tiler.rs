use crate::geom::range::LineRange;
use crate::tile::column::TileColumn;
use crate::tile::min_max_x::XRangeAndCount;
use alloc::vec::Vec;
use i_shape::int::shape::IntContour;
use crate::tile::layout::TileLayout;
use crate::tile::mapper::TileMapper;
use crate::tile::source::GeometrySource;

struct Tiler;

impl Tiler {
    fn build_columns(subj: &[IntContour], clip: &[IntContour], parallel: bool, column_max_width_count: usize) -> Vec<TileColumn> {
        let layout = Self::calculate_layout(subj, clip, parallel, column_max_width_count);

        let mut mapper = TileMapper::new(layout);
        mapper.add_contours(subj);
        mapper.add_contours(clip);

        let mut columns = Vec::with_capacity(mapper.layout.count);
        for part in mapper.iter_by_parts() {
            columns.push(TileColumn {
                range: part.range,
                source: GeometrySource::with_part(part),
            });
        }

        columns
    }

    fn calculate_layout(subj: &[IntContour], clip: &[IntContour], parallel: bool, column_max_width_count: usize) -> TileLayout {
        debug_assert!(column_max_width_count.is_power_of_two());

        let (subj_range, subj_count) = subj.x_range_and_count(parallel);
        let (clip_range, clip_count) = clip.x_range_and_count(parallel);

        let range = LineRange::with_min_max(
            subj_range.min.min(clip_range.min),
            subj_range.max.max(clip_range.max),
        );
        let items_count = subj_count + clip_count;

        let avg_count_per_dimension = items_count.isqrt();
        let opt_chunks_count = avg_count_per_dimension >> column_max_width_count.ilog2();

        TileLayout::new(range, opt_chunks_count)
    }

    fn calculate_memory_size(subj: &[IntContour], clip: &[IntContour]) {



    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn test_0() {



    }
}