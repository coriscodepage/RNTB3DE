use bumpalo::Bump;
use glam::Vec4;
use parking_lot::RwLock;

use crate::framebuffer::Framebuffer;

pub struct Renderer {
    framebuffers: Vec<RwLock<Option<Framebuffer>>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            framebuffers: Vec::new(),
        }
    }

    pub fn create_framebuffer(&mut self, width: usize, height: usize) -> usize {
        let fb = Framebuffer::new(width, height);
        self.framebuffers.push(RwLock::new(Some(fb)));
        self.framebuffers.len() - 1
    }

    pub(crate) fn take_framebuffer(&mut self, id: usize) -> Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            self.framebuffers[id].write().take().unwrap()
        }
    }

    pub(crate) fn clone_empty_framebuffer(&mut self, id: usize) -> Framebuffer {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            Framebuffer::new(
                self.framebuffers[id].read().as_ref().unwrap().width() as usize,
                self.framebuffers[id].read().as_ref().unwrap().height() as usize,
            )
        }
    }

    pub(crate) fn put_framebuffer(&mut self, id: usize, fb: Framebuffer) {
        if id < self.framebuffers.len() {
            *self.framebuffers[id].write() = Some(fb);
        } else {
            panic!("Invalid framebuffer ID: {}", id);
        }
    }

    #[inline]
    pub fn buffer_to_u8(&self, id: usize, out: &mut [i32]) {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            let binding = self.framebuffers[id].read();
            let buffer = binding.as_ref().unwrap();
            let (ra, ga, ba, _, generation) = buffer.get_color();
            for i in 0..ra.len() {
                if unsafe { *generation.get_unchecked(i) } == buffer.current_generation() {
                    let r: i32 = unsafe { (ra[i] * 255.0).to_int_unchecked() };
                    let g: i32 = unsafe { (ga[i] * 255.0).to_int_unchecked() };
                    let b: i32 = unsafe { (ba[i] * 255.0).to_int_unchecked() };
                    let combined = ((r as i32) << 16) + ((g as i32) << 8) + (b as i32);
                    unsafe { *out.get_unchecked_mut(i) = combined };
                }
            }
        }
    }

    pub fn clear_framebuffer(&mut self, id: usize) {
        if id >= self.framebuffers.len() {
            panic!("Invalid framebuffer ID: {}", id);
        } else {
            self.framebuffers[id]
                .write()
                .as_mut()
                .unwrap()
                .clear(Vec4::new(0.0, 0.0, 0.0, 0.0));
        }
    }
}
