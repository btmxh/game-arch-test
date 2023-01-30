use glam::Mat3;

use crate::ui::{
    common::{
        UIDrawContext, UIEventContext, UIStateOpacity, UIStatePos, UIStateSizeTrait,
        UIStateTransform,
    },
    utils::{
        geom::{UIPos, UISize},
        helpers::{pick_size, wrap_if_not_equals},
    },
};

pub fn relocate(
    _: &mut UIEventContext,
    self_pos: &mut UIStatePos,
    new_pos: UIPos,
    child_pos: &UIStatePos,
) -> RelocateToken {
    if new_pos == *self_pos {
        return RelocateToken::default();
    }

    let new_child_pos = UIPos::new(
        new_pos.x - self_pos.x + child_pos.x,
        new_pos.y - self_pos.y + child_pos.y,
    );

    *self_pos = new_pos;
    RelocateToken {
        new_child_pos: Some(new_child_pos),
    }
}

pub fn resize(
    _: &mut UIEventContext,
    self_pos: &mut UIStatePos,
    self_size: &mut impl UIStateSizeTrait,
    new_size: UISize,
    child_pos: &UIStatePos,
    child_size: &impl UIStateSizeTrait,
) -> ResizeToken {
    if **self_size == new_size {
        return ResizeToken::default();
    }

    **self_size = new_size;
    let new_child_size = pick_size(child_size, self_size);
    let new_child_pos = UIPos::new(
        self_pos.x + 0.5 * (self_size.width - new_child_size.width),
        self_pos.y + 0.5 * (self_size.height - new_child_size.height),
    );

    ResizeToken {
        new_child_pos: wrap_if_not_equals(new_child_pos, child_pos),
        new_child_size: wrap_if_not_equals(new_child_size, &**child_size),
    }
}

pub fn draw(
    _: &mut UIDrawContext,
    self_opacity: &UIStateOpacity,
    self_transform: &UIStateTransform,
    parent_opacity: f32,
    parent_transform: &Mat3,
) -> DrawToken {
    DrawToken {
        self_opacity: parent_opacity * **self_opacity,
        self_transform: parent_transform.mul_mat3(self_transform),
    }
}

// relocating container can only relocate child
#[derive(Default)]
pub struct RelocateToken {
    pub new_child_pos: Option<UIPos>,
}

// resizing container may end up relocating child
#[derive(Default)]
pub struct ResizeToken {
    pub new_child_pos: Option<UIPos>,
    pub new_child_size: Option<UISize>,
}

#[derive(Default)]
pub struct DrawToken {
    pub self_opacity: f32,
    pub self_transform: Mat3,
}
