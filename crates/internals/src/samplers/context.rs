use std::ops::{Deref, DerefMut};

use arrayvec::ArrayVec;

use itertools::Itertools;
use renderer::{
    abstraction::context::{
        RequestedSamplers, RequestedWriters, ResolvedSamplers, ResolvedWriters,
    },
    renderer::{FramebufferStore, TextureStore},
};

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
    tex_store: &TextureStore,
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
    let read_textures = sampler_texture_binds
        .iter()
        .map(|&tex_req| tex_store.borrow_texture(tex_req))
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
