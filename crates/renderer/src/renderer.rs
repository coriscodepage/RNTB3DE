use glam::Vec4;

use crate::framebuffer::Framebuffer;

pub struct Renderer {
    framebuffers: Vec<Option<Framebuffer>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            framebuffers: Vec::new(),
        }
    }

    pub fn create_framebuffer(&mut self, width: usize, height: usize) -> usize {
        let fb = Framebuffer::new(width, height);
        self.framebuffers.push(Some(fb));
        self.framebuffers.len() - 1
    }

    pub(crate) fn take_framebuffer(&mut self, id: usize) -> Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            self.framebuffers[id].take().unwrap()
        }
    }

    pub(crate) fn put_framebuffer(&mut self, id: usize, fb: Framebuffer) {
        if id < self.framebuffers.len() {
            self.framebuffers[id] = Some(fb);
        } else {
            panic!("Invalid framebuffer ID: {}", id);
        }
    }

    pub fn borrow_framebuffer(&self, id: usize) -> &Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            self.framebuffers[id].as_ref().unwrap()
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
