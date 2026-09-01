use std::{collections::HashMap, fs::File, path::Path};

use glam::vec4;
use renderer::texture::Texture;

const MAX_UNUSED_COUNT: u32 = 10;

pub enum TextureLoadState {
    Unloaded,
    Loading,
    Loaded(Texture),
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum TextureSrc {
    Png(&'static str),
}

impl TextureSrc {
    pub fn load(&self) -> Texture {
        match self {
            TextureSrc::Png(path) => from_png(path),
        }
    }
}

pub struct TextureStorage {
    store: HashMap<TextureSrc, TextureHandle>,
}

impl TextureStorage {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn add(&mut self, source: TextureSrc) {
        self.store.insert(source, TextureHandle::new());
    }

    pub fn tick(&mut self) {
        self.store.iter_mut().for_each(|(_, texture)| {
            if matches!(texture.state, TextureLoadState::Loaded(_)) {
                texture.unused_count += 1;
                if texture.unused_count >= MAX_UNUSED_COUNT {
                    texture.unload();
                }
            }
        });
    }

    fn load(&mut self, query: &TextureSrc) {
        if let Some(texture) = self.store.get_mut(query) {
            if matches!(texture.state, TextureLoadState::Unloaded) {
                let tex = query.load();
                texture.state = TextureLoadState::Loaded(tex);
                texture.unused_count = 0;
            }
        }
    }

    pub fn ensure_loaded(&mut self, query: &TextureSrc) {
        let needs_load = self
            .store
            .get(query)
            .map(|tex| matches!(tex.state, TextureLoadState::Unloaded))
            .unwrap();

        if needs_load {
            self.load(query);
        }
    }

    pub fn get(&self, query: &TextureSrc) -> &Texture {
        match &self.store.get(query).unwrap().state {
            TextureLoadState::Unloaded | TextureLoadState::Loading => panic!(),
            TextureLoadState::Loaded(texture) => texture,
        }
    }
}

pub struct TextureHandle {
    state: TextureLoadState,
    unused_count: u32,
}

impl TextureHandle {
    pub fn new() -> Self {
        Self {
            state: TextureLoadState::Unloaded,
            unused_count: 0,
        }
    }

    pub fn unload(&mut self) {
        self.state = TextureLoadState::Unloaded;
        self.unused_count = 0;
    }
}

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
