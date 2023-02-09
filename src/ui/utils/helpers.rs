use super::geom::{UIPos, UISize};

pub const ZERO_POS: UIPos = UIPos::new(0.0, 0.0);
pub const ZERO_SIZE: UISize = UISize::new(0.0, 0.0);
pub const INFINITE_SIZE: UISize = UISize::new(f32::INFINITY, f32::INFINITY);

pub fn wrap_if_not_equals<T>(value: T, compare_value: &T) -> Option<T>
where
    T: PartialEq<T>,
{
    (*compare_value != value).then_some(value)
}
