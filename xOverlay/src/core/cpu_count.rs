#[derive(Debug, Clone, Copy)]
pub enum CPUCount {
    Auto,
    Fixed(usize),
    Single,
}

impl CPUCount {
    #[inline]
    pub(crate) fn count(&self) -> usize {
        #[cfg(feature = "allow_multithreading")]
        {
            extern crate std;
            return match self {
                CPUCount::Auto => match std::thread::available_parallelism() {
                    Ok(value) => value.get(),
                    Err(_) => 1,
                },
                CPUCount::Fixed(count) => (*count).max(1),
                CPUCount::Single => 1,
            }
        }

        1
    }

    #[inline]
    pub(crate) fn is_parallel(&self) -> bool {
        self.count() > 1
    }
}
