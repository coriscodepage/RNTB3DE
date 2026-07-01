use std::{fs::File, path::Path};

use glam::{Vec4, vec4};
use wide::bytemuck;

pub struct Texture {
    color: Vec<Vec4>,
    width: i32,
    height: i32,
}

impl Texture {
    pub fn new() -> Self {
        Self {
            color: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    pub fn from_png<P: AsRef<Path>>(path: P) -> Self {
        let decoder = png::Decoder::new(std::io::BufReader::new(
            File::open::<P>(path.into()).unwrap(),
        ));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        let binding = buf[..info.buffer_size()]
            .iter()
            .copied()
            .map(|c| c as f32 / 255.0)
            .collect::<Vec<f32>>();
        let texture: &[[f32; 3]] = bytemuck::cast_slice(&binding);
        let color = texture
            .iter()
            .map(|tri| vec4(tri[0], tri[1], tri[2], 1.0))
            .collect();
        Self {
            color,
            width: info.width as i32,
            height: info.height as i32,
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
