use std::ops::{Deref, DerefMut};

use arrayvec::ArrayVec;

use renderer::{
    abstraction::context::{
        FramebufferUnit, RequestedWriters, ResolvedSamplers, ResolvedWriters, TextureUnit,
    },
    framebuffer_storage::{FramebufferId, FramebufferStore},
};

use crate::samplers::texture::{TextureSrc, TextureStorage};

#[derive(Debug, Clone)]
pub struct RequestedSamplers<const COUNT: usize> {
    texture_binds: ArrayVec<TextureSrc, COUNT>,
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
    pub fn bind_texture(&mut self, source: TextureSrc) -> Result<TextureUnit, &'static str> {
        self.texture_binds
            .try_push(source)
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
    pub fn get_texture_binds(&self) -> &[TextureSrc] {
        &self.texture_binds
    }

    #[inline]
    pub fn get_framebuffer_binds(&self) -> &[FramebufferId] {
        &self.framebuffer_read_binds
    }
}

pub struct RequestedWritersGuard<'a, const COUNT: usize> {
    fb_store: &'a mut FramebufferStore,
    writers_req: &'a RequestedWriters<COUNT>,
    writers_res: Option<ResolvedWriters<COUNT>>,
}

impl<'a, const COUNT: usize> Deref for RequestedWritersGuard<'a, COUNT> {
    type Target = ResolvedWriters<COUNT>;

    fn deref(&self) -> &Self::Target {
        self.writers_res.as_ref().unwrap()
    }
}

impl<'a, const COUNT: usize> DerefMut for RequestedWritersGuard<'a, COUNT> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.writers_res.as_mut().unwrap()
    }
}

impl<'a, const COUNT: usize> Drop for RequestedWritersGuard<'a, COUNT> {
    fn drop(&mut self) {
        release(
            self.fb_store,
            self.writers_req,
            self.writers_res.take().unwrap(),
        );
    }
}

pub fn with_acquired<
    const COUNT: usize,
    F: FnOnce(&ResolvedSamplers<COUNT>, &mut ResolvedWriters<COUNT>) -> R,
    R,
>(
    tex_store: &mut TextureStorage,
    fb_store: &mut FramebufferStore,
    sampler_request: &RequestedSamplers<COUNT>,
    writer_request: &RequestedWriters<COUNT>,
    f: F,
) -> R {
    let writer_binds = writer_request.get_framebuffer_binds();
    let sampler_fb_binds = sampler_request.get_framebuffer_binds();
    let sampler_texture_binds = sampler_request.get_texture_binds();

    if sampler_fb_binds.iter().any(|x| writer_binds.contains(x)) {
        panic!("Can't both read and sample from the same framebuffer");
    }
    let write_fbs = writer_binds
        .iter()
        .map(|&fb_req| fb_store.take_framebuffer(fb_req))
        .collect::<ArrayVec<_, COUNT>>();
    let read_fbs = sampler_fb_binds
        .iter()
        .map(|&fb_req| fb_store.borrow_framebuffer(fb_req))
        .collect::<ArrayVec<_, COUNT>>();
    sampler_texture_binds
        .iter()
        .for_each(|tex_req| tex_store.ensure_loaded(tex_req));
    let read_textures = sampler_texture_binds
        .iter()
        .map(|tex_req| tex_store.get(&tex_req))
        .collect::<ArrayVec<_, COUNT>>();
    let samplers_resolved = ResolvedSamplers::new(read_fbs, read_textures);
    let mut writers_resolved = ResolvedWriters::new(write_fbs);
    let result = f(&samplers_resolved, &mut writers_resolved);
    drop(samplers_resolved);
    release(fb_store, writer_request, writers_resolved);
    result
}

fn release<const COUNT: usize>(
    fb_store: &mut FramebufferStore,
    writer_request: &RequestedWriters<COUNT>,
    resolved_writers: ResolvedWriters<COUNT>,
) {
    resolved_writers
        .into_inner()
        .into_iter()
        .zip(writer_request.get_framebuffer_binds())
        .for_each(|(fb, &id)| fb_store.put_framebuffer(id, fb));
}
