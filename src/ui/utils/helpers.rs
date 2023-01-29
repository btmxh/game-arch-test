use crate::ui::common::UIStateSizeTrait;

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

pub fn pick_size(size_state: &impl UIStateSizeTrait, parent_size: &UISize) -> UISize {
    let pref = size_state.pref_size();
    let mut max_size = size_state
        .max_size()
        .unwrap_or(INFINITE_SIZE)
        .clamp(&ZERO_SIZE, parent_size);
    if let Some(UISize { width, height }) = pref {
        max_size.width = max_size.width.max(width);
        max_size.height = max_size.height.max(height);
    }
    max_size
}
