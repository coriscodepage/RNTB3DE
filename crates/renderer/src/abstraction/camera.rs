use glam::{Mat4, Vec3};

pub struct Camera {
    eye: Vec3,
    center: Vec3,
    up: Vec3,
}

impl Camera {
    pub fn new(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        Self { eye, center, up }
    }

    pub fn to_mat4(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(
            self.eye,    // eye
            self.center, // target
            self.up,     // up
        )
    }
}
