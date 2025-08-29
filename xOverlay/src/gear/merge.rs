use crate::core::winding::WindingCount;
use alloc::vec::Vec;
use crate::gear::segment::Segment;

pub(crate) trait Merge {
    fn merge_if_needed(&mut self) -> bool;
    fn merge_after(&mut self, after: usize) -> usize;
}

impl Merge for Vec<Segment> {
    fn merge_if_needed(&mut self) -> bool {
        // data is already sorted

        match self.len() {
            0 => return false,
            1 => {
                if self[0].is_zero_length() {
                    self.clear();
                    return true;
                }
                return false;
            }
            _ => {}
        }

        if self[0].is_zero_length() {
            let new_len = self.merge_after(0);
            self.truncate(new_len);
            return true;
        }

        let mut prev = &self[0];
        for i in 1..self.len() {
            let this = &self[i];
            if this.is_zero_length() || prev.is_same_geometry(this) {
                let new_len = self.merge_after(i);
                self.truncate(new_len);
                return true;
            }
            prev = this;
        }
        false
    }

    fn merge_after(&mut self, after: usize) -> usize {
        let n = self.len();
        if n == 0 { return 0; }

        let (mut w, mut r, mut prev) = if after > 0 {
            let w = after - 1;
            let prev = self[w].clone();
            let mut r = after;

            // advance r past any zero-lengths
            while r < n && self[r].is_zero_length() { r += 1; }

            (w, r, prev)
        } else {
            // nothing written yet; start from first non-zero
            let mut r = 0;
            while r < n && self[r].is_zero_length() { r += 1; }
            if r == n { return 0; } // all zero-length
            let prev = self[r].clone();
            r += 1;

            (0usize, r, prev)
        };

        while r < n {
            let s = &self[r];
            r += 1;

            if s.is_zero_length() {
                continue;
            }

            if prev.is_same_geometry(s) {
                prev.dir = prev.dir.add(s.dir);
            } else {
                let ss = s.clone();
                if prev.dir.is_not_empty() {
                    self[w] = prev;
                    w += 1;
                }
                prev = ss;
            }
        }

        if prev.dir.is_not_empty() {
            self[w] = prev;
            w += 1;
        }

        w
    }
}

impl Segment {
    #[inline(always)]
    fn is_same_geometry(&self, other: &Self) -> bool {
        self.pos == other.pos && self.range == other.range
    }

    #[inline(always)]
    fn is_zero_length(&self) -> bool {
        self.range.min == self.range.max
    }
}