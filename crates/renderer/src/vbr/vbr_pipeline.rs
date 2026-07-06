use std::{collections::HashMap, marker::PhantomData};

use glam::{Vec2, Vec2Swizzles, Vec4};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    datatypes::{DrawTri, FragmentInput, Triangle, Vertex},
    forward_pipeline::PipelineForward,
    framebuffer::{Framebuffer, Tile, TileBin},
    lerp::Lerp,
    mesh::Mesh,
    rasterizer::{self, Rasterizer},
    renderer::Renderer,
};
use std::fmt::Debug;

pub struct VbrPipeline<'a, T: Lerp + Send + Sync> {
    v_buffer: Vec<DrawTri>,
    depth_buffer: Option<usize>,
    width: i32,
    height: i32,
    triangles: HashMap<DrawTri, Triangle<T>>,
    render_buffer: Vec<usize>,
    bins: Vec<TileBin<'a>>,
    _marker: PhantomData<T>,
}

impl<'a, T> VbrPipeline<'a, T>
where
    T: Lerp + Copy + Debug + Send + Sync,
{
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            v_buffer: vec![DrawTri::default(); width as usize * height as usize],
            depth_buffer: None,
            width,
            height,
            triangles: HashMap::new(),
            render_buffer: Vec::new(),
            bins: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn run_pass<P, F>(
        &mut self,
        draw_id: u32,
        renderer: &mut Renderer,
        mesh: &Mesh<T>,
        pre: P,
        shaders: &[F],
    ) where
        P: Fn(Vertex<T>) -> Vertex<T> + Send + Sync,
        F: Fn(&DrawTri, &Triangle<T>) -> Vec4 + Send + Sync + Clone,
    {
        if self.render_buffer.is_empty() {
            panic!("No render buffer attached to the pipeline.");
        };

        if self.render_buffer.len() != shaders.len() {
            panic!("Render buffer count does not match shader count.");
        }

        let Some(depth_buffer_id) = self.depth_buffer else {
            panic!("Depth buffer not bound.");
        };

        let mut framebuffers = self
            .render_buffer
            .iter()
            .map(|&b| renderer.take_framebuffer(b))
            .collect::<Vec<_>>();

        let mut depth_buffer = renderer.take_framebuffer(depth_buffer_id);
        let screen_width = framebuffers.iter().map(|b| b.width()).min().unwrap_or(0);
        let screen_height = framebuffers.iter().map(|b| b.height()).min().unwrap_or(0);

        if (screen_width, screen_height) != (self.width, self.height) {
            panic!("Render buffer size does not match V-Buffer size");
        }

        if (depth_buffer.width(), depth_buffer.height()) != (self.width, self.height) {
            panic!("Depth buffer size does not match V-Buffer size");
        }

        mesh.positions
            .chunks_exact(3)
            .zip(mesh.data.chunks_exact(3))
            .zip(0..mesh.positions.len() as u32 / 3)
            .for_each(|((position, data), tri_id)| {
                let v0 = pre(Vertex::new(position[0], data[0]));
                let v1 = pre(Vertex::new(position[1], data[1]));
                let v2 = pre(Vertex::new(position[2], data[2]));
                let triangle = Triangle {
                    position: [
                        PipelineForward::to_screen_space(v0.position, screen_width, screen_height).unwrap(), // TODO: New degenerate triangle impl!
                        PipelineForward::to_screen_space(v1.position, screen_width, screen_height).unwrap(), // TODO: New degenerate triangle impl!
                        PipelineForward::to_screen_space(v2.position, screen_width, screen_height).unwrap(), // TODO: New degenerate triangle impl!
                    ],
                    depth: [
                        Vec2::new(v0.position.z, 1.0),
                        Vec2::new(v1.position.z, 1.0),
                        Vec2::new(v2.position.z, 1.0),
                    ],
                    data: [v0.data, v1.data, v2.data],
                };
                let call = DrawTri { draw_id, tri_id };
                self.triangles.insert(call, triangle);
                Rasterizer::rasterize(
                    &triangle,
                    &Tile {
                        width: screen_width,
                        height: screen_height,
                        x: 0,
                        y: 0,
                    },
                    |fragment| {
                        let position =
                            (fragment.position.x + fragment.position.y * screen_width) as usize;
                        if depth_buffer.depth_test(
                            fragment.position.x,
                            fragment.position.y,
                            fragment.depth,
                        ) {
                            self.v_buffer[position] = call;
                            unsafe {
                                depth_buffer.write_fragment(
                                    fragment.position.x,
                                    fragment.position.y,
                                    fragment.depth,
                                    Vec4::default(),
                                )
                            };
                        }
                    },
                );
            });

        for (i, framebuffer) in framebuffers.iter_mut().enumerate() {
            for (j, draw) in self.v_buffer.iter().enumerate() {
                let color = shaders[i](draw, self.triangles.get(&draw).unwrap());
                unsafe {
                    framebuffer.write_fragment(
                        j as i32 % screen_width,
                        j as i32 / screen_width,
                        0.0,
                        color,
                    )
                };
            }
        }

        for (&id, fb) in self.render_buffer.iter().zip(framebuffers) {
            renderer.put_framebuffer(id, fb);
        }
        renderer.put_framebuffer(depth_buffer_id, depth_buffer);
    }

    pub fn run_pixel<P>(&mut self, renderer: &mut Renderer, shader: P)
    where
        P: Fn((i32, i32)) -> Vec4 + Send + Sync,
    {
        if self.render_buffer.is_empty() {
            panic!("No render buffer attached to the pipeline.");
        };

        for &render_buffer in self.render_buffer.iter() {
            let mut framebuffer = renderer.take_framebuffer(render_buffer);
            let fb_ptr = &mut framebuffer as *mut Framebuffer as usize;
            framebuffer.get_tiles().tiles.par_iter().for_each(|tile| {
                let min_t = tile.min();
                let max_t = tile.max();
                for x in min_t.x..max_t.x {
                    for y in min_t.y..max_t.y {
                        let frag_color = shader((x, y));
                        unsafe {
                            (*(fb_ptr as *mut Framebuffer)).write_fragment(x, y, 1.0, frag_color)
                        };
                    }
                }
            });
            renderer.put_framebuffer(render_buffer, framebuffer);
        }
    }

    pub fn resize_if_needed(&mut self, width: i32, height: i32) {
        if (self.width, self.height) != (width, height) {
            (self.width, self.height) = (width, height);
            self.v_buffer = vec![DrawTri::default(); width as usize * height as usize];
        }
    }
}
