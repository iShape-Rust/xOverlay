#[derive(Debug, Clone, Copy)]
pub enum CPUCount {
    Auto,
    Fixed(usize),
    Single,
}

#[derive(Debug, Clone)]
pub struct Solver {
    pub cpu: CPUCount,
}

impl Default for Solver {
    fn default() -> Self {
        Self {
            cpu: CPUCount::Auto,
        }
    }
}

impl Solver {
    pub(crate) fn cpu_count(&self) -> usize {
        #[cfg(feature = "allow_multithreading")]
        {
            extern crate std;
            return match self.cpu {
                CPUCount::Auto => match std::thread::available_parallelism() {
                    Ok(value) => value.get(),
                    Err(_) => 1,
                },
                CPUCount::Fixed(count) => count,
                CPUCount::Single => 1,
            }
        }

        1
    }

    pub fn single() -> Self {
        Self {
            cpu: CPUCount::Single,
        }
    }

    pub fn new(multithreading: bool) -> Self {
        if multithreading {
            #[cfg(feature = "allow_multithreading")]
            {
                return Self {
                    cpu: CPUCount::Auto,
                };
            }
        }

        Self::single()
    }
}
