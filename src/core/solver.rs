pub struct Solver {
    pub multithreading: bool
}

impl Default for Solver {
    fn default() -> Self {
        Self {
            multithreading: true,
        }
    }
}