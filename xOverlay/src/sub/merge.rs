use crate::core::winding::WindingCount;
use alloc::vec::Vec;

pub(crate) trait Merge<C> {
    fn merge_if_needed(&mut self);
    fn merge_after(&mut self, after: usize) -> usize;
}

impl<C: WindingCount, S: CountMergeable<C>> Merge<C> for Vec<S> {
    fn merge_if_needed(&mut self) {
        // data is already sorted by pos and min

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
                let count = prev.count().add(self[i].count());
                prev.update(count);
            } else {
                if prev.count().is_not_empty() {
                    self[j] = prev;
                    j += 1;
                }
                prev = self[i].clone();
            }
            i += 1;
        }

        if prev.count().is_not_empty() {
            self[j] = prev.clone();
            j += 1;
        }

        j
    }
}

pub(crate) trait CountMergeable<C: WindingCount>: Clone {
    fn is_same_geometry(&self, other: &Self) -> bool;
    fn count(&self) -> C;
    fn update(&mut self, count: C);
}
