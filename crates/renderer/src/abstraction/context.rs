use std::{marker::PhantomData, ops::{Deref, DerefMut}};

use arrayvec::ArrayVec;
use glam::Vec4;

use crate::{
    framebuffer::Framebuffer,
    renderer::{FramebufferId, FramebufferStore, Renderer, TextureId, TextureStore},
    texture::Texture,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextureUnit(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FramebufferUnit(usize);

#[derive(Debug)]
pub struct RequestedSamplers<const COUNT: usize> {
    texture_binds: ArrayVec<TextureId, COUNT>,
    framebuffer_read_binds: ArrayVec<FramebufferId, COUNT>,
}

impl<'a, const COUNT: usize> RequestedSamplers<COUNT> {
    pub fn new() -> Self {
        Self {
            texture_binds: ArrayVec::new(),
            framebuffer_read_binds: ArrayVec::new(),
        }
    }

    #[inline]
    pub fn bind_texture(&mut self, id: TextureId) -> Result<TextureUnit, &'static str> {
        self.texture_binds
            .try_push(id)
            .map_err(|_| "No free binds left")?;
        Ok(TextureUnit(self.texture_binds.len() - 1))
    }

    #[inline]
    pub fn bind_framebuffers_read(
        &mut self,
        id: FramebufferId,
    ) -> Result<FramebufferUnit, &'static str> {
        self.framebuffer_read_binds
            .try_push(id)
            .map_err(|_| "No free binds left")?;
        Ok(FramebufferUnit(self.framebuffer_read_binds.len() - 1))
    }

    #[inline]
    pub fn get_texture_binds(&self) -> &[TextureId] {
        &self.texture_binds
    }

    #[inline]
    pub fn get_framebuffer_binds(&self) -> &[FramebufferId] {
        &self.framebuffer_read_binds
    }
}

pub struct ResolvedSamplers<'a, const COUNT: usize> {
    textures_resolved: ArrayVec<&'a Texture, COUNT>,
    framebuffer_read_resolved: ArrayVec<&'a Framebuffer, COUNT>,
}

impl<'a, const COUNT: usize> ResolvedSamplers<'a, COUNT> {
    pub fn new(
        fbs: ArrayVec<&'a Framebuffer, COUNT>,
        textures: ArrayVec<&'a Texture, COUNT>,
    ) -> Self {
        Self {
            framebuffer_read_resolved: fbs,
            textures_resolved: textures,
        }
    }

    #[inline]
    pub fn sample_texture(&self, id: TextureUnit, u: f32, v: f32) -> Vec4 {
        self.textures_resolved[id.0].sample(u, v)
    }

    #[inline]
    pub fn sample_texture_fail_silent(&self, id: TextureUnit, u: f32, v: f32) -> Vec4 {
        self.textures_resolved[id.0].sample_fail_silent(u, v)
    }
}

pub struct RequestedWriters<const COUNT: usize> {
    framebuffer_write_binds: ArrayVec<FramebufferId, COUNT>,
}

impl<const COUNT: usize> RequestedWriters<COUNT> {
    pub fn new() -> Self {
        Self {
            framebuffer_write_binds: ArrayVec::new(),
        }
    }

    #[inline]
    pub fn bind_framebuffers_write(
        &mut self,
        id: FramebufferId,
    ) -> Result<FramebufferUnit, &'static str> {
        self.framebuffer_write_binds
            .try_push(id)
            .map_err(|_| "No free binds left")?;
        Ok(FramebufferUnit(self.framebuffer_write_binds.len() - 1))
    }

    #[inline]
    pub fn get_framebuffer_binds(&self) -> &[FramebufferId] {
        &self.framebuffer_write_binds
    }
}

pub struct ResolvedWriters<const COUNT: usize> {
    framebuffer_write_resolved: ArrayVec<Framebuffer, COUNT>,
}

impl<const COUNT: usize> ResolvedWriters<COUNT> {
    pub fn new(fbs: ArrayVec<Framebuffer, COUNT>) -> Self {
        Self {
            framebuffer_write_resolved: fbs,
        }
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut [Framebuffer] {
        &mut self.framebuffer_write_resolved
    }

    #[inline]
    pub fn into_inner(self) -> ArrayVec<Framebuffer, COUNT> {
        self.framebuffer_write_resolved
    }
}
