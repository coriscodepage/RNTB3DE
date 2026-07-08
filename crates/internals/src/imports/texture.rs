use std::{fs::File, path::Path};

use glam::vec4;
use renderer::texture::Texture;

pub fn from_png<P: AsRef<Path>>(path: P) -> Texture {
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
    Texture::new(color, info.width as i32, info.height as i32)
}
