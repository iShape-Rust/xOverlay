//! Parallel-execution configuration for the overlay solver.

/// Controls how much parallelism the overlay solver may use.
#[derive(Debug, Clone, Copy)]
pub enum CPUCount {
    /// Uses the number of threads in the current Rayon thread pool.
    Auto,
    /// Uses `count` as a solver layout hint without reconfiguring the Rayon thread pool.
    Fixed(usize),
    /// Forces serial execution.
    Single,
}

impl CPUCount {
    /// Selects [`Self::Auto`] when `multithreading` is enabled and [`Self::Single`] otherwise.
    #[inline]
    pub const fn new(multithreading: bool) -> Self {
        if multithreading {
            Self::Auto
        } else {
            Self::Single
        }
    }

    #[inline]
    pub(crate) fn count(&self) -> usize {
        #[cfg(feature = "allow_multithreading")]
        {
            match self {
                CPUCount::Auto => rayon::current_num_threads(),
                CPUCount::Fixed(count) => (*count).max(1),
                CPUCount::Single => 1,
            }
        }

        #[cfg(not(feature = "allow_multithreading"))]
        {
            1
        }
    }

    #[cfg(feature = "allow_multithreading")]
    #[inline]
    pub(crate) fn is_parallel(&self) -> bool {
        self.count() > 1
    }
}

#[cfg(all(test, feature = "allow_multithreading"))]
mod tests {
    use super::CPUCount;

    #[test]
    fn auto_uses_current_rayon_pool_size() {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .expect("test Rayon pool must be created");

        assert_eq!(pool.install(|| CPUCount::Auto.count()), 2);
    }
}
