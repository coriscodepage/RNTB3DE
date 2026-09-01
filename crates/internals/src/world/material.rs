use renderer::abstraction::{
    context::RequestedWriters,
    program::{AnyProgram, ProgramHandle},
};

use crate::{imports::model::MeshData, samplers::context::RequestedSamplers};

pub type StdProgram = dyn AnyProgram<MeshData, f32, f32>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialHandle(pub usize);

#[derive(Debug, Clone)]
pub struct Material<const COUNT: usize = 16> {
    program: ProgramHandle,
    samplers: RequestedSamplers<COUNT>,
    writers: RequestedWriters<COUNT>,
}

impl<const COUNT: usize> Material<COUNT> {
    pub fn new(
        program: ProgramHandle,
        samplers: RequestedSamplers<COUNT>,
        writers: RequestedWriters<COUNT>,
    ) -> Self {
        Self {
            program,
            samplers,
            writers,
        }
    }

    pub fn get_handle(&self) -> ProgramHandle {
        self.program
    }

    pub fn get_samplers(&self) -> &RequestedSamplers<COUNT> {
        &self.samplers
    }

    pub fn get_writers(&self) -> &RequestedWriters<COUNT> {
        &self.writers
    }
}

pub struct MaterialStorage {
    store: Vec<Material>,
}

impl MaterialStorage {
    pub fn new() -> Self {
        Self { store: Vec::new() }
    }

    pub fn put(&mut self, material: Material) -> MaterialHandle {
        self.store.push(material);
        MaterialHandle(self.store.len() - 1)
    }

    pub fn get(&self, index: MaterialHandle) -> &Material {
        &self.store[index.0]
    }
}
