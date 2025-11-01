use crate::geom::range::LineRange;
use crate::tile::column::TileColumn;
use alloc::vec::Vec;
use i_shape::int::shape::IntContour;
use crate::core::overlay::OverlayError;
use crate::core::shape_type::ShapeType;
use crate::core::cpu_count::CPUCount;
use crate::tile::layout::TileLayout;
use crate::tile::mapper::TileMapper;
use crate::util::x_range::XRangeAndCount;

pub(crate) struct TileMap {
    pub(super) columns: Vec<TileColumn>
}

impl TileMap {
    fn with_subj_and_clip(subj: &[IntContour], clip: &[IntContour], cpu: CPUCount, column_max_width_count: usize) -> Result<Self, OverlayError> {
        let layout = Self::calculate_layout(subj, clip, cpu, column_max_width_count);

        let mut mapper = TileMapper::new(layout);
        mapper.add_contours(subj);
        mapper.add_contours(clip);

        let mut tilemap = TileMap { columns: Vec::with_capacity(mapper.layout.count) };

        tilemap.pre_init_columns(&mapper);
        tilemap.add_contours(subj, ShapeType::Subject, &mapper.layout)?;
        tilemap.add_contours(clip, ShapeType::Clip, &mapper.layout)?;

        Ok(tilemap)
    }

    fn calculate_layout(subj: &[IntContour], clip: &[IntContour], cpu: CPUCount, column_max_width_count: usize) -> TileLayout {
        debug_assert!(column_max_width_count.is_power_of_two());

        let (subj_range, subj_count) = subj.x_range_and_count(cpu);
        let (clip_range, clip_count) = clip.x_range_and_count(cpu);

        let range = LineRange::with_min_max(
            subj_range.min.min(clip_range.min),
            subj_range.max.max(clip_range.max),
        );
        let items_count = subj_count + clip_count;

        let avg_count_per_dimension = items_count.isqrt();
        let opt_chunks_count = avg_count_per_dimension >> column_max_width_count.ilog2();

        TileLayout::new(range, opt_chunks_count)
    }
}


#[cfg(test)]
mod tests {

}