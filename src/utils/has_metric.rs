use glam::Vec2;

// used for floating-point comparison
// for types consist of multiple f32/f64, use the maximum metric for consistency
// btw does not need to satisfy the triangle inequality
// (this is because distance impls for f32/f64 output max(relative_dist, absolute_dist))
pub trait HasDistance {
    fn distance(&self, other: &Self) -> f32;
}

const FLOAT_TOLERANCE: f32 = 1e-4;

// https://stackoverflow.com/a/32334103
impl HasDistance for f32 {
    fn distance(&self, other: &Self) -> f32 {
        if *self == *other {
            return 0.0;
        }

        let diff = (*self - *other).abs();
        let norm = (self.abs() + other.abs()).clamp(FLOAT_TOLERANCE, f32::MAX);
        diff / norm
    }
}

impl HasDistance for f64 {
    fn distance(&self, other: &Self) -> f32 {
        if *self == *other {
            return 0.0;
        }

        let diff = (*self - *other).abs();
        let norm = (self.abs() + other.abs()).clamp(FLOAT_TOLERANCE as _, f64::MAX);
        (diff / norm) as _
    }
}

impl HasDistance for Vec2 {
    fn distance(&self, other: &Self) -> f32 {
        f32::max(self.x.distance(&other.x), self.y.distance(&other.y))
    }
}

#[test]
pub fn test() {
    assert!(1.0.distance(&1.0) == 0.0);
    assert!(1.0.distance(&1.0) < FLOAT_TOLERANCE);
    assert!(1.0.distance(&1.0000001) < FLOAT_TOLERANCE);
    assert!(1.0.distance(&2.0) > FLOAT_TOLERANCE);
    assert!(1.0.distance(&-1.0) > FLOAT_TOLERANCE);
    assert!(0.0.distance(&1e-9) < FLOAT_TOLERANCE);
}
