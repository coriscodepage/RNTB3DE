use bumpalo::Bump;
use glam::Vec4;
use parking_lot::RwLock;
use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    slice::{ParallelSlice, ParallelSliceMut},
};
use wide::bytemuck;

use crate::{framebuffer::Framebuffer, texture::Texture};

pub struct Renderer {
    framebuffers: Vec<Option<Framebuffer>>,
    textures: Vec<Option<Texture>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            framebuffers: Vec::new(),
            textures: Vec::new(),
        }
    }

    pub fn create_framebuffer(&mut self, width: usize, height: usize) -> usize {
        let fb = Framebuffer::new(width, height);
        self.framebuffers.push(Some(fb));
        self.framebuffers.len() - 1
    }

    pub(crate) fn take_framebuffer(&mut self, id: usize) -> Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id); // FIXME: This does not check anything!!!!!!!
        } else {
            self.framebuffers[id].take().unwrap()
        }
    }

    pub fn borrow_framebuffer(&self, id: usize) -> &Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            self.framebuffers[id].as_ref().unwrap()
        }
    }

    pub fn borrow_framebuffer_mut(&mut self, id: usize) -> &mut Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            self.framebuffers[id].as_mut().unwrap()
        }
    }

    pub(crate) fn clone_empty_framebuffer(&mut self, id: usize) -> Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            let fb = self.framebuffers[id].as_ref().unwrap();
            Framebuffer::new(fb.width() as usize, fb.height() as usize)
        }
    }

    pub(crate) fn put_framebuffer(&mut self, id: usize, fb: Framebuffer) {
        if id < self.framebuffers.len() {
            self.framebuffers[id] = Some(fb);
        } else {
            panic!("Invalid framebuffer ID: {}", id);
        }
    }

    pub fn insert_texture(&mut self, tex: Texture) -> usize {
        self.textures.push(Some(tex));
        self.textures.len() - 1
    }

    pub(crate) fn take_texture(&mut self, id: usize) -> Texture {
        if id >= self.textures.len() {
            panic!("Invalid texture ID: {}", id);
        } else {
            self.textures[id].take().unwrap()
        }
    }

    pub(crate) fn borrow_texture(&self, id: usize) -> &Texture {
        if id >= self.textures.len() {
            panic!("Invalid texture ID: {}", id);
        } else {
            self.textures[id].as_ref().unwrap()
        }
    }

    pub(crate) fn put_texturte(&mut self, id: usize, tex: Texture) {
        if id < self.textures.len() {
            self.textures[id] = Some(tex);
        } else {
            panic!("Invalid texture ID: {}", id);
        }
    }

    pub fn clear_framebuffer(&mut self, id: usize) {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            self.framebuffers[id]
                .as_mut()
                .unwrap()
                .clear(Vec4::new(0.0, 0.0, 0.0, 0.0));
        }
    }
}

pub(crate) fn morton(mut x: u32, mut y: u32) -> u32 {
    // TODO: Either expand the morton or do a bounds check before. Usize is 64 bit while x and y got restrictions well bellow that
    // https://graphics.stanford.edu/~seander/bithacks.html#InterleaveBMN
    // x and y must initially be less than 65536.

    x = (x | (x << 8)) & 0x00FF00FF;
    x = (x | (x << 4)) & 0x0F0F0F0F;
    x = (x | (x << 2)) & 0x33333333;
    x = (x | (x << 1)) & 0x55555555;

    y = (y | (y << 8)) & 0x00FF00FF;
    y = (y | (y << 4)) & 0x0F0F0F0F;
    y = (y | (y << 2)) & 0x33333333;
    y = (y | (y << 1)) & 0x55555555;

    x | (y << 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec4;

    #[test]
    fn framebuffer_binding_round_trips_through_renderer() {
        let mut renderer = Renderer::new();
        let framebuffer_id = renderer.create_framebuffer(2, 2);

        let mut framebuffer = renderer.take_framebuffer(framebuffer_id);
        unsafe {
            framebuffer.write_fragment(0, 0, 0.25, vec4(1.0, 0.0, 0.0, 1.0));
        }
        renderer.put_framebuffer(framebuffer_id, framebuffer);

        let framebuffer = renderer.take_framebuffer(framebuffer_id);
        assert_eq!(framebuffer.read_pixel(0, 0), vec4(1.0, 0.0, 0.0, 1.0));
        renderer.put_framebuffer(framebuffer_id, framebuffer);

        renderer.clear_framebuffer(framebuffer_id);

        let framebuffer = renderer.take_framebuffer(framebuffer_id);
        assert_eq!(framebuffer.read_pixel(0, 0), vec4(0.0, 0.0, 0.0, 1.0));
        assert!(framebuffer.depth_test(0, 0, 0.5));
    }
}
