use glam::{Affine2, Vec2};

use crate::ui::utils::geom::UIPos;

#[derive(Default)]
pub struct TransformStack(Vec<Affine2>);

impl TransformStack {
    pub fn push(&mut self) {
        self.0.push(self.0.last().copied().unwrap_or_default());
    }

    pub fn pop(&mut self) {
        self.0.pop().expect("empty stack");
    }

    pub fn peek(&self) -> &Affine2 {
        self.0.last().expect("empty stack")
    }

    pub fn peek_mut(&mut self) -> &mut Affine2 {
        self.0.last_mut().expect("empty stack")
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn apply(&mut self, transform: &Affine2) {
        let current = self.peek_mut();
        *current = *current * *transform;
    }

    pub fn translate(&mut self, offset: UIPos) {
        self.peek_mut().translation += Vec2::from(offset);
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn reset_current_transform(&mut self) {
        *self.peek_mut() = Affine2::IDENTITY;
    }
}
