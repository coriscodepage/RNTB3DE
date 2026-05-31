use glam::Vec3;

use crate::lerp::Lerp;

#[derive(Debug, Clone, Copy)]
pub struct Vertex<T: Lerp> {
    pub position: glam::Vec3,
    pub data: T,
}

impl<T: Lerp> Vertex<T> {
    pub fn new(position: glam::Vec3, data: T) -> Self {
        Self { position, data }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FragmentInput<T: Lerp> {
    pub position: glam::IVec2,
    pub depth: f32,
    pub data: T,
}

impl<T: Lerp> FragmentInput<T> {
    pub fn new(position: glam::IVec2, depth: f32, data: T) -> Self {
        Self { position, depth, data }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Triangle<T>
where
    T: Lerp + Send + Sync,
{
    pub(crate) position: [Vec3; 3],
    pub(crate) data: [T; 3],
}
