use glam::{Mat4, Vec3};

#[derive(Debug)]
pub struct Camera {
    eye: Vec3,
    view_direction: Vec3,
    up: Vec3,
    active: bool,
}

impl Camera {
    pub fn new(eye: Vec3, view_direction: Vec3, up: Vec3) -> Self {
        Self {
            eye,
            view_direction,
            up,
            active: false,
        }
    }

    pub fn to_mat4(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(
            self.eye,                       // eye
            self.eye + self.view_direction, // center
            self.up,                        // up
        )
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn release(&mut self) {
        self.active = false;
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn move_forward(&mut self, speed: f32) {
        self.eye += self.view_direction * speed;
    }

    pub fn move_back(&mut self, speed: f32) {
        self.eye -= self.view_direction * speed;
    }

    pub fn move_left(&mut self, speed: f32) {
        let right_vector = self.view_direction.cross(self.up).normalize();
        self.eye -= right_vector * speed;
    }

    pub fn move_right(&mut self, speed: f32) {
        let right_vector = self.view_direction.cross(self.up).normalize();
        self.eye += right_vector * speed;
    }

    pub fn mouse_look(&mut self, delta_x: f32, delta_y: f32) {
        self.view_direction = self
            .view_direction
            .rotate_axis(self.up, delta_x.to_radians() * 0.1);
        let right_vector = self.view_direction.cross(self.up).normalize();
        self.view_direction = self
            .view_direction
            .rotate_axis(right_vector, delta_y.to_radians() * 0.1);
    }
}
