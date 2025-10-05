use crate::core::options::IntOverlayOptions;
use crate::gear::process::SegmentsPack;
use crate::geom::x_segment::XSegment;
use crate::graph::data::{OverlayGraph, OverlayLink, OverlayNode};
use alloc::vec;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::ops::Range;
use i_float::int::point::IntPoint;
use i_float::int::rect::IntRect;
use i_key_sort::sort::two_keys::TwoKeysSort;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

#[derive(Debug, Clone, Copy)]
struct End {
    segment_index: usize,
    point: IntPoint,
}

#[derive(Clone, Copy, Debug)]
struct FirstSlot {
    node_index: usize, // node of the first end seen
    link_index: usize, // index into incidence `links` for that first end
}

impl OverlayGraph {
    pub(super) fn new(cpu: usize, pack: SegmentsPack, options: IntOverlayOptions) -> Self {
        let fills = pack.fills;
        let segments = pack.segments;

        let links_count = segments.len();
        let ends = Self::create_and_sort_ends(cpu, segments);
        let (links, nodes) = Self::create_links_and_nodes(links_count, ends);

        Self {
            options,
            nodes,
            links,
            fills,
        }
    }

    fn create_links_and_nodes(
        segments_count: usize,
        ends: Vec<End>,
    ) -> (Vec<OverlayLink>, Vec<OverlayNode>) {
        let mut nodes = Vec::with_capacity(ends.len() / 2);

        // edge between 2 nodes
        let mut links: Vec<OverlayLink> = Vec::with_capacity(ends.len());

        // binder holds the *first end index*
        // index is an order in segments
        let mut slots = vec![
            FirstSlot {
                node_index: usize::MAX,
                link_index: 0
            };
            segments_count
        ];

        let mut i = 0usize;
        while i < ends.len() {
            let point = ends[i].point;
            let start = i;
            i += 1;
            while i < ends.len() && ends[i].point == point {
                i += 1;
            }
            let ends_group = &ends[start..i];

            // we create a new node per an ends_group
            let node_index = nodes.len();

            nodes.push(OverlayNode {
                point,
                links: links.len()..links.len() + ends_group.len(),
            });

            for end in ends_group.iter() {
                let slot = unsafe { slots.get_unchecked_mut(end.segment_index) };

                // is it first slot or not
                if slot.node_index == usize::MAX {
                    // first slot

                    slot.node_index = node_index;
                    slot.link_index = links.len();

                    links.push(OverlayLink::with_segm_and_node(
                        end.segment_index,
                        usize::MAX,
                    ));
                } else {
                    // second slot

                    // update other link
                    let link = unsafe { links.get_unchecked_mut(slot.link_index) };
                    link.node_index = node_index;

                    // add self link
                    links.push(OverlayLink::with_segm_and_node(
                        end.segment_index,
                        slot.node_index,
                    ));
                }
            }
        }

        (links, nodes)
    }

    fn create_and_sort_ends(cpu: usize, segments: Vec<XSegment>) -> Vec<End> {
        let (cell_ranges, mut ends) = Self::create_ends(segments);
        Self::sort_ends(cpu, cell_ranges, &mut ends);
        ends
    }

    fn create_ends(segments: Vec<XSegment>) -> (Vec<Range<usize>>, Vec<End>) {
        // XSegment a, b sorted by x
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for s in segments.iter() {
            debug_assert!(s.a.x <= s.b.x, "XSegment endpoints must be x-sorted");
            min_x = min_x.min(s.a.x);
            max_x = max_x.max(s.b.x);

            // y: check both endpoints
            min_y = min_y.min(s.a.y.min(s.b.y));
            max_y = max_y.max(s.a.y.max(s.b.y));
        }

        let mut mapper = XMapper::new(IntRect::new(min_x, max_x, min_y, max_y));

        for s in segments.iter() {
            mapper.increment(s.a.x);
            mapper.increment(s.b.x);
        }

        let cell_ranges = mapper.layout_cell_ranges();
        let end_count = segments.len() * 2;
        let mut ends = Vec::with_capacity(end_count);
        let scratch: &mut [MaybeUninit<End>] = &mut ends.spare_capacity_mut()[..end_count];

        for (index, s) in segments.iter().enumerate() {
            let i0 = mapper.next_index(s.a.x);
            let i1 = mapper.next_index(s.b.x);
            let e0 = End {
                segment_index: index,
                point: s.a,
            };
            let e1 = End {
                segment_index: index,
                point: s.b,
            };
            unsafe {
                scratch.get_unchecked_mut(i0).write(e0);
                scratch.get_unchecked_mut(i1).write(e1);
            }
        }
        #[allow(clippy::uninit_vec)]
        unsafe {
            ends.set_len(end_count);
        }

        (cell_ranges, ends)
    }

    fn sort_ends(cpu: usize, cell_ranges: Vec<Range<usize>>, ends: &mut [End]) {
        if cpu <= 1 {
            // trivial mostly for debug
            let mut buffer = Vec::new();
            for range in cell_ranges {
                let slice = unsafe { ends.get_unchecked_mut(range) };
                slice.sort_by_two_keys_and_buffer(false, &mut buffer, |e| e.point.x, |e| e.point.y);
            }
        } else {
            let optimal_count_per_cpu = ends.len() / cpu;
            let mut src = ends;
            let mut count = 0;
            let mut offset = 0;
            let mut sub_ranges =
                Vec::with_capacity(2 * (cell_ranges.len() / optimal_count_per_cpu));
            let mut sections = Vec::with_capacity(optimal_count_per_cpu);
            for range in cell_ranges {
                count += range.len();
                sub_ranges.push((range.start - offset)..(range.end - offset));

                if count >= optimal_count_per_cpu {
                    let (left_src, right_src) = src.split_at_mut(count);
                    src = right_src;

                    sections.push(EndSortSection {
                        ranges: sub_ranges.to_vec(),
                        slice: left_src,
                    });

                    sub_ranges.clear();
                    offset += count;
                    count = 0;
                }
            }

            if !src.is_empty() {
                sections.push(EndSortSection {
                    ranges: sub_ranges,
                    slice: src,
                });
            }

            sections.par_iter_mut().for_each(|s| s.sort());
        }
    }
}

struct EndSortSection<'a> {
    ranges: Vec<Range<usize>>,
    slice: &'a mut [End],
}

impl EndSortSection<'_> {
    fn sort(&mut self) {
        let mut buf = Vec::new();
        for range in self.ranges.iter() {
            let slice = unsafe { self.slice.get_unchecked_mut(range.clone()) };
            slice.sort_by_two_keys_and_buffer(
                false,
                &mut buf,
                |e| e.point.y,
                |e| e.point.x,
            )
        }
    }
}

const COLUMNS_COUNT_PWR: u32 = 7;
const COLUMNS_COUNT: usize = 1 << COLUMNS_COUNT_PWR;

struct XMapper {
    count: [u32; COLUMNS_COUNT],
    offset: [usize; COLUMNS_COUNT],
    min_x: i32,
    scl_x: u32,
    cnt_x: usize,
}

impl XMapper {
    fn new(rect: IntRect) -> Self {
        let w = rect.width() as u32;
        let scl_x = Self::layout_power(w + 1);
        let min_x = rect.min_x;
        let cnt_x = (w as usize >> scl_x) + 1;
        Self {
            count: [0; COLUMNS_COUNT],
            offset: [0; COLUMNS_COUNT],
            min_x,
            scl_x,
            cnt_x,
        }
    }

    #[inline(always)]
    fn x_index(&self, x: i32) -> usize {
        let dx = (x - self.min_x) as usize;
        dx >> self.scl_x
    }

    #[inline(always)]
    fn layout_power(size: u32) -> u32 {
        // Find smallest p satisfying the bound.
        if size <= 1 {
            return 0;
        }
        let max_cells = (COLUMNS_COUNT as u32).saturating_sub(1);
        if size <= max_cells {
            return 0;
        }
        let numerator = size + max_cells - 1;
        let need = numerator / max_cells;
        need.next_power_of_two().ilog2() // p such that 2^p >= need
    }

    #[inline(always)]
    fn increment(&mut self, x: i32) {
        let index = self.x_index(x);
        unsafe {
            *self.count.get_unchecked_mut(index) += 1;
        }
    }

    #[inline(always)]
    fn layout_cell_ranges(&mut self) -> Vec<Range<usize>> {
        let mut offset = 0usize;
        let mut ranges = Vec::with_capacity(self.cnt_x);
        for ix in 0..self.cnt_x {
            let cell_offset = unsafe { self.offset.get_unchecked_mut(ix) };
            let n = unsafe { *self.count.get_unchecked(ix) as usize };
            *cell_offset = offset;
            ranges.push(offset..offset + n);
            offset += n;
        }

        ranges
    }

    #[inline(always)]
    fn next_index(&mut self, x: i32) -> usize {
        let index = self.x_index(x);
        unsafe {
            let offset = self.offset.get_unchecked_mut(index);
            let result = *offset;
            *offset += 1;
            result
        }
    }
}
