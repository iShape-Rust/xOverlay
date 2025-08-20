use alloc::vec::Vec;

#[derive(Debug)]
pub(crate) enum OverlayNode {
    Bridge([usize; 2]),
    Cross(Vec<usize>),
}