use crate::geom::range::LineRange;
use crate::tile::column::Column;
use crate::tile::min_max_x::XRangeAndCount;
use alloc::vec::Vec;
use i_shape::int::shape::IntContour;

const MIN_COLUMN_COUNT: usize = 4;

struct Tiler;

impl Tiler {
    fn build_columns(subj: &[IntContour], clip: &[IntContour]) -> Vec<Column> {
        let (subj_range, subj_count) = subj.x_range_and_count();
        let (clip_range, clip_count) = clip.x_range_and_count();

        let range = LineRange::with_min_max(
            subj_range.min.min(clip_range.min),
            subj_range.max.max(clip_range.max),
        );
        let items_count = subj_count + clip_count;

        if items_count < MIN_COLUMN_COUNT {
            panic!("return trivial solution");
        }




        Vec::new()
    }
}
