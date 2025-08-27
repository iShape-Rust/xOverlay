use crate::geom::range::LineRange;

pub(crate) struct PositiveDiagonal {
    min_y: i32,
    x_range: LineRange,
}

pub(crate) struct NegativeDiagonal {
    min_y: i32,
    x_range: LineRange,
}

impl PositiveDiagonal {
    #[inline(always)]
    pub(crate) fn new(x_range: LineRange, min_y: i32) -> Self {
        Self {
            min_y,
            x_range,
        }
    }
}

impl NegativeDiagonal {
    #[inline(always)]
    pub(crate) fn new(x_range: LineRange, min_y: i32) -> Self {
        Self {
            min_y,
            x_range,
        }
    }
}

pub(crate) trait Diagonal {
    fn find_x(&self, y: i32) -> i32;
    fn find_y(&self, x: i32) -> i32;
}

impl Diagonal for PositiveDiagonal {
    #[inline(always)]
    fn find_y(&self, x: i32) -> i32 {
        let dx = x - self.x_range.min;
        self.min_y + dx
    }

    #[inline(always)]
    fn find_x(&self, y: i32) -> i32 {
        let dy = y - self.min_y;
        self.x_range.min + dy
    }
}

impl Diagonal for NegativeDiagonal {
    #[inline(always)]
    fn find_y(&self, x: i32) -> i32 {
        let dx = self.x_range.max - x;
        self.min_y + dx
    }

    #[inline(always)]
    fn find_x(&self, y: i32) -> i32 {
        let dy = y - self.min_y;
        self.x_range.max - dy
    }
}
