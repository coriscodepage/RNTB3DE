use crate::{
    abstraction::context::Context, datatypes::{FragmentInput, Vertex}, framebuffer::MAX_BINDS, lerp::Lerp,
};
use glam::Vec4;
use smallvec::SmallVec;
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Program<T, VS, FS> {
    vertex_shader: VS,
    fragment_shaders: SmallVec<[FS; 4]>, // TODO: Maybe ArrayVec?
    _marker: PhantomData<T>,
}

impl<T, VS, FS> Program<T, VS, FS>
where
    T: Lerp + Copy + Debug + Send + Sync,
    VS: Fn(Vertex<T>) -> Vertex<T> + Send + Sync,
    FS: Fn(&FragmentInput<T>, &mut Context<MAX_BINDS>) -> Vec4 + Send + Sync + Clone,
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
