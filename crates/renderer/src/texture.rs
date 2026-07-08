use std::{fs::File, path::Path};

use glam::{Vec4, vec4};
use wide::bytemuck;

pub struct Texture {
    color: Vec<Vec4>,
    width: i32,
    height: i32,
}

impl Texture {
    pub fn new(color: Vec<Vec4>, width: i32, height: i32) -> Self {
        Self {
            color,
            width,
            height,
        }
    }

    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> Vec4 {
        let tex_x = (u * self.width as f32) as usize;
        let tex_y = (v * self.height as f32) as usize;
        let tex_idx = tex_y * self.width as usize + tex_x;
        self.color[tex_idx]
    }

    #[inline]
    pub fn sample_fail_silent(&self, u: f32, v: f32) -> Vec4 {
        let tex_x = (u * self.width as f32) as usize;
        let tex_y = (v * self.height as f32) as usize;
        let tex_idx = tex_y * self.width as usize + tex_x;
        if tex_idx >= self.color.len() {
            vec4(0.0, 0.0, 0.0, 1.0)
        } else {
            self.color[tex_idx]
        }
    }
}
