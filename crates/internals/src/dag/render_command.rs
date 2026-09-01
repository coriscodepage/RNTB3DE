use std::sync::mpsc::{Receiver, Sender};

use renderer::{
    abstraction::program::{AnyProgram, ProgramHandle, ProgramStorage},
    framebuffer_storage::{FramebufferId, FramebufferStore},
    lerp::Lerp,
};

use crate::{
    samplers::texture::{TextureSrc, TextureStorage},
    world::material::{Material, MaterialHandle, MaterialStorage},
};
use std::fmt::Debug;

pub enum RenderCommand<T, D, N> {
    CreateFramebuffer(usize, (usize, usize)),
    RemoveFramebuffer(FramebufferId),
    CreateTexture(TextureSrc),
    RemoveTexture(TextureSrc),
    CreateMaterial(usize, Material),
    CreateProgram(usize, Box<dyn AnyProgram<T, D, N>>),
}

pub struct Renderer<T, D, N> {
    pub framebuffer_store: FramebufferStore,
    pub texture_store: TextureStorage,
    pub material_store: MaterialStorage,
    pub program_store: ProgramStorage<T, D, N>,
    rx: Receiver<RenderCommand<T, D, N>>,
}

impl<T, D, N> Renderer<T, D, N>
where
    T: Lerp + Copy + Debug + Send + Sync,
    D: Send + Sync,
    N: Send + Sync,
{
    pub fn new(rx: Receiver<RenderCommand<T, D, N>>) -> Self {
        Self {
            framebuffer_store: FramebufferStore::new(),
            texture_store: TextureStorage::new(),
            rx,
            material_store: MaterialStorage::new(),
            program_store: ProgramStorage::new(),
        }
    }

    pub fn poll_commands(&mut self) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                RenderCommand::CreateFramebuffer(id, (width, height)) => {
                    let actual_id = self.framebuffer_store.create_framebuffer(width, height);
                    debug_assert_ne!(id, actual_id.index);
                }
                RenderCommand::RemoveFramebuffer(framebuffer_id) => todo!(),
                RenderCommand::CreateTexture(texture_src) => self.texture_store.add(texture_src),
                RenderCommand::RemoveTexture(texture_src) => todo!(),
                RenderCommand::CreateMaterial(id, material) => {
                    let actual_id = self.material_store.put(material);
                    debug_assert_ne!(id, actual_id.0);
                }
                RenderCommand::CreateProgram(id, any_program) => {
                    let actual_id = self.program_store.put_boxed(any_program);
                    debug_assert_ne!(id, actual_id.0);
                }
            }
        }
    }
}

pub struct RendererHandle<T, D, N> {
    tx: Sender<RenderCommand<T, D, N>>,
    next_fb_id: usize,
    next_program_id: usize,
    next_material_id: usize,
}

impl<T, D, N> RendererHandle<T, D, N>
where
    T: Lerp + Copy + Debug + Send + Sync,
    D: Send + Sync,
    N: Send + Sync,
{
    pub fn new(tx: Sender<RenderCommand<T, D, N>>) -> Self {
        Self {
            tx,
            next_fb_id: 0,
            next_program_id: 0,
            next_material_id: 0,
        }
    }

    pub fn create_framebuffer(&mut self, width: usize, height: usize) -> FramebufferId {
        let current = self.next_fb_id;
        self.tx
            .send(RenderCommand::CreateFramebuffer(current, (width, height)))
            .unwrap();
        self.next_fb_id += 1;
        FramebufferId {
            index: current,
            generation: 0,
        }
    }

    pub fn create_texture(&self, source: TextureSrc) {
        self.tx.send(RenderCommand::CreateTexture(source)).unwrap();
    }

    pub fn create_program<P: AnyProgram<T, D, N> + 'static>(
        &mut self,
        program: P,
    ) -> ProgramHandle {
        let current = self.next_program_id;
        self.tx
            .send(RenderCommand::CreateProgram(current, Box::new(program)))
            .unwrap();
        self.next_program_id += 1;
        ProgramHandle(current)
    }

    pub fn create_material(&mut self, material: Material) -> MaterialHandle {
        let current = self.next_material_id;

        self.tx
            .send(RenderCommand::CreateMaterial(current, material))
            .unwrap();
        self.next_material_id += 1;
        MaterialHandle(current)
    }
}
