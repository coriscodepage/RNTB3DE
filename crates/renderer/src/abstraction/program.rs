use crate::{
    abstraction::context::{ResolvedSamplers, ResolvedWriters},
    datatypes::{FragmentInput, Vertex, VertexHomogenous},
    forward_pipeline::PipelineForward,
    framebuffer::MAX_BINDS,
    lerp::Lerp,
    mesh::Mesh,
};
use glam::Vec4;
use smallvec::SmallVec;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProgramHandle(pub usize);

#[derive(Debug, Clone)]
pub struct Program<T, D, N, VS, FS> {
    vertex_shader: VS,
    fragment_shaders: SmallVec<[FS; 4]>, // TODO: Maybe ArrayVec?
    _marker: PhantomData<(T, D, N)>,
}

impl<T, D, N, VS, FS> Program<T, D, N, VS, FS>
where
    T: Lerp + Copy + Debug + Send + Sync,
    D: Send + Sync,
    N: Send + Sync,
    VS: Fn(Vertex<T>, &D) -> VertexHomogenous<T> + Send + Sync,
    FS: Fn(&FragmentInput<T>, &ResolvedSamplers<MAX_BINDS>, &N) -> Vec4 + Send + Sync + Clone,
{
    pub fn new(vertex_shader: VS, fragment_shaders: &[FS]) -> Self {
        let fragment_shaders = SmallVec::from(fragment_shaders);
        Self {
            vertex_shader,
            fragment_shaders,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn vertex(&self) -> &VS {
        &self.vertex_shader
    }

    #[inline]
    pub fn fragment(&self) -> &[FS] {
        &self.fragment_shaders
    }
}

pub trait AnyProgram<T, D, N>: Send + Sync
where
    T: Lerp + Copy + Debug + Send + Sync,
    D: Send + Sync,
    N: Send + Sync,
{
    fn run_forward(
        &self,
        pipeline: &mut PipelineForward,
        samplers: &ResolvedSamplers<MAX_BINDS>,
        writers: &mut ResolvedWriters<MAX_BINDS>,
        uniforms_vertex: D,
        uniforms_fragment: N,
        mesh: &Mesh<T>,
    );
}

impl<T, D, N, VS, FS> AnyProgram<T, D, N> for Program<T, D, N, VS, FS>
where
    T: Lerp + Copy + Debug + Send + Sync,
    D: Send + Sync,
    N: Send + Sync,
    VS: Fn(Vertex<T>, &D) -> VertexHomogenous<T> + Send + Sync,
    FS: Fn(&FragmentInput<T>, &ResolvedSamplers<MAX_BINDS>, &N) -> Vec4 + Send + Sync + Clone,
{
    fn run_forward(
        &self,
        pipeline: &mut PipelineForward,
        samplers: &ResolvedSamplers<MAX_BINDS>,
        writers: &mut ResolvedWriters<MAX_BINDS>,
        uniforms_vertex: D,
        uniforms_fragment: N,
        mesh: &Mesh<T>,
    ) {
        pipeline.assemble_and_run(
            samplers,
            writers,
            self,
            uniforms_vertex,
            uniforms_fragment,
            mesh,
        );
    }
}

pub struct ProgramStorage<T, D, N> {
    store: Vec<Box<dyn AnyProgram<T, D, N>>>,
}

impl<T, D, N> ProgramStorage<T, D, N>
where
    T: Lerp + Copy + Debug + Send + Sync,
    D: Send + Sync,
    N: Send + Sync,
{
    pub fn new() -> Self {
        Self { store: Vec::new() }
    }

    pub fn put<P: AnyProgram<T, D, N> + 'static>(&mut self, program: P) -> ProgramHandle {
        self.store.push(Box::new(program));
        ProgramHandle(self.store.len() - 1)
    }

    pub fn put_boxed(&mut self, program: Box<dyn AnyProgram<T, D, N>>) -> ProgramHandle {
        self.store.push(program);
        ProgramHandle(self.store.len() - 1)
    }

    pub fn get(&self, index: ProgramHandle) -> &dyn AnyProgram<T, D, N> {
        self.store[index.0].as_ref()
    }
}
