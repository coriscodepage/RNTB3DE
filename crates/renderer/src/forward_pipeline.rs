use std::{
    cmp::{max, min},
    marker::PhantomData,
};

use crate::{
    datatypes::{FragmentInput, Triangle, Vertex},
    framebuffer::{Framebuffer, TILE_SIZE, Tile, TileBin},
    lerp::Lerp,
    mesh::Mesh,
    rasterizer::Rasterizer,
    renderer::Renderer,
};
use glam::{IVec2, Vec4};
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
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
    T: Lerp + Copy + Debug + Send + Sync,
    VS: Fn(Vertex<T>) -> Vertex<T> + Send + Sync,
    FS: Fn(FragmentInput<T>) -> Vec4 + Send + Sync,
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
                Triangle {
                    position: [v0.position, v1.position, v2.position],
                    data: [v0.data, v1.data, v2.data],
                }
            })
            .collect::<Vec<_>>();
        // for triangle in triangles {
        //     // let fragments =
        //     Rasterizer::rasterize(
        //         &triangle,
        //         framebuffer.width(),
        //         framebuffer.height(),
        //         |fragment| {
        //             let frag_color = (self.fragment_shader)(fragment);
        //             unsafe {
        //                 framebuffer.write_fragment(
        //                     fragment.position.x as usize,
        //                     fragment.position.y as usize,
        //                     0.5, // dummy depth value
        //                     frag_color,
        //                 )
        //             };
        //         },
        //     );
        // }
        let screen_width = framebuffer.width();
        let screen_height = framebuffer.height();
        let cols = screen_width.div_ceil(TILE_SIZE.0);
        let rows = screen_height.div_ceil(TILE_SIZE.1);
        let fb_ptr = (&mut framebuffer as *mut Framebuffer) as usize;

        // let tiles = (0..rows)
        //     .flat_map(|row| {
        //         (0..cols).map(move |col| {
        //             let x = col * TILE_SIZE.0;
        //             let y = row * TILE_SIZE.1;
        //             Tile {
        //                 x,
        //                 y,
        //                 width: TILE_SIZE.0.min(screen_width - x),
        //                 height: TILE_SIZE.1.min(screen_height - y),
        //             }
        //         })
        //     })
        //     .collect::<Vec<_>>();
        // let fragment_shader = &self.fragment_shader;
        // let bins = self.bin_triangles(&triangles, &tiles, screen_width, screen_height);
        // bins.iter().for_each(move |bin| {
        //     // let mut frags = vec![];
        //     for &index in &bin.indices {
        //         Rasterizer::rasterize(&triangles[index], screen_width, screen_height, &bin.tile, |fragment| {
        //             let frag_color = fragment_shader(fragment);
        //             let fb_ptr = fb_ptr as *mut Framebuffer; // Safety: This is whack
        //             unsafe {
        //                 (*fb_ptr).write_fragment(
        //                     fragment.position.x as usize,
        //                     fragment.position.y as usize,
        //                     fragment.depth,
        //                     frag_color,
        //                 )
        //             };
        //             // frags.push((fragment.position, frag_color));
        //         });
        //     }
        //     // frags
        // });
        // .flatten()
        // .collect::<Vec<_>>();
        // for (pos, col) in rendered {
        //     unsafe {
        //         framebuffer.write_fragment(
        //             pos.x as usize,
        //             pos.y as usize,
        //             0.5, // dummy depth value
        //             col,
        //         )
        //     };
        // }
        for triangle in &triangles {
            Rasterizer::rasterize(triangle, screen_width, screen_height, |fragment| {
                let frag_color = (self.fragment_shader)(fragment);
                unsafe {
                    framebuffer.write_fragment(
                        fragment.position.x as usize,
                        fragment.position.y as usize,
                        0.5, // dummy depth value
                        frag_color,
                    )
                };
            });
        }
        renderer.put_framebuffer(render_buffer, framebuffer);
    }

    fn bin_triangles(
        &self,
        triangles: &[Triangle<T>],
        tiles: &[Tile],
        screen_width: usize,
        screen_height: usize,
    ) -> Vec<TileBin> {
        let mut binned: Vec<TileBin> = tiles
            .iter()
            .map(|t| TileBin {
                tile: *t,
                indices: Vec::new(),
            })
            .collect();
        for (i, triangle) in triangles.iter().enumerate() {
            let IVec2 { x: ax, y: ay } =
                Rasterizer::to_screen_space(triangle.position[0], screen_width, screen_height);
            let IVec2 { x: bx, y: by } =
                Rasterizer::to_screen_space(triangle.position[1], screen_width, screen_height);
            let IVec2 { x: cx, y: cy } =
                Rasterizer::to_screen_space(triangle.position[2], screen_width, screen_height);

            let bb_min_x = min(min(ax, bx), cx);
            let bb_min_y = min(min(ay, by), cy);
            let bb_max_x = max(max(ax, bx), cx);
            let bb_max_y = max(max(ay, by), cy);
            for bin in binned.iter_mut() {
                let tile_min = bin.tile.min();
                let tile_max = bin.tile.max();
                if tile_max.x < bb_min_x
                    || tile_min.x > bb_max_x
                    || tile_max.y < bb_min_y
                    || tile_min.y > bb_max_y
                {
                    continue;
                }

                bin.indices.push(i);
            }
        }
        binned
    }
}
