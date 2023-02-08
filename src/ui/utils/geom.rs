use glam::Vec2;
use winit::dpi::LogicalSize;

#[derive(Debug, Clone, Copy, Default)]
pub struct UIPos {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UISize {
    pub width: f32,
    pub height: f32,
}

impl UIPos {
    pub const ZERO: UIPos = UIPos::new(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Vec2> for UIPos {
    fn from(v: Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<UIPos> for Vec2 {
    fn from(pos: UIPos) -> Self {
        Self::new(pos.x, pos.y)
    }
}

impl UISize {
    pub const ZERO: UISize = Self::new(0.0, 0.0);

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn clamp(&self, min: &UISize, max: &UISize) -> Self {
        Self {
            width: self.width.clamp(min.width, max.width),
            height: self.height.clamp(min.height, max.height),
        }
    }

    pub fn enclosed_in(&self, other: &UISize) -> bool {
        self.width <= other.width && self.height <= other.height
    }
}

impl From<Vec2> for UISize {
    fn from(v: Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<UISize> for Vec2 {
    fn from(size: UISize) -> Self {
        Self::new(size.width, size.height)
    }
}

impl From<LogicalSize<f32>> for UISize {
    fn from(s: LogicalSize<f32>) -> Self {
        Self::new(s.width, s.height)
    }
}

impl From<UISize> for LogicalSize<f32> {
    fn from(s: UISize) -> Self {
        Self::new(s.width, s.height)
    }
}

impl PartialEq for UIPos {
    fn eq(&self, other: &Self) -> bool {
        equals_2d((self.x, self.y), (other.x, other.y))
    }
}

impl PartialEq for UISize {
    fn eq(&self, other: &Self) -> bool {
        equals_2d((self.width, self.height), (other.width, other.height))
    }
}

fn equals_2d(lhs: (f32, f32), rhs: (f32, f32)) -> bool {
    const EPSILON: f32 = 0.01;
    let dx = lhs.0 - rhs.0;
    let dy = lhs.1 - rhs.1;
    dx * dx + dy * dy <= EPSILON
}

#[derive(Clone, Copy, Default)]
pub struct UIRect {
    pub pos: UIPos,
    pub size: UISize,
}

impl UIRect {
    pub fn contains(&self, pos: UIPos) -> bool {
        self.pos.x <= pos.x
            && pos.x <= self.pos.x + self.size.width
            && self.pos.y <= pos.y
            && pos.y <= self.pos.y + self.size.height
    }
}
