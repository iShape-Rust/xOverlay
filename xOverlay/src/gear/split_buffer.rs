use crate::gear::segment::Segment;
use crate::gear::y_layout::YLayout;
use crate::gear::y_mapper::YMapper;
use alloc::vec::Vec;
use i_float::int::rect::IntRect;

#[derive(Debug, Clone)]
struct SplitHz {
    index: usize,
    y: i32,
    min_x: i32,
    max_x: i32,
}

#[derive(Debug, Clone)]
struct SplitDg {
    index: usize,
    y: i32,
    rect: IntRect,
}

pub(super) struct SplitBuffer {
    mapper: YMapper,
    hz_edges: Vec<SplitHz>,
    dg_pos_edges: Vec<SplitDg>,
    dg_neg_edges: Vec<SplitDg>,
    // vr_edges: Vec<SplitEdge>,
    // hz_edges: Vec<SplitEdge>,
    // dg_pos_edges: Vec<SplitEdge>,
    // dg_neg_edges: Vec<SplitEdge>,
}/*

impl SplitBuffer {
    fn new(rect: IntRect, log_height: u32) -> Self {
        let layout = YLayout::new(rect, log_height);
        let mapper = YMapper::new(layout);
        Self {
            mapper,
            hz_edges: Vec::new(),
            dg_pos_edges: Vec::new(),
            dg_neg_edges: Vec::new(),
        }
    }

    fn add_hz_segments(&mut self, min_x: i32, max_x: i32, offset_index: usize, segments: &[Segment]) {
        self.mapper.add_hz_list(segments);
        self.hz_edges.resize(segments.len(), SplitEdge::default());
        for hz in segments.iter().enumerate() {
            let i = self.mapper.next_hz_index(hz.pos);
            let e = SplitEdge::new_hz(min_x, max_x, );
            unsafe {
                *self.hz_edges.get_unchecked_mut(i) = e;
            }
        }

    }

    fn add_dg_pos_segments(&mut self, offset_index: usize, segments: &[Segment]) {

    }

    fn add_dg_neg_segments(&mut self, offset_index: usize, segments: &[Segment]) {

    }

    fn split_with(&self, offset_index: usize, vr_segments: &[Segment]) {
        for (local_index,vr) in vr_segments.iter().enumerate() {
            let i0 = self.mapper.layout.bottom_index(vr.min);
            let i1 = self.mapper.layout.top_single_index(vr.min);
            for i in i0..i1 {
                let vr_range =



            }
        }
    }
}

impl SplitEdge {

    fn new_hz(index: usize, min_x: i32, max_x: i32, hz: &Segment) -> Self {

    }

}

impl Default for SplitEdge {

    #[inline]
    fn default() -> Self {
        Self {
            index: 0,
            rect: IntRect {
                min_x: 0,
                max_x: 0,
                min_y: 0,
                max_y: 0,
            },

            parent: Default::default(),
        }
    }
}
*/