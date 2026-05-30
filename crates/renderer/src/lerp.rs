use std::ops::{Add, Mul};

pub trait Lerp: Mul<f32, Output = Self> + Add<Output = Self> + Copy {
    fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        *self * (1.0 - t) + *other * t
    }
}

impl Lerp for glam::Vec3 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        *self * (1.0 - t) + *other * t
    }
}

impl Lerp for glam::Vec4 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        *self * (1.0 - t) + *other * t
    }
}