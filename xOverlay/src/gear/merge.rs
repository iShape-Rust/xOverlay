use crate::core::winding::WindingCount;
use alloc::vec::Vec;
use crate::gear::segment::Segment;

pub(crate) trait Merge {
    fn merge_if_needed(&mut self);
    fn merge_after(&mut self, after: usize) -> usize;
}

impl Merge for Vec<Segment> {
    fn merge_if_needed(&mut self) {
        // data is already sorted

        if self.len() < 2 {
            return;
        }

        let mut prev = &self[0];
        for i in 1..self.len() {
            let this = &self[i];
            if prev.is_same_geometry(this) {
                let new_len = self.merge_after(i);
                self.truncate(new_len);
                return;
            }
            prev = this;
        }
    }

    fn merge_after(&mut self, after: usize) -> usize {
        let mut i = after;
        let mut j = i - 1;
        let mut prev = self[j].clone();

        while i < self.len() {
            if prev.is_same_geometry(&self[i]) {
                prev.dir = prev.dir.add(self[i].dir);
            } else {
                if prev.dir.is_not_empty() {
                    self[j] = prev;
                    j += 1;
                }
                prev = self[i].clone();
            }
            i += 1;
        }

        if prev.dir.is_not_empty() {
            self[j] = prev.clone();
            j += 1;
        }

        j
    }
}

impl Segment {
    #[inline(always)]
    fn is_same_geometry(&self, other: &Self) -> bool {
        self.pos == other.pos && self.range == other.range
    }
}