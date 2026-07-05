use std::{
    cmp::{max, min},
    default,
    mem::MaybeUninit,
};

use crate::{
    abstraction::{context::Context, program::Program},
    datatypes::{FragmentInput, Triangle, Vertex},
    framebuffer::{Framebuffer, MAX_BINDS, TILE_SIZE, TileBin, TilesInfo},
    lerp::Lerp,
    mesh::Mesh,
    rasterizer::Rasterizer,
    renderer::morton,
};
use bumpalo::{Bump, collections::CollectIn};
use glam::{IVec2, Vec2, Vec3, Vec4};
use itertools::Itertools;
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    slice::ParallelSlice,
};
use smallvec::SmallVec;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
struct Candidate<const COUNT: usize = 64> {
    count: usize,
    indeces: [(usize, usize); COUNT],
}

#[derive(Debug)]
pub struct PipelineForward {
    arena: Bump,
}

impl PipelineForward {
    pub fn new() -> Self {
        Self {
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
        let screen_width = framebuffers.iter().map(|b| b.width()).min().unwrap_or(0);
        let screen_height = framebuffers.iter().map(|b| b.height()).min().unwrap_or(0);

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

        let tiles = framebuffers
            .iter()
            .find(|fb| fb.width() == screen_width && fb.height() == screen_height)
            .unwrap()
            .get_tiles();

        let bins = Self::bin_triangles(&self.arena, tiles, triangles);

        let fb_ptr = self.arena.alloc_slice_fill_iter(
            framebuffers
                .iter_mut()
                .map(|fb| (&mut *fb as *mut Framebuffer) as usize),
        );
        let ctx_ptr = context as *mut Context<MAX_BINDS> as usize;
        let fragment_shader = program.fragment();
        bins.par_iter().for_each(|bin| {
            for &index in bin.indices {
                Rasterizer::rasterize(&triangles[index], &bin.tile, |fragment| {
                    for (&fb_ptr, fragment_shader) in fb_ptr.iter().zip(fragment_shader) {
                        if unsafe {
                            (*(fb_ptr as *mut Framebuffer)).depth_test(
                                fragment.position.x,
                                fragment.position.y,
                                fragment.depth,
                            )
                        } {
                            let frag_color = fragment_shader(&fragment, unsafe {
                                &mut *(ctx_ptr as *mut Context<MAX_BINDS>)
                            });
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

    pub fn bin_triangles<'a, T: Lerp + Copy + Debug + Send + Sync>(
        arena: &'a Bump,
        tiles: &TilesInfo,
        triangles: &[Triangle<T>],
    ) -> &'a mut [TileBin<'a>] {
        let TilesInfo { cols, rows, tiles } = tiles;
        const CHUNK: usize = 8000;
        const BATCH: usize = 256;
        let (s, r) = crossbeam::channel::unbounded();


        let bins_scratch = arena.alloc_slice_fill_with(tiles.len(), |_| {
            bumpalo::collections::Vec::with_capacity_in(64, arena)
        });
        let mut bins = bumpalo::collections::Vec::with_capacity_in(tiles.len(), arena);

        rayon::in_place_scope(|scope| {
            scope.spawn(|_| {
                triangles
                    .par_chunks(CHUNK)
                    .enumerate()
                    .for_each(|(current_chunk, triangles)| {
                        let mut candidates = Candidate {
                            count: 0,
                            indeces: [(0, 0); BATCH],
                        };
                        for (i, triangle) in triangles.iter().enumerate() {
                            let i = current_chunk * CHUNK + i;
                            let [
                                IVec2 { x: ax, y: ay },
                                IVec2 { x: bx, y: by },
                                IVec2 { x: cx, y: cy },
                            ] = triangle.position;

                            let bb_min_x = min(min(ax, bx), cx);
                            let bb_min_y = min(min(ay, by), cy);
                            let bb_max_x = max(max(ax, bx), cx);
                            let bb_max_y = max(max(ay, by), cy);
                            let total_area =
                                Self::double_signed_triangle_area(ax, ay, bx, by, cx, cy);
                            let (x_normal_a, y_normal_a) = (-(by - ay), bx - ax);
                            let (x_normal_b, y_normal_b) = (-(cy - by), cx - bx);
                            let (x_normal_c, y_normal_c) = (-(ay - cy), ax - cx);
                            let cols_min = (bb_min_x / TILE_SIZE.0 as i32).max(0) as usize;
                            let cols_max =
                                ((bb_max_x / TILE_SIZE.0 as i32).max(0) as usize).min(*cols - 1);
                            let rows_min = (bb_min_y / TILE_SIZE.1 as i32).max(0) as usize;
                            let rows_max =
                                ((bb_max_y / TILE_SIZE.1 as i32).max(0) as usize).min(*rows - 1);
                            if total_area >= 2.0 {
                                for row in rows_min..=rows_max {
                                    for col in cols_min..=cols_max {
                                        let tile = &tiles[row * cols + col];

                                        let tile_min = tile.min();
                                        let tile_max = tile.max();
                                        if tile_min.x <= bb_min_x
                                            && tile_max.x >= bb_max_x
                                            && tile_min.y <= bb_min_y
                                            && tile_max.y >= bb_max_y
                                        {
                                            candidates.indeces[candidates.count] =
                                                (row * cols + col, i);
                                            candidates.count += 1;
                                            if candidates.count == BATCH {
                                                candidates.indeces.sort_unstable_by_key(|v| v.0);
                                                s.send(candidates).unwrap();
                                                candidates.count = 0;
                                            }
                                            // bins_scratch[row * cols + col].push(i);
                                            continue;
                                        }

                                        let a = Self::furthest_point(
                                            tile_min, tile_max, x_normal_a, y_normal_a,
                                        );
                                        let b = Self::furthest_point(
                                            tile_min, tile_max, x_normal_b, y_normal_b,
                                        );
                                        let c = Self::furthest_point(
                                            tile_min, tile_max, x_normal_c, y_normal_c,
                                        );
                                        if Self::edge_function(
                                            triangle.position[0],
                                            triangle.position[1],
                                            a,
                                        ) < 0
                                            || Self::edge_function(
                                                triangle.position[1],
                                                triangle.position[2],
                                                b,
                                            ) < 0
                                            || Self::edge_function(
                                                triangle.position[2],
                                                triangle.position[0],
                                                c,
                                            ) < 0
                                        {
                                            continue;
                                        }
                                        candidates.indeces[candidates.count] =
                                            (row * cols + col, i);
                                        candidates.count += 1;
                                        if candidates.count == BATCH {
                                            candidates.indeces.sort_unstable_by_key(|v| v.0);
                                            s.send(candidates).unwrap();
                                            candidates.count = 0;
                                        } // bins_scratch[row * cols + col].push(i);
                                    }
                                }
                            }
                        }
                        if candidates.count > 0 {
                            s.send(candidates).unwrap();
                        }
                    });
                drop(s);
            });

            while let Ok(candidates) = r.recv() {
                for i in 0..candidates.count {
                    let candidate = candidates.indeces[i];
                    unsafe { bins_scratch.get_unchecked_mut(candidate.0).push(candidate.1) };
                }
            }
        });

        bins_scratch.iter().enumerate().for_each(|(bin, indices)| {
            let tile = tiles[bin];
            bins.push(TileBin {
                tile,
                indices,
                morton_key: morton(tile.x as u32, tile.y as u32),
            });
        });
        bins.sort_unstable_by_key(|v| v.morton_key);
        bins.into_bump_slice_mut()
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
    fn double_signed_triangle_area(ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> f32 {
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::{datatypes::Vertex, mesh::Mesh, renderer::Renderer};
    use glam::{Vec4, vec3, vec4};

    #[test]
    fn assemble_and_run_writes_the_rasterized_triangle() {
        let mut renderer = Renderer::new();
        let framebuffer_id = renderer.create_framebuffer(4, 4);
        let program = Program::new(
            |vertex: Vertex<Vec4>| vertex,
            &[|fragment, _| fragment.data],
        );
        let mut pipeline = PipelineForward::new();

        let mesh = Mesh::new(
            vec![
                Vertex::new(vec3(-1.0, 1.0, 0.2), vec4(1.0, 0.0, 0.0, 1.0)),
                Vertex::new(vec3(1.0, 1.0, 0.4), vec4(0.0, 1.0, 0.0, 1.0)),
                Vertex::new(vec3(-1.0, -1.0, 0.6), vec4(0.0, 0.0, 1.0, 1.0)),
            ],
            None,
        );
        let renderer = RefCell::new(renderer);
        let mut context = Context::new(&renderer);
        context.bind_framebuffers_write(framebuffer_id).unwrap();

        pipeline.assemble_and_run(&mut context, &program, &mesh);

        let framebuffer = renderer.borrow_mut().take_framebuffer(framebuffer_id);
        assert_eq!(framebuffer.read_pixel(0, 0), vec4(1.0, 0.0, 0.0, 1.0));
        assert!((framebuffer.read_depth(0, 0) - 0.2).abs() < 1e-6);
    }

    #[test]
    fn run_pixel_writes_through_the_bound_framebuffer() {
        let mut renderer = Renderer::new();
        let framebuffer_id = renderer.create_framebuffer(2, 2);
        let mut pipeline = <PipelineForward>::new();
        let renderer = RefCell::new(renderer);
        let mut context = Context::new(&renderer);
        context.bind_framebuffers_write(framebuffer_id).unwrap();

        pipeline.run_pixel(&mut context, |(x, y)| vec4(x as f32, y as f32, 0.25, 1.0));

        let framebuffer = renderer.borrow_mut().take_framebuffer(framebuffer_id);
        assert_eq!(framebuffer.read_pixel(0, 0), vec4(0.0, 0.0, 0.25, 1.0));
        assert_eq!(framebuffer.read_pixel(1, 0), vec4(1.0, 0.0, 0.25, 1.0));
        assert_eq!(framebuffer.read_pixel(0, 1), vec4(0.0, 1.0, 0.25, 1.0));
        assert_eq!(framebuffer.read_pixel(1, 1), vec4(1.0, 1.0, 0.25, 1.0));
    }
}
