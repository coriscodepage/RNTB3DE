use std::{mem, sync::{
    Mutex,
    atomic::{
        AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
    },
}};

use renderer::framebuffer::Framebuffer;

#[derive(Debug)]
pub struct PresentationBuffer {
    buffers: [Mutex<Framebuffer>; 2],
    latest: AtomicUsize,
}

impl PresentationBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffers: [
                Mutex::new(Framebuffer::new(width, height)),
                Mutex::new(Framebuffer::new(width, height)),
            ],
            latest: AtomicUsize::new(0),
        }
    }

    pub fn read<F: FnMut(&Framebuffer)>(&self, mut callback: F) {
        let index = self.latest.load(Acquire);
        let buffer = self.buffers[index].lock().unwrap();
        callback(&buffer);
    }

    pub fn write(&self, fb: &mut Framebuffer) {
        // TODO: Check the sizes of the fbs. Maybe do that in the Framebuffer struct proper?
        let index = 1 - self.latest.load(Relaxed);
        let mut buffer = self.buffers[index].lock().unwrap();
        mem::swap(&mut *buffer, fb);
        self.latest.store(index, Release);
    }
}
