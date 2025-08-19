use alloc::vec::Vec;
use crate::core::winding::WindingCount;
use crate::ortho::column::Column;
use crate::ortho::segm::OrthoSegment;
use crate::sub::merge::Merge;

struct Mark {
    index: u32,
    value: i32,
}

impl<C: WindingCount + Clone> Column<C> {
    pub(super) fn split(&mut self) {
        self.vr_segments
            .sort_unstable_by(|s0, s1| s0.pos.cmp(&s1.pos).then(s0.min.cmp(&s1.min)));
        self.hz_segments
            .sort_unstable_by(|s0, s1| s0.min.cmp(&s1.min).then(s0.pos.cmp(&s1.pos)));

        let mut vr_marks = Vec::with_capacity(self.vr_segments.len().ilog2().min(16) as usize);
        let mut hz_marks = Vec::with_capacity(self.vr_segments.len().ilog2().min(16) as usize);

        let mut i = 0;

        for (hz_index, hz) in self.hz_segments.iter().enumerate() {
            while i < self.vr_segments.len() && self.vr_segments[i].pos < hz.min {
                i += 1;
            }

            let mut vr_index = i;

            while vr_index < self.vr_segments.len() {
                let vr = &self.vr_segments[vr_index];
                if vr.pos > hz.max {
                    break;
                }

                if vr.is_inside(hz.pos) {
                    vr_marks.push(Mark {
                        index: vr_index as u32,
                        value: hz.pos,
                    });
                }
                if hz.is_inside(vr.pos) {
                    hz_marks.push(Mark {
                        index: hz_index as u32,
                        value: vr.pos,
                    });
                }
                vr_index += 1;
            }
        }

        if !self.border_points.is_empty() {
            self.border_points.sort_unstable();

            for (hz_index, hz) in self.hz_segments.iter().enumerate() {
                if hz.pos != self.min {
                    continue;
                }

                while i < self.border_points.len() && self.border_points[i] <= hz.min {
                    i += 1;
                }

                let mut j = i;

                while j < self.border_points.len() {
                    let value = self.border_points[j];
                    if value >= hz.max {
                        break;
                    }

                    hz_marks.push(Mark {
                        index: hz_index as u32,
                        value,
                    });

                    j += 1;
                }
            }
        }

        if !vr_marks.is_empty() {
            split_segments(&mut self.vr_segments, vr_marks);
            self.vr_segments
                .sort_unstable_by(|s0, s1| s0.pos.cmp(&s1.pos).then(s0.min.cmp(&s1.min)));
        }
        self.vr_segments.merge_if_needed();

        if !hz_marks.is_empty() {
            split_segments(&mut self.hz_segments, hz_marks);
            self.hz_segments
                .sort_unstable_by(|s0, s1| s0.min.cmp(&s1.min).then(s0.pos.cmp(&s1.pos)));
        }
        self.hz_segments.merge_if_needed();
    }
}

fn split_segments<C: Clone>(segments: &mut Vec<OrthoSegment<C>>, mut marks: Vec<Mark>) {
    marks.sort_unstable_by(|m0, m1| m0.index.cmp(&m1.index).then(m0.value.cmp(&m1.value)));
    segments.reserve(marks.len());

    let mut i = 0;
    while i < marks.len() {
        let current_index = marks[i].index;
        let mut j = i + 1;
        while j < marks.len() && marks[j].index == current_index {
            j += 1;
        }

        let m0 = &marks[i];
        let s = unsafe { segments.get_unchecked_mut(m0.index as usize) };
        let mut si = s.cut_tail(m0.value);

        i += 1;
        if i < j {
            for m in marks[i..j].iter() {
                segments.push(si.cut_head(m.value));
            }
        }
        segments.push(si);

        i = j;
    }
}