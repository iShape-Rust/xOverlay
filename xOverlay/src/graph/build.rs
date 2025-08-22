use crate::core::options::IntOverlayOptions;
use crate::graph::OverlayGraph;
use crate::graph::end::End;
use crate::graph::link::OverlayLink;
use crate::graph::node::OverlayNode;
use alloc::vec::Vec;
use i_float::int::point::IntPoint;
use i_key_sort::bin_key::index::BinLayout;
use i_shape::util::reserve::Reserve;
use rayon::prelude::ParallelSliceMut;

impl OverlayGraph {
    pub(crate) fn build(&mut self, option: IntOverlayOptions, multithreading: bool) {
        self.options = option;
        // at this time
        // links are sorted
        // ends are sorted

        let n = self.links.len();
        if n == 0 {
            return;
        }

        self.build_ends(multithreading);
        self.nodes.reserve_capacity(n);
        self.nodes.clear();

        let mut ai = 0;
        let mut bi = 0;
        let mut a = self.links[0].a.point;
        let mut b = self.ends[0].point;
        let mut next_a_cnt = self.links.size(a, ai);
        let mut next_b_cnt = self.ends.size(b, bi);
        let mut indices = Vec::with_capacity(4);
        while next_a_cnt > 0 || next_b_cnt > 0 {
            let (a_cnt, b_cnt) = if a == b {
                (next_a_cnt, next_b_cnt)
            } else if next_a_cnt > 0 && a < b {
                (next_a_cnt, 0)
            } else {
                (0, next_b_cnt)
            };

            let node_id = self.nodes.len();

            if a_cnt > 0 {
                next_a_cnt = 0;
                for _ in 0..a_cnt {
                    unsafe { self.links.get_unchecked_mut(ai) }.a.id = node_id;
                    indices.push(ai);
                    ai += 1;
                }
                if ai < n {
                    a = unsafe { self.links.get_unchecked(ai) }.a.point;
                    next_a_cnt = self.links.size(a, ai);
                }
            }

            if b_cnt > 0 {
                next_b_cnt = 0;
                for _ in 0..b_cnt {
                    let e = unsafe { self.ends.get_unchecked(bi) };
                    indices.push(e.index);
                    unsafe { self.links.get_unchecked_mut(e.index) }.b.id = node_id;
                    bi += 1;
                }

                if bi < n {
                    b = unsafe { self.ends.get_unchecked(bi) }.point;
                    next_b_cnt = self.ends.size(b, bi);
                }
            }

            self.nodes
                .push(OverlayNode::with_indices(indices.as_slice()));
            indices.clear();
        }
    }

    fn build_ends(&mut self, multithreading: bool) {
        if let Some(layout) = self.bin_layout() {
            self.bin_store.init(layout);
            self.bin_store
                .reserve_bins_space(self.links.iter().map(|link| &link.b.point.x));
            let count = self.bin_store.prepare_bins();
            self.ends.resize(count, End::default());

            for (i, link) in self.links.iter().enumerate() {
                self.bin_store.feed_vec(
                    &mut self.ends,
                    End {
                        index: i,
                        point: link.b.point,
                    },
                );
            }

            for bin in self.bin_store.bins.iter() {
                let start = bin.offset;
                let end = bin.data;
                if start < end {
                    self.ends[start..end].sort_by(|a, b| a.point.cmp(&b.point));
                }
            }
        } else {
            self.ends.clear();
            let additional = self.links.len().saturating_sub(self.ends.capacity());
            if additional > 0 {
                self.ends.reserve(additional);
            }
            for (i, link) in self.links.iter().enumerate() {
                self.ends.push(End {
                    index: i,
                    point: link.b.point,
                });
            }

            if multithreading {
                self.ends.par_sort_unstable_by(|a, b| a.point.cmp(&b.point));
            } else {
                self.ends.sort_unstable_by(|a, b| a.point.cmp(&b.point));
            }
        }
    }

    #[inline]
    fn bin_layout(&self) -> Option<BinLayout<i32>> {
        let count = self.links.len();
        if !(64..=1_000_000).contains(&count) {
            // direct approach work better for small and large data
            return None;
        }

        let mut min = i32::MAX;
        let mut max = i32::MIN;
        for link in self.links.iter() {
            min = min.min(link.b.point.x);
            max = max.max(link.b.point.x);
        }

        BinLayout::new(min..max, count)
    }
}

trait Size {
    fn size(&self, point: IntPoint, index: usize) -> usize;
}

impl<T: SortPoint> Size for [T] {
    #[inline]
    fn size(&self, point: IntPoint, index: usize) -> usize {
        let mut i = index + 1;
        while i < self.len() && self[i].point() == point {
            i += 1;
        }
        i - index
    }
}

trait SortPoint {
    fn point(&self) -> IntPoint;
}

impl SortPoint for OverlayLink {
    #[inline(always)]
    fn point(&self) -> IntPoint {
        self.a.point
    }
}

impl SortPoint for End {
    #[inline(always)]
    fn point(&self) -> IntPoint {
        self.point
    }
}

impl OverlayNode {
    #[inline(always)]
    fn with_indices(indices: &[usize]) -> Self {
        if indices.len() == 2 {
            Self::Bridge(unsafe { [*indices.get_unchecked(0), *indices.get_unchecked(1)] })
        } else {
            Self::Cross(indices.to_vec())
        }
    }
}
