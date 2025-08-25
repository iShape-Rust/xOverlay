use crate::gear::section::Section;
use crate::gear::segment::Segment;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_float::int::rect::IntRect;
use i_key_sort::sort::key_sort::KeyBinSort;
use crate::gear::x_mapper::XMapper;
use crate::gear::y_layout::YLayout;
use crate::gear::y_mapper::YMapper;

impl Section {
    pub(super) fn split_border(&mut self) {
        if self.border_points.is_empty() {
            return;
        }
        let mut vr_indices = Vec::new();
        let max_x = self.layout.boundary().max_x;
        for (i, vr) in self.source.vr_list.iter().enumerate() {
            if vr.pos == max_x {
                vr_indices.push(i);
            }
        }

        if vr_indices.is_empty() {
            return;
        }

        self.border_points.sort_with_bins(|y0, y1| y0.cmp(y1));

        for vr_index in vr_indices {
            let vr0 = &mut self.source.vr_list[vr_index];

            let i0 = match self.border_points.binary_search(&vr0.min) {
                Ok(index) => index + 1,
                Err(left) => left + 1,
            };

            let mut n = 0;
            while i0 + n < self.border_points.len() && self.border_points[i0 + n] < vr0.max {
                n += 1;
            }

            if n == 0 {
                continue;
            }

            let y0 = self.border_points[i0];
            let mut vr = vr0.cut_tail(y0);

            for &y in self.border_points[i0 + 1..i0 + n + 1].iter() {
                let head = vr.cut_head(y);
                self.source.vr_list.push(head);
            }

            self.source.vr_list.push(vr);
        }

        self.border_points.clear();
    }
}

impl Section {
    pub(super) fn split(&mut self) {
        let (source_by_columns, map_by_columns) = self.source.map_by_columns(&self.layout);

        let mut vr_offset = 0;
        let mut hz_offset = 0;
        let mut dg_pos_offset = 0;
        let mut dg_neg_offset = 0;



        for part in map_by_columns.iter_by_parts() {
            let vr_slice = &source_by_columns.vr_list[vr_offset..part.vr];
            let hz_slice = &source_by_columns.hz_list[hz_offset..part.hz];
            let dg_pos_slice = &source_by_columns.dg_pos_list[dg_pos_offset..part.dg_pos];
            let dg_neg_slice = &source_by_columns.dg_neg_list[dg_neg_offset..part.dg_neg];

            // count


            // add this part started

            // add previous parts and clean

            // split


            vr_offset += part.vr;
            hz_offset += part.hz;
            dg_pos_offset += part.dg_pos;
            dg_neg_offset += part.dg_neg;
        }

    }

}


impl Segment {
    #[inline(always)]
    fn is_inside(&self, val: i32) -> bool {
        self.min < val && val < self.max
    }

    #[inline(always)]
    fn cut_tail(&mut self, mid: i32) -> Self {
        let tail = Self {
            pos: self.pos,
            min: mid,
            max: self.max,
            dir: self.dir,
        };

        self.max = mid;

        tail
    }

    #[inline(always)]
    fn cut_head(&mut self, mid: i32) -> Self {
        let tail = Self {
            pos: self.pos,
            min: self.min,
            max: mid,
            dir: self.dir,
        };

        self.min = mid;

        tail
    }
}