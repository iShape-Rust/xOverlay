use crate::core::shape_type::ShapeType;

pub(crate) trait WindingCount
where
    Self: Clone + Copy + Send + Eq,
{
    fn is_not_empty(&self) -> bool;
    fn empty() -> Self;
    fn new(subj: i16, clip: i16) -> Self;
    fn with_shape_type(shape_type: ShapeType) -> (Self, Self);
    fn add(self, count: Self) -> Self;
    fn invert(self) -> Self;
}