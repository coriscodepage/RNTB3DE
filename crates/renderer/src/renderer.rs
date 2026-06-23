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
            let current_gen = buffer.current_generation();
            let mut i = 0;
            while i < ra.len() {
                if unsafe { *generation.get_unchecked(i) } == current_gen {
                    let r: i32 = unsafe { (ra[i] * 255.0).to_int_unchecked() };
                    let g: i32 = unsafe { (ga[i] * 255.0).to_int_unchecked() };
                    let b: i32 = unsafe { (ba[i] * 255.0).to_int_unchecked() };
                    let combined = ((r as i32) << 16) + ((g as i32) << 8) + (b as i32);
                    unsafe { *out.get_unchecked_mut(i) = combined };
                } else {
                    unsafe { *out.get_unchecked_mut(i) = 0 };
                }
                i += 1;
            }
            // const CHUNK: usize = 4096;
            // out.par_chunks_mut(CHUNK)
            //     .zip(ra.par_chunks(CHUNK))
            //     .zip(ga.par_chunks(CHUNK))
            //     .zip(ba.par_chunks(CHUNK))
            //     .zip(generation.par_chunks(CHUNK))
            //     .for_each(|((((out_c, ra_c), ga_c), ba_c), gen_c)| {
            //         for i in 0..out_c.len() {
            //             unsafe {
            //                 if *gen_c.get_unchecked(i) == current_gen {
            //                     let r: i32 = (ra_c[i] * 255.0).to_int_unchecked();
            //                     let g: i32 = (ga_c[i] * 255.0).to_int_unchecked();
            //                     let b: i32 = (ba_c[i] * 255.0).to_int_unchecked();
            //                     *out_c.get_unchecked_mut(i) = (r << 16) | (g << 8) | b;
            //                 } else {
            //                     *out_c.get_unchecked_mut(i) = 0;
            //                 }
            //             }
            //         }
            //     });
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
