pub(crate) trait Log2 {
    fn ilog2_ceil(&self) -> u32;
}

impl Log2 for u32 {
    fn ilog2_ceil(&self) -> u32 {
        let floor = self.ilog2();
        if self.is_power_of_two() {
            floor
        } else {
            floor + 1
        }
    }
}