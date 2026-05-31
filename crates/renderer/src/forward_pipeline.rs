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
use glam::{IVec2, Vec2, Vec3, Vec4};
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
        let screen_width = framebuffer.width();
        let screen_height = framebuffer.height();

        let triangles = mesh
            .positions
            .chunks_exact(3)
            .zip(mesh.data.chunks_exact(3))
            .map(|(pos, data)| {
                let v0 = (self.vertex_shader)(Vertex::new(pos[0], data[0]));
                let v1 = (self.vertex_shader)(Vertex::new(pos[1], data[1]));
                let v2 = (self.vertex_shader)(Vertex::new(pos[2], data[2]));
                Triangle {
                    screen_space: [
                        Self::to_screen_space(v0.position, screen_width, screen_height),
                        Self::to_screen_space(v1.position, screen_width, screen_height),
                        Self::to_screen_space(v2.position, screen_width, screen_height),
                    ],
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
        let cols = screen_width.div_ceil(TILE_SIZE.0);
        let rows = screen_height.div_ceil(TILE_SIZE.1);
        let fb_ptr = (&mut framebuffer as *mut Framebuffer) as usize;

        let tiles = (0..rows)
            .flat_map(|row| {
                (0..cols).map(move |col| {
                    let x = col * TILE_SIZE.0;
                    let y = row * TILE_SIZE.1;
                    Tile {
                        x,
                        y,
                        width: TILE_SIZE.0.min(screen_width - x),
                        height: TILE_SIZE.1.min(screen_height - y),
                    }
                })
            })
            .collect::<Vec<_>>();
        let fragment_shader = &self.fragment_shader;
        let bins = self.bin_triangles(&triangles, &tiles);
        bins.iter().for_each(move |bin| {
            // let mut frags = vec![];
            for &index in &bin.indices {
                Rasterizer::rasterize(&triangles[index], |fragment| {
                    let frag_color = fragment_shader(fragment);
                    let fb_ptr = fb_ptr as *mut Framebuffer; // Safety: This is whack
                    unsafe {
                        (*fb_ptr).write_fragment(
                            fragment.position.x as usize,
                            fragment.position.y as usize,
                            fragment.depth,
                            frag_color,
                        )
                    };
                    // frags.push((fragment.position, frag_color));
                });
            }
            // frags
        });
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
        renderer.put_framebuffer(render_buffer, framebuffer);
    }

    #[inline(always)]
    pub(crate) fn to_screen_space(position: Vec3, width: usize, height: usize) -> Vec3 {
        let x = (position.x + 1.0) * 0.5 * width as f32;
        let y = (1.0 - (position.y + 1.0) * 0.5) * height as f32;
        Vec3::new(x, y, position.z)
    }

    fn bin_triangles(
        &self,
        triangles: &[Triangle<T>],
        tiles: &[Tile],
    ) -> Vec<TileBin> {
        let mut binned: Vec<TileBin> = tiles
            .iter()
            .map(|t| TileBin {
                tile: *t,
                indices: Vec::new(),
            })
            .collect();
        for (i, triangle) in triangles.iter().enumerate() {
            let Vec3 { x: ax, y: ay, .. } = triangle.screen_space[0];
            let Vec3 { x: bx, y: by, .. } = triangle.screen_space[1];
            let Vec3 { x: cx, y: cy, .. } = triangle.screen_space[2];

            let bb_min_x = f32::min(f32::min(ax, bx), cx); // bounding box for the triangle
            let bb_min_y = f32::min(f32::min(ay, by), cy); // defined by its top left and bottom right corners
            let bb_max_x = f32::max(f32::max(ax, bx), cx);
            let bb_max_y = f32::max(f32::max(ay, by), cy);
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
