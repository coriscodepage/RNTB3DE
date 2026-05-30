use std::marker::PhantomData;

use crate::{
    datatypes::{FragmentInput, Triangle, Vertex},
    lerp::Lerp,
    mesh::Mesh,
    rasterizer::Rasterizer,
    renderer::Renderer,
};
use glam::Vec4;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct PipelineForward<T, VS, FS> {
    vertex_shader: VS,
    fragment_shader: FS,
    render_buffer: Option<usize>,
    _marker: PhantomData<T>,
}

impl<T, VS, FS> PipelineForward<T, VS, FS>
where
    T: Lerp + Copy + Debug,
    VS: Fn(Vertex<T>) -> Vertex<T>,
    FS: Fn(FragmentInput<T>) -> Vec4,
{
    pub fn new(vertex_shader: VS, fragment_shader: FS) -> Self {
        Self {
            vertex_shader,
            fragment_shader,
            render_buffer: None,
            _marker: PhantomData,
        }
    }

    pub fn attach_render_buffer(&mut self, buffer_id: usize) {
        self.render_buffer = Some(buffer_id);
    }

    pub fn detach_render_buffer(&mut self) {
        self.render_buffer = None;
    }

    pub fn assemble_and_run(&self, renderer: &mut Renderer, mesh: &Mesh<T>) {
        let Some(render_buffer) = self.render_buffer else {
            panic!("No render buffer attached to the pipeline.");
        };

        let mut framebuffer = renderer.take_framebuffer(render_buffer);
        let triangles = mesh
            .positions
            .chunks_exact(3)
            .zip(mesh.data.chunks_exact(3))
            .map(|(pos, data)| {
                let v0 = (self.vertex_shader)(Vertex::new(pos[0], data[0]));
                let v1 = (self.vertex_shader)(Vertex::new(pos[1], data[1]));
                let v2 = (self.vertex_shader)(Vertex::new(pos[2], data[2]));
                Triangle { position: [v0.position, v1.position, v2.position], data: [v0.data, v1.data, v2.data] }
            });
        for triangle in triangles {
            let fragments =
                Rasterizer::rasterize(&triangle, framebuffer.width(), framebuffer.height());
            for fragment in fragments {
                let frag_color = (self.fragment_shader)(fragment);
                framebuffer.write_fragment(
                    fragment.position.x as usize,
                    fragment.position.y as usize,
                    0.5, // dummy depth value
                    frag_color,
                );
            }
        }
        renderer.put_framebuffer(render_buffer, framebuffer);
    }
}
