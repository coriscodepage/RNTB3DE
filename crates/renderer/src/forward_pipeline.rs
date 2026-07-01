use std::{
    cmp::{max, min},
    marker::PhantomData,
    mem::MaybeUninit,
};

use crate::{
    abstraction::{context::Context, program::Program}, datatypes::{FragmentInput, Triangle, Vertex}, framebuffer::{Framebuffer, MAX_BINDS, TILE_SIZE, Tile, TileBin, TilesInfo}, lerp::Lerp, mesh::Mesh, rasterizer::Rasterizer, renderer::{Renderer, morton},
};
use bumpalo::Bump;
use glam::{IVec2, Vec2, Vec3, Vec4};
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
        IntoParallelRefMutIterator, ParallelExtend, ParallelIterator,
    },
    slice::ParallelSlice,
};
use smallvec::SmallVec;
use std::fmt::Debug;

#[derive(Debug)]
pub struct PipelineForward {
    bins: Vec<TileBin>,
    arena: Bump,
}

impl PipelineForward {
    pub fn new() -> Self {
        Self {
            bins: Vec::with_capacity(100),
            arena: Bump::with_capacity(1024),
        }
    }

    pub fn assemble_and_run<T, VS, FS>(
        &mut self,
        context: &mut Context<MAX_BINDS>,
        program: &Program<T, VS, FS>,
        mesh: &Mesh<T>,
    ) where
        T: Lerp + Copy + Debug + Send + Sync,
        VS: Fn(Vertex<T>) -> Vertex<T> + Send + Sync,
        FS: Fn(&FragmentInput<T>, &mut Context<MAX_BINDS>) -> Vec4 + Send + Sync + Clone,
    {
        
        if context.framebuffer_write_count() != program.fragment().len() {
            panic!("Render buffer count does not match shader count.");
        }

        self.arena.reset();
        context.resolve();

        let framebuffers = &mut context.framebuffers_write_resolved;
        let screen_width = framebuffers
            .iter()
            .map(|b| b.width())
            .min()
            .unwrap_or(0);
        let screen_height = framebuffers
            .iter()
            .map(|b| b.height())
            .min()
            .unwrap_or(0);

        let vertex_shader = program.vertex();
        let triangles: &mut [MaybeUninit<Triangle<T>>] = self
            .arena
            .alloc_slice_fill_clone(mesh.positions.len() / 3, &MaybeUninit::uninit());
        triangles
            .par_iter_mut()
            .zip(mesh.positions.par_chunks_exact(3))
            .zip(mesh.data.par_chunks_exact(3))
            .for_each(|((tri, pos), data)| {
                let v0 = vertex_shader(Vertex::new(pos[0], data[0]));
                let v1 = vertex_shader(Vertex::new(pos[1], data[1]));
                let v2 = vertex_shader(Vertex::new(pos[2], data[2]));
                tri.write(Triangle {
                    position: [
                        Self::to_screen_space(v0.position, screen_width, screen_height),
                        Self::to_screen_space(v1.position, screen_width, screen_height),
                        Self::to_screen_space(v2.position, screen_width, screen_height),
                    ],
                    depth: [
                        Vec2::new(v0.position.z, 1.0),
                        Vec2::new(v1.position.z, 1.0),
                        Vec2::new(v2.position.z, 1.0),
                    ],
                    data: [v0.data, v1.data, v2.data],
                });
            });

        let triangles: &mut [Triangle<T>] = unsafe {
            std::slice::from_raw_parts_mut(
                triangles.as_mut_ptr() as *mut Triangle<T>,
                triangles.len(),
            )
        };

        Self::bin_triangles(
            &mut self.bins,
            framebuffers
                .iter()
                .find(|fb| fb.width() == screen_width && fb.height() == screen_height)
                .unwrap()
                .get_tiles(),
            triangles,
        );

        let fb_ptr = self.arena.alloc_slice_fill_iter(
            framebuffers
                .iter_mut()
                .map(|fb| (&mut *fb as *mut Framebuffer) as usize),
        );
        let ctx_ptr = context as *mut Context<MAX_BINDS> as usize;
        let fragment_shader = program.fragment();
        self.bins.par_iter().for_each(|bin| {
            for &index in &bin.indices {
                Rasterizer::rasterize(&triangles[index], &bin.tile, |fragment| {
                    for (i, &fb_ptr) in fb_ptr.iter().enumerate() {
                        if unsafe {
                            (*(fb_ptr as *mut Framebuffer)).depth_test(
                                fragment.position.x,
                                fragment.position.y,
                                fragment.depth,
                            )
                        } {
                            let frag_color = fragment_shader[i](&fragment, unsafe { &mut *(ctx_ptr as *mut Context<MAX_BINDS>) });
                            unsafe {
                                (*(fb_ptr as *mut Framebuffer)).write_fragment(
                                    fragment.position.x,
                                    fragment.position.y,
                                    fragment.depth,
                                    frag_color,
                                )
                            };
                        }
                    }
                });
            }
        });
        // for (&id, fb) in self.render_buffer.iter().zip(framebuffers) {
        //     renderer.put_framebuffer(id, fb);
        // }
        context.put_back();
    }

    pub fn bin_triangles<T: Lerp + Copy + Debug + Send + Sync>(
        bins: &mut Vec<TileBin>,
        tiles: &TilesInfo,
        triangles: &[Triangle<T>],
    ) {
        let TilesInfo { cols, rows, tiles } = tiles;
        if bins.len() < tiles.len() {
            bins.extend((bins.len()..tiles.len()).map(|i| TileBin {
                tile: tiles[i],
                indices: SmallVec::new(),
                morton_key: 0,
            }));
        }
        for i in 0..bins.len() {
            bins[i].tile = tiles[i];
            bins[i].indices.clear();
            bins[i].morton_key = morton(
                tiles[i].min().x as u32 / TILE_SIZE.0 as u32,
                tiles[i].min().y as u32 / TILE_SIZE.1 as u32,
            );
        }
        triangles.iter().enumerate().for_each(|(i, triangle)| {
            let [
                IVec2 { x: ax, y: ay },
                IVec2 { x: bx, y: by },
                IVec2 { x: cx, y: cy },
            ] = triangle.position;

            let bb_min_x = min(min(ax, bx), cx);
            let bb_min_y = min(min(ay, by), cy);
            let bb_max_x = max(max(ax, bx), cx);
            let bb_max_y = max(max(ay, by), cy);
            let total_area = Self::signed_triangle_area_wrong(ax, ay, bx, by, cx, cy);
            let (x_normal_a, y_normal_a) = (-(by - ay), bx - ax);
            let (x_normal_b, y_normal_b) = (-(cy - by), cx - bx);
            let (x_normal_c, y_normal_c) = (-(ay - cy), ax - cx);
            let cols_min = (bb_min_x as usize / TILE_SIZE.0).max(0);
            let cols_max = (bb_max_x as usize / TILE_SIZE.0).min(*cols - 1);
            let rows_min = (bb_min_y as usize / TILE_SIZE.1).max(0);
            let rows_max = (bb_max_y as usize / TILE_SIZE.1).min(*rows - 1);
            if total_area >= 2.0 {
                for row in rows_min..=rows_max {
                    for col in cols_min..=cols_max {
                        let bin = &mut bins[row * cols + col];
                        let tile_min = bin.tile.min();
                        let tile_max = bin.tile.max();
                        if tile_min.x <= bb_min_x
                            && tile_max.x >= bb_max_x
                            && tile_min.y <= bb_min_y
                            && tile_max.y >= bb_max_y
                        {
                            bin.indices.push(i);
                            continue;
                        }

                        let a = Self::furthest_point(tile_min, tile_max, x_normal_a, y_normal_a);
                        let b = Self::furthest_point(tile_min, tile_max, x_normal_b, y_normal_b);
                        let c = Self::furthest_point(tile_min, tile_max, x_normal_c, y_normal_c);
                        if Self::edge_function(triangle.position[0], triangle.position[1], a) < 0
                            || Self::edge_function(triangle.position[1], triangle.position[2], b)
                                < 0
                            || Self::edge_function(triangle.position[2], triangle.position[0], c)
                                < 0
                        {
                            continue;
                        }
                        bin.indices.push(i);
                    }
                }
            }
        });
        bins.retain(|b| !b.indices.is_empty());
        bins.sort_unstable_by_key(|v| v.morton_key);
    }

    pub fn run_pixel<P>(&mut self, context: &mut Context<MAX_BINDS>, shader: P)
    where
        P: Fn((i32, i32)) -> Vec4 + Send + Sync,
    {
        context.resolve();

        let render_buffers = &mut context.framebuffers_write_resolved;

        for framebuffer in render_buffers.iter_mut() {
            let fb_ptr = framebuffer as *mut Framebuffer as usize;
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
            // for y in 0..framebuffer.height() {
            //     for x in 0..framebuffer.width() {
            //         let frag_color = shader((x, y));
            //         unsafe { framebuffer.write_fragment(x, y, 1.0, frag_color) };
            //     }
            // }
        }
        context.put_back();
    }

    #[inline]
    fn furthest_point(tile_min: IVec2, tile_max: IVec2, x_normal: i32, y_normal: i32) -> IVec2 {
        IVec2::new(
            if x_normal >= 0 {
                tile_max.x
            } else {
                tile_min.x
            },
            if y_normal >= 0 {
                tile_max.y
            } else {
                tile_min.y
            },
        )
    }

    #[inline(always)]
    fn edge_function(a: IVec2, b: IVec2, c: IVec2) -> i32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }

    #[inline(always)]
    fn signed_triangle_area_wrong(ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> f32 {
        return ((by - ay) * (bx + ax) + (cy - by) * (cx + bx) + (ay - cy) * (ax + cx)) as f32;
    }

    #[inline(always)]
    pub(crate) fn to_screen_space(position: Vec3, width: i32, height: i32) -> IVec2 {
        let x = ((position.x + 1.0) * 0.5 * width as f32)
            .max(0.0)
            .min((width - 1) as f32);
        let y = ((1.0 - (position.y + 1.0) * 0.5) * height as f32)
            .max(0.0)
            .min((height - 1) as f32);
        IVec2::new(unsafe { x.to_int_unchecked() }, unsafe {
            y.to_int_unchecked()
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{datatypes::Vertex, mesh::Mesh, renderer::Renderer};
//     use glam::{Vec4, vec3, vec4};

//     #[test]
//     fn assemble_and_run_writes_the_rasterized_triangle() {
//         let mut renderer = Renderer::new();
//         let framebuffer_id = renderer.create_framebuffer(4, 4);
//         let program = Program::new(|vertex: Vertex<Vec4>| vertex, &[|fragment| fragment.data]);
//         let mut pipeline = PipelineForward::new();
//         pipeline.attach_render_buffer(framebuffer_id);

//         let mesh = Mesh::new(
//             vec![
//                 Vertex::new(vec3(-1.0, 1.0, 0.2), vec4(1.0, 0.0, 0.0, 1.0)),
//                 Vertex::new(vec3(1.0, 1.0, 0.4), vec4(0.0, 1.0, 0.0, 1.0)),
//                 Vertex::new(vec3(-1.0, -1.0, 0.6), vec4(0.0, 0.0, 1.0, 1.0)),
//             ],
//             None,
//         );

//         pipeline.assemble_and_run(&mut renderer, &program, &mesh);

//         let framebuffer = renderer.take_framebuffer(framebuffer_id);
//         assert_eq!(framebuffer.read_pixel(0, 0), vec4(1.0, 0.0, 0.0, 1.0));
//         assert!((framebuffer.read_depth(0, 0) - 0.2).abs() < 1e-6);
//     }

//     #[test]
//     fn run_pixel_writes_through_the_bound_framebuffer() {
//         let mut renderer = Renderer::new();
//         let framebuffer_id = renderer.create_framebuffer(2, 2);
//         let mut pipeline = <PipelineForward>::new();
//         pipeline.attach_render_buffer(framebuffer_id);

//         pipeline.run_pixel(&mut renderer, |(x, y)| vec4(x as f32, y as f32, 0.25, 1.0));

//         let framebuffer = renderer.take_framebuffer(framebuffer_id);
//         assert_eq!(framebuffer.read_pixel(0, 0), vec4(0.0, 0.0, 0.25, 1.0));
//         assert_eq!(framebuffer.read_pixel(1, 0), vec4(1.0, 0.0, 0.25, 1.0));
//         assert_eq!(framebuffer.read_pixel(0, 1), vec4(0.0, 1.0, 0.25, 1.0));
//         assert_eq!(framebuffer.read_pixel(1, 1), vec4(1.0, 1.0, 0.25, 1.0));
//     }
// }
