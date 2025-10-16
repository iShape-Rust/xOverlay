use crate::core::winding::WindingCount;
use crate::gear::segment::Segment;
use crate::gear::winding_count::ShapeCountBoolean;
use crate::geom::range::LineRange;
use crate::partition::min_heap::PosMinHeap;
use crate::partition::pos::Pos;
use crate::partition::row::Row;
use alloc::vec::Vec;
use core::mem::swap;
use i_key_sort::sort::two_keys::TwoKeysSort;

pub(crate) struct PartitionSolver;

impl PartitionSolver {
    fn split_overlaps(rows: &mut [Row], parallel: bool) {
        if parallel {
            #[cfg(feature = "allow_multithreading")]
            {
                use rayon::iter::ParallelIterator;
                use rayon::prelude::IntoParallelRefMutIterator;

                rows.par_iter_mut().for_each(|row| {
                    let mut buffer = Vec::new();
                    let mut line_solver = LineSolver {
                        pos: 0,
                        ends_heap: PosMinHeap::with_capacity(16),
                    };
                    row.split_overlaps(&mut buffer, &mut line_solver);
                    swap(&mut row.segments, &mut buffer);
                });
                return;
            }
            #[cfg(not(feature = "allow_multithreading"))]
            {
                debug_assert!(
                    false,
                    "parallel partitioning requested without allow_multithreading feature"
                );
            }
        }

        let mut line_solver = LineSolver {
            pos: 0,
            ends_heap: PosMinHeap::with_capacity(16),
        };
        let mut buffer = Vec::new();

        for row in rows {
            row.split_overlaps(&mut buffer, &mut line_solver);
            row.segments.clear();
            row.segments.extend_from_slice(&buffer);
        }
    }
}
impl Row {
    fn split_overlaps(&mut self, buffer: &mut Vec<Segment>, line_solver: &mut LineSolver) {
        if self.segments.is_empty() {
            return;
        }
        self.segments
            .sort_by_two_keys_and_buffer(false, buffer, |s| s.pos, |s| s.range.min);

        buffer.clear();

        let n = self.segments.len();
        let mut i = 0;
        while i < n {
            let start = i;
            let pos = self.segments[i].pos;
            i += 1;

            while i < n && self.segments[i].pos == pos {
                i += 1;
            }
            line_solver.pos = pos;
            line_solver.split(&self.segments[start..i], buffer);
        }
    }
}

fn fast_check(segments: &[Segment]) -> Option<usize> {
    // check may be there is no overlap at all (often case)

    let mut x0 = i32::MIN;
    for (i, s) in segments.iter().enumerate() {
        if x0 > s.range.min {
            let i0 = i.saturating_sub(1);
            return Some(i0);
        }
        x0 = s.range.max;
    }

    None
}

struct LineSolver {
    pos: i32,
    ends_heap: PosMinHeap,
}

impl LineSolver {
    fn split(&mut self, segments: &[Segment], result: &mut Vec<Segment>) {
        let split = if let Some(split) = fast_check(segments) {
            split
        } else {
            result.extend_from_slice(segments);
            return;
        };

        if split > 0 {
            result.extend_from_slice(&segments[0..split]);
        }

        let mut head = Pos {
            x: i32::MIN,
            count: ShapeCountBoolean::empty(),
        };

        self.ends_heap.clear();

        for s in segments[split..].iter() {
            self.scan_heap_until(s.range.min, &mut head, result);

            if head.x == s.range.min {
                head.count = head.count.add(s.count);
            } else {
                self.add_head(&head, s.range.min, result);

                head = Pos {
                    x: s.range.min,
                    count: head.count.add(s.count),
                };
            }

            self.ends_heap.push(Pos {
                x: s.range.max,
                count: s.count,
            });
        }

        self.scan_heap_until(i32::MAX, &mut head, result);

        debug_assert!(!head.count.is_not_empty());
    }

    #[inline]
    fn scan_heap_until(&mut self, max_x: i32, head: &mut Pos, result: &mut Vec<Segment>) {
        while !self.ends_heap.is_empty() && self.ends_heap.min_x() <= max_x {
            let e0 = self.ends_heap.pop();

            self.add_head(head, e0.x, result);

            let mut count = head.count.sub(e0.count);

            while !self.ends_heap.is_empty() && self.ends_heap.min_x() == e0.x {
                let e = self.ends_heap.pop();
                count = count.sub(e.count);
            }

            *head = Pos { x: e0.x, count };
        }
    }

    #[inline]
    fn add_head(&self, head: &Pos, max_x: i32, result: &mut Vec<Segment>) {
        if head.count.is_not_empty() {
            if let Some(last) = result.last_mut() {
                if last.count == head.count && last.pos == self.pos && head.x == last.range.max {
                    last.range.max = max_x;
                    return;
                }
            }

            result.push(Segment {
                pos: self.pos,
                range: LineRange::with_min_max(head.x, max_x),
                count: head.count,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::winding::WindingCount;
    use crate::gear::segment::Segment;
    use crate::gear::winding_count::ShapeCountBoolean;
    use crate::geom::range::LineRange;
    use crate::partition::min_heap::PosMinHeap;
    use crate::partition::solver::LineSolver;
    use alloc::vec::Vec;

    impl LineSolver {
        fn test_split(&mut self, segments: &mut [Segment]) -> Vec<Segment> {
            segments.sort_unstable_by(|s0, s1| s0.range.min.cmp(&s1.range.min));
            let mut result = Vec::new();

            self.split(segments, &mut result);

            result
        }
    }

    #[test]
    fn test_0() {
        let s0 = Segment {
            pos: 0,
            range: LineRange::with_min_max(0, 10),
            count: ShapeCountBoolean::new(1, 0),
        };
        let s1 = Segment {
            pos: 0,
            range: LineRange::with_min_max(5, 15),
            count: ShapeCountBoolean::new(-1, 0),
        };

        let mut solver = LineSolver {
            pos: 0,
            ends_heap: PosMinHeap::with_capacity(16),
        };

        let result = solver.test_split(&mut [s0, s1]);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].range, LineRange::with_min_max(0, 5));
        assert_eq!(result[0].count, ShapeCountBoolean::new(1, 0));
        assert_eq!(result[1].range, LineRange::with_min_max(10, 15));
        assert_eq!(result[1].count, ShapeCountBoolean::new(-1, 0));
    }

    #[test]
    fn test_1() {
        let s0 = Segment {
            pos: 0,
            range: LineRange::with_min_max(0, 10),
            count: ShapeCountBoolean::new(1, 0),
        };
        let s1 = Segment {
            pos: 0,
            range: LineRange::with_min_max(0, 5),
            count: ShapeCountBoolean::new(1, 0),
        };
        let s2 = Segment {
            pos: 0,
            range: LineRange::with_min_max(5, 10),
            count: ShapeCountBoolean::new(1, 0),
        };

        let mut solver = LineSolver {
            pos: 0,
            ends_heap: PosMinHeap::with_capacity(16),
        };
        let result = solver.test_split(&mut [s0, s1, s2]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].range, LineRange::with_min_max(0, 10));
        assert_eq!(result[0].count, ShapeCountBoolean::new(2, 0));
    }

    #[test]
    fn test_2() {
        let s0 = Segment {
            pos: 0,
            range: LineRange::with_min_max(0, 15),
            count: ShapeCountBoolean::new(1, 0),
        };
        let s1 = Segment {
            pos: 0,
            range: LineRange::with_min_max(5, 10),
            count: ShapeCountBoolean::new(-1, 0),
        };

        let mut solver = LineSolver {
            pos: 0,
            ends_heap: PosMinHeap::with_capacity(16),
        };

        let result = solver.test_split(&mut [s0, s1]);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].range, LineRange::with_min_max(0, 5));
        assert_eq!(result[0].count, ShapeCountBoolean::new(1, 0));

        assert_eq!(result[1].range, LineRange::with_min_max(10, 15));
        assert_eq!(result[1].count, ShapeCountBoolean::new(1, 0));
    }

    #[test]
    fn test_3() {
        let s0 = Segment {
            pos: 0,
            range: LineRange::with_min_max(0, 15),
            count: ShapeCountBoolean::new(1, 0),
        };
        let s1 = Segment {
            pos: 0,
            range: LineRange::with_min_max(5, 10),
            count: ShapeCountBoolean::new(1, 0),
        };

        let mut solver = LineSolver {
            pos: 0,
            ends_heap: PosMinHeap::with_capacity(16),
        };
        let result = solver.test_split(&mut [s0, s1]);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].range, LineRange::with_min_max(0, 5));
        assert_eq!(result[0].count, ShapeCountBoolean::new(1, 0));

        assert_eq!(result[1].range, LineRange::with_min_max(5, 10));
        assert_eq!(result[1].count, ShapeCountBoolean::new(2, 0));

        assert_eq!(result[2].range, LineRange::with_min_max(10, 15));
        assert_eq!(result[2].count, ShapeCountBoolean::new(1, 0));
    }
}
