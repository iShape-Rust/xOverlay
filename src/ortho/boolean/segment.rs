use crate::graph::boolean::winding_count::ShapeCountBoolean;
use crate::ortho::segment::OrthoSegment;

impl OrthoSegment<ShapeCountBoolean> {
    #[inline]
    pub(super) fn new_boolean(
        z0: i32,
        z1: i32,
        pos: i32,
        direct: ShapeCountBoolean,
        invert: ShapeCountBoolean,
    ) -> Self {
        if z0 < z1 {
            Self {
                pos,
                min: z0,
                max: z1,
                count: direct,
            }
        } else {
            Self {
                pos,
                min: z1,
                max: z0,
                count: invert,
            }
        }
    }
}