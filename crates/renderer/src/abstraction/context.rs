use std::cell::{Ref, RefCell};

use arrayvec::ArrayVec;
use glam::Vec4;

use crate::{framebuffer::Framebuffer, renderer::Renderer, texture::Texture};

pub struct Context<'a, const COUNT: usize> {
    renderer: &'a RefCell<Renderer>,
    texture_binds: ArrayVec<usize, COUNT>,
    framebuffer_write_binds: ArrayVec<usize, COUNT>,
    framebuffer_read_binds: ArrayVec<usize, COUNT>,
    pub framebuffers_write_resolved: ArrayVec<Framebuffer, COUNT>, // TODO: Make this not public if able.
    textures_resolved: ArrayVec<Ref<'a, Texture>, COUNT>,
    framebuffers_read_resolved: ArrayVec<Ref<'a, Framebuffer>, COUNT>,
}

impl<'a, const COUNT: usize> Context<'a, COUNT> {
    pub fn new(renderer: &'a RefCell<Renderer>) -> Self {
        Self {
            renderer,
            texture_binds: ArrayVec::new(),
            framebuffer_read_binds: ArrayVec::new(),
            framebuffer_write_binds: ArrayVec::new(),
            textures_resolved: ArrayVec::new(),
            framebuffers_write_resolved: ArrayVec::new(),
            framebuffers_read_resolved: ArrayVec::new(),
        }
    }

    pub fn bind_texture(&mut self, id: usize) -> Result<usize, &'static str> {
        self.texture_binds
            .try_push(id)
            .map_err(|_| "No free binds left")?;
        Ok(self.texture_binds.len() - 1)
    }

    pub fn bind_framebuffers_write(&mut self, id: usize) -> Result<usize, &'static str> {
        if self.framebuffer_read_binds.iter().any(|v| *v == id) {
            return Err("Cant bind same framebuffer for read and write");
        }

        self.framebuffer_write_binds
            .try_push(id)
            .map_err(|_| "No free binds left")?;
        Ok(self.framebuffer_write_binds.len() - 1)
    }

    pub fn bind_framebuffers_read(&mut self, id: usize) -> Result<usize, &'static str> {
        if self.framebuffer_write_binds.iter().any(|v| *v == id) {
            return Err("Cant bind same framebuffer for read and write");
        }

        self.framebuffer_read_binds
            .try_push(id)
            .map_err(|_| "No free binds left")?;
        Ok(self.framebuffer_read_binds.len() - 1)
    }

    pub fn resolve(&mut self) {
        if self.framebuffer_write_binds.is_empty() {
            panic!("No render buffer attached to the context.");
        };

        self.framebuffer_write_binds.iter().for_each(|id| {
            self.framebuffers_write_resolved
                .push(self.renderer.borrow_mut().take_framebuffer(*id));
        });

        self.framebuffer_read_binds.iter().for_each(|id| {
            self.framebuffers_read_resolved
                .push(Ref::map(self.renderer.borrow(), |r| {
                    r.borrow_framebuffer(*id)
                }));
        });

        self.texture_binds.iter().enumerate().for_each(|(i, id)| {
            self.textures_resolved.push(Ref::map(self.renderer.borrow(), |r| r.borrow_texture(*id)))
        });
    }

    pub fn put_back(&mut self) {
        self.framebuffers_read_resolved.clear();
        self.textures_resolved.clear();
        for (i, id) in self.framebuffer_write_binds.iter().enumerate() {
            let fb = self.framebuffers_write_resolved.remove(i);
            self.renderer.borrow_mut().put_framebuffer(*id, fb);
        }
    }

    pub fn framebuffer_write_count(&self) -> usize {
        self.framebuffer_write_binds.len()
    }

    pub fn sample_texture(&self, id: usize, u: f32, v: f32) -> Vec4 {
        if id > self.texture_binds.len() {
            panic!("Texture index out of bounds")
        }
        self.textures_resolved[id].sample(u, v)
    }

    pub fn sample_texture_fail_silent(&self, id: usize, u: f32, v: f32) -> Vec4 {
        if id > self.texture_binds.len() {
            panic!("Texture index out of bounds")
        }
        self.textures_resolved[id].sample_fail_silent(u, v)
    }
}
