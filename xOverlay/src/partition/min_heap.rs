use crate::partition::pos::Pos;
use alloc::vec::Vec;

pub(super) struct PosMinHeap {
    data: Vec<Pos>,
}

impl PosMinHeap {
    #[inline]
    pub(super) fn with_capacity(cap: usize) -> Self {
        Self {
            data: Vec::with_capacity(cap),
        }
    }

    #[inline]
    pub(super) fn clear(&mut self) {
        self.data.clear();
    }

    #[inline]
    pub(super) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub(super) fn min_x(&self) -> i32 {
        self.data.first().unwrap().x
    }

    pub(super) fn push(&mut self, item: Pos) {
        let x = item.x;
        let last = self.data.len();
        self.data.push(item);
        self.sift_up(last, x);
    }

    /// Pops the smallest-x `Pos`
    pub(super) fn pop(&mut self) -> Pos {
        let n = self.data.len();
        debug_assert!(n != 0);

        self.data.swap(0, n - 1);
        let min = self.data.pop().unwrap();

        if self.data.len() > 1 {
            self.sift_down(0);
        }
        min
    }

    #[inline]
    fn sift_up(&mut self, mut i: usize, x: i32) {
        while i > 0 {
            let j = (i - 1) / 2;
            let xp = unsafe { self.data.get_unchecked(j) }.x;
            if x < xp {
                self.data.swap(i, j);
                i = j;
            } else {
                break;
            }
        }
    }

    #[inline]
    fn sift_down(&mut self, mut i: usize) {
        let n = self.data.len();
        let x = unsafe { self.data.get_unchecked(i) }.x;
        loop {
            let l = 2 * i + 1;
            let r = l + 1;

            if l >= n {
                break;
            }

            let xl = unsafe { self.data.get_unchecked(l) }.x;

            if r >= n {
                if x > xl {
                    self.data.swap(i, l);
                }
                break;
            }

            let xr = unsafe { self.data.get_unchecked(r) }.x;

            let (min, j) = if xl < xr { (xl, l) } else { (xr, r) };

            if x < min {
                break;
            }

            self.data.swap(i, j);
            i = j;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::winding_count::ShapeCountBoolean;
    use alloc::vec;

    fn make_pos(x: i32) -> Pos {
        Pos {
            x,
            count: ShapeCountBoolean::default(),
        }
    }

    #[test]
    fn test_0() {
        let mut h = PosMinHeap::with_capacity(0);
        assert!(h.is_empty());

        h.clear(); // should be no-op
        assert!(h.is_empty());
    }

    #[test]
    fn test_1() {
        let mut h = PosMinHeap::with_capacity(1);
        h.push(make_pos(7));
        assert!(!h.is_empty());
        assert_eq!(h.min_x(), 7);
        assert_eq!(h.pop().x, 7);
        assert!(h.is_empty());
    }


    #[test]
    fn test_2() {
        let mut h = PosMinHeap::with_capacity(8);
        for x in [1, 2, 3] {
            h.push(make_pos(x));
        }
        for expected in [1, 2, 3] {
            assert_eq!(h.pop().x, expected);
        }
        assert!(h.is_empty());
    }

    #[test]
    fn test_3() {
        let mut h = PosMinHeap::with_capacity(8);
        for x in [1, 2, 3, 4, 5, 6, 7] {
            h.push(make_pos(x));
        }
        for expected in [1, 2, 3, 4, 5, 6, 7] {
            assert_eq!(h.pop().x, expected);
        }
        assert!(h.is_empty());
    }

    #[test]
    fn test_4() {
        let mut h = PosMinHeap::with_capacity(8);
        for x in [1, 2, 3, 4, 5, 6, 7, 8] {
            h.push(make_pos(x));
        }
        for expected in [1, 2, 3, 4, 5, 6, 7, 8] {
            assert_eq!(h.pop().x, expected);
        }
        assert!(h.is_empty());
    }

    #[test]
    fn test_5() {
        // Heap needn't be stable, just ordered by min.
        let mut h = PosMinHeap::with_capacity(8);
        let vals = [5, 3, 3, 7, 1, 1, 1, 4];
        for &x in &vals {
            h.push(make_pos(x));
        }

        let mut popped = vec![];
        while !h.is_empty() {
            popped.push(h.pop().x);
        }
        // Must be sorted nondecreasing.
        assert!(popped.windows(2).all(|w| w[0] <= w[1]));
        // multiset must match
        let mut a = vals.to_vec();
        a.sort();
        assert_eq!(popped, a);
    }

    #[test]
    fn interleaved_push_pop_maintains_heap_property() {
        let mut h = PosMinHeap::with_capacity(16);

        // mix operations
        h.push(make_pos(10));
        h.push(make_pos(2));
        h.push(make_pos(7));
        assert_eq!(h.min_x(), 2);
        assert_eq!(h.pop().x, 2);

        h.push(make_pos(3));
        h.push(make_pos(8));
        assert_eq!(h.pop().x, 3);
        assert_eq!(h.pop().x, 7);
        assert_eq!(h.pop().x, 8);
        assert_eq!(h.pop().x, 10);
        assert!(h.is_empty());
    }

    #[test]
    fn stress_random_like_sequence() {
        // Deterministic pseudo-random-ish sequence without rand.
        let mut h = PosMinHeap::with_capacity(128);
        let mut seed = 1234567u32;

        for _ in 0..200 {
            // xorshift32-ish
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let x = (seed as i32) & 0x7FFF;

            if (seed & 1) == 0 || h.is_empty() {
                h.push(make_pos(x));
            } else {
                // pop and check heap invariant around peek
                let _ = h.pop();
                if !h.is_empty() {
                    let top = h.min_x();
                    // check local heap property: top is <= all entries
                    for p in &h.data {
                        assert!(top <= p.x);
                    }
                }
            }
        }

        // Drain must be sorted
        let mut out = vec![];
        while !h.is_empty() {
            out.push(h.pop().x);
        }
        assert!(out.windows(2).all(|w| w[0] <= w[1]));
    }
}
