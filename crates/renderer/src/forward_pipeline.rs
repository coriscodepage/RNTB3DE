use std::{
    cmp::{max, min},
    mem::MaybeUninit,
    ptr::NonNull,
    sync::atomic::{
        AtomicUsize,
        Ordering::{self, Relaxed},
    },
};

use crate::{
    abstraction::{
        context::{ResolvedSamplers, ResolvedWriters},
        program::Program,
    },
    datatypes::{FragmentInput, Triangle, Vertex, VertexHomogenous},
    framebuffer::{Framebuffer, MAX_BINDS, TILE_SIZE, Tile, TileBin, TilesInfo},
    lerp::Lerp,
    mesh::Mesh,
    rasterizer::Rasterizer,
    framebuffer_storage::morton,
};
use bumpalo::Bump;
use glam::{
    IVec2, Vec2, Vec3, Vec4,
};
use rayon::{
    iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    slice::{ParallelSlice, ParallelSliceMut},
};
use std::fmt::Debug;

// SAFETY: This is bad on many levels. We are taking a ref and coercing it into a mut via a pointer deref...
#[derive(Debug, Clone, Copy)]
struct SendPointer<T> {
    pointer: NonNull<T>,
}

impl<T> SendPointer<T> {
    #[inline(always)]
    unsafe fn as_ref(&self) -> &T {
        unsafe { self.pointer.as_ref() }
    }

    #[inline(always)]
    unsafe fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.pointer.as_ptr() }
    }

    #[inline(always)]
    unsafe fn as_mut_ptr(&self) -> *mut T {
        unsafe { &mut *self.pointer.as_ptr() }
    }
}

unsafe impl<T> Send for SendPointer<T> {}
unsafe impl<T> Sync for SendPointer<T> {}

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

    pub fn assemble_and_run<T, D, N, VS, FS>(
        &mut self,
        samplers: &ResolvedSamplers<MAX_BINDS>,
        writers: &mut ResolvedWriters<MAX_BINDS>,
        program: &Program<T, D, N, VS, FS>,
        uniforms_vertex: D,
        uniforms_fragment: N,
        mesh: &Mesh<T>,
    ) where
        T: Lerp + Copy + Debug + Send + Sync,
        D: Send + Sync,
        N: Send + Sync,
        VS: Fn(Vertex<T>, &D) -> VertexHomogenous<T> + Send + Sync,
        FS: Fn(&FragmentInput<T>, &ResolvedSamplers<MAX_BINDS>, &N) -> Vec4 + Send + Sync + Clone,
    {
        let framebuffers = writers.get_mut();
        if framebuffers.len() != program.fragment().len() {
            panic!("Render buffer count does not match shader count.");
        }

        self.arena.reset();

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
                let v0 = vertex_shader(Vertex::new(pos[0], data[0]), &uniforms_vertex);
                let v1 = vertex_shader(Vertex::new(pos[1], data[1]), &uniforms_vertex);
                let v2 = vertex_shader(Vertex::new(pos[2], data[2]), &uniforms_vertex);

                let ndc = [
                    v0.position.truncate() / v0.position.w,
                    v1.position.truncate() / v1.position.w,
                    v2.position.truncate() / v2.position.w,
                ];

                let p0 = Self::to_screen_space(ndc[0], screen_width, screen_height);
                let p1 = Self::to_screen_space(ndc[1], screen_width, screen_height);
                let p2 = Self::to_screen_space(ndc[2], screen_width, screen_height);
                match (p0, p1, p2) {
                    // FIXME: We don't need this. We need to reject some other way. This is another branch in the hot loop.
                    (Some(p0), Some(p1), Some(p2)) => {
                        tri.write(Triangle {
                            position: [p0, p1, p2],
                            depth: [
                                Vec2::new(v0.position.z, 1.0),
                                Vec2::new(v1.position.z, 1.0),
                                Vec2::new(v2.position.z, 1.0),
                            ],
                            data: [v0.data, v1.data, v2.data],
                        });
                    }
                    _ => {
                        tri.write(Triangle::degenerate([v0.data, v0.data, v0.data]));
                    }
                }
            });

        // INFO: Same memory? What is going on really? We conjure a new let binding out of thin air.
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

        let fb_ptr = self
            .arena
            .alloc_slice_fill_iter(framebuffers.iter_mut().map(|fb| SendPointer {
                pointer: NonNull::from_mut(fb),
            }));
        // let ctx_ptr = &mut context as *mut Context<MAX_BINDS> as usize;
        let ctx_ptr = SendPointer {
            pointer: NonNull::from_ref(&samplers),
        };
        let fragment_shader = program.fragment();
        bins.par_iter().for_each(|bin| {
            for &index in bin.indices {
                Rasterizer::rasterize(&triangles[index], &bin.tile, |fragment| {
                    for (fb_ptr, fragment_shader) in fb_ptr.iter().zip(fragment_shader) {
                        if unsafe {
                            fb_ptr.as_ref().depth_test(
                                fragment.position.x,
                                fragment.position.y,
                                fragment.depth,
                            )
                        } {
                            let frag_color =
                                fragment_shader(&fragment, unsafe { ctx_ptr.as_mut() }, &uniforms_fragment);
                            unsafe {
                                fb_ptr.as_mut().write_fragment(
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
    }

    pub fn bin_triangles<'a, T: Lerp + Copy + Debug + Send + Sync>(
        arena: &'a Bump,
        tiles: &TilesInfo,
        triangles: &[Triangle<T>],
    ) -> &'a mut [TileBin<'a>] {
        let TilesInfo { cols, rows, tiles } = tiles;
        const CHUNK: usize = 400;

        let mut bins = bumpalo::collections::Vec::with_capacity_in(tiles.len(), arena);

        let counts = arena.alloc_slice_fill_with(tiles.len(), |_| AtomicUsize::new(0));

        triangles.par_chunks(CHUNK).for_each(|triangles| {
            for triangle in triangles.iter() {
                Self::check_triangle(triangle, *rows, *cols, tiles, |tile_idx| {
                    counts[tile_idx].fetch_add(1, Ordering::Relaxed);
                });
            }
        });

        let offsets = arena.alloc_slice_fill_with(tiles.len() + 1, |_| 0usize);

        let mut running_total = 0;
        counts
            .iter()
            .zip(offsets.iter_mut())
            .for_each(|(count, offset)| {
                *offset = running_total;
                running_total += count.load(Relaxed);
            });
        offsets[tiles.len()] = running_total;

        let cursors = arena.alloc_slice_fill_with(tiles.len(), |i| AtomicUsize::new(offsets[i]));
        let flat = arena.alloc_slice_fill_copy(running_total, 0usize);
        let flat_ptr = flat.as_mut_ptr() as usize;

        triangles
            .par_chunks(CHUNK)
            .enumerate()
            .for_each(|(current_chunk, triangles)| {
                for (i, triangle) in triangles.iter().enumerate() {
                    let i = current_chunk * CHUNK + i;
                    Self::check_triangle(triangle, *rows, *cols, tiles, |tile_idx| {
                        let cursor = cursors[tile_idx].fetch_add(1, Ordering::Relaxed);
                        unsafe { *(flat_ptr as *mut usize).add(cursor) = i };
                    });
                }
            });

        tiles.iter().enumerate().for_each(|(i, &tile)| {
            let start = offsets[i];
            let stop = offsets[i + 1];
            let indices = &flat[start..stop];
            if !indices.is_empty() {
                bins.push(TileBin {
                    tile,
                    indices,
                    morton_key: morton(tile.x as u32, tile.y as u32),
                });
            }
        });

        bins.par_sort_unstable_by_key(|v| v.morton_key);
        bins.into_bump_slice_mut()
    }

    fn check_triangle<T: Lerp + Copy + Debug + Send + Sync, F: FnMut(usize)>(
        triangle: &Triangle<T>,
        rows: usize,
        cols: usize,
        tiles: &[Tile],
        mut func: F,
    ) {
        let [
            IVec2 { x: ax, y: ay },
            IVec2 { x: bx, y: by },
            IVec2 { x: cx, y: cy },
        ] = triangle.position;

        let bb_min_x = min(min(ax, bx), cx);
        let bb_min_y = min(min(ay, by), cy);
        let bb_max_x = max(max(ax, bx), cx);
        let bb_max_y = max(max(ay, by), cy);
        let total_area = Self::double_signed_triangle_area(ax, ay, bx, by, cx, cy);
        let normal_a = IVec2::new(-(by - ay), bx - ax);
        let normal_b = IVec2::new(-(cy - by), cx - bx);
        let normal_c = IVec2::new(-(ay - cy), ax - cx);
        let cols_min = (bb_min_x / TILE_SIZE.0 as i32).max(0) as usize;
        let cols_max = ((bb_max_x / TILE_SIZE.0 as i32).max(0) as usize).min(cols - 1);
        let rows_min = (bb_min_y / TILE_SIZE.1 as i32).max(0) as usize;
        let rows_max = ((bb_max_y / TILE_SIZE.1 as i32).max(0) as usize).min(rows - 1);
        if total_area >= 2 {
            for row in rows_min..=rows_max {
                for col in cols_min..=cols_max {
                    let tile = &tiles[row * cols + col];

                    let tile_min = tile.min();
                    let tile_max = tile.max();
                    if (tile_min.x <= bb_min_x)
                        & (tile_max.x >= bb_max_x)
                        & (tile_min.y <= bb_min_y)
                        & (tile_max.y >= bb_max_y)
                    {
                        func(row * cols + col);
                        continue;
                    }

                    let a = Self::furthest_point(tile_min, tile_max, normal_a.x, normal_a.y);
                    let b = Self::furthest_point(tile_min, tile_max, normal_b.x, normal_b.y);
                    let c = Self::furthest_point(tile_min, tile_max, normal_c.x, normal_c.y);
                    if Self::edge_function(triangle.position[0], triangle.position[1], a) < 0
                        || Self::edge_function(triangle.position[1], triangle.position[2], b) < 0
                        || Self::edge_function(triangle.position[2], triangle.position[0], c) < 0
                    {
                        continue;
                    }
                    func(row * cols + col);
                }
            }
        }
    }

    pub fn run_pixel<P>(
        &mut self,
        samplers: &ResolvedSamplers<MAX_BINDS>,
        writers: &mut ResolvedWriters<MAX_BINDS>,
        shader: P,
    ) where
        P: Fn((i32, i32), &ResolvedSamplers<MAX_BINDS>) -> Vec4 + Send + Sync,
    {
        let sampler_ptr = SendPointer {
            pointer: NonNull::from_ref(samplers),
        };

        for framebuffer in writers.get_mut() {
            let fb_ptr = framebuffer as *mut Framebuffer as usize;
            framebuffer.get_tiles().tiles.par_iter().for_each(|tile| {
                let min_t = tile.min();
                let max_t = tile.max();
                for y in min_t.y..max_t.y {
                    for x in min_t.x..max_t.x {
                        let frag_color = shader((x, y), unsafe { sampler_ptr.as_ref() });
                        unsafe {
                            (*(fb_ptr as *mut Framebuffer)).write_fragment(x, y, 1.0, frag_color)
                        };
                    }
                }
            });
        }
    }

    #[inline(always)]
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
    fn double_signed_triangle_area(ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> i32 {
        return ((by - ay) * (bx + ax) + (cy - by) * (cx + bx) + (ay - cy) * (ax + cx));
    }

    // fn project(v: Vec3, f: f32) -> Vec3 {
    //     v * Mat4::from
    // }

    #[inline(always)]
    pub(crate) fn to_screen_space(ndc: Vec3, width: i32, height: i32) -> Option<IVec2> {
        // FIXME: This is a reject approach. This straight up won't work for a game engine.
        // TODO: Implement https://pl.wikipedia.org/wiki/Algorytm_Sutherlanda-Hodgmana
        let x = (ndc.x + 1.0) * 0.5 * width as f32;
        let y = (ndc.y + 1.0) * 0.5 * height as f32;
        if x < 0.0 || x > (width - 1) as f32 || y < 0.0 || y > (height - 1) as f32 {
            None
        } else {
            Some(IVec2::new(unsafe { x.to_int_unchecked() }, unsafe {
                y.to_int_unchecked()
            }))
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use std::cell::RefCell;

//     use super::*;
//     use crate::{datatypes::Vertex, mesh::Mesh, renderer::Renderer};
//     use glam::{Vec4, vec3, vec4};

//     #[test]
//     fn assemble_and_run_writes_the_rasterized_triangle() {
//         let mut renderer = Renderer::new();
//         let framebuffer_id = renderer.create_framebuffer(4, 4);
//         let program = Program::new(
//             |vertex: Vertex<Vec4>| vertex,
//             &[|fragment, _| fragment.data],
//         );
//         let mut pipeline = PipelineForward::new();

//         let mesh = Mesh::new(
//             vec![
//                 Vertex::new(vec3(-1.0, 1.0, 0.2), vec4(1.0, 0.0, 0.0, 1.0)),
//                 Vertex::new(vec3(1.0, 1.0, 0.4), vec4(0.0, 1.0, 0.0, 1.0)),
//                 Vertex::new(vec3(-1.0, -1.0, 0.6), vec4(0.0, 0.0, 1.0, 1.0)),
//             ],
//             None,
//         );
//         let renderer = RefCell::new(renderer);
//         let mut context = Context::new(&renderer);
//         context.bind_framebuffers_write(framebuffer_id).unwrap();

//         pipeline.assemble_and_run(&mut context, &program, &mesh);

//         let framebuffer = renderer.borrow_mut().take_framebuffer(framebuffer_id);
//         assert_eq!(framebuffer.read_pixel(0, 0), vec4(1.0, 0.0, 0.0, 1.0));
//         assert!((framebuffer.read_depth(0, 0) - 0.2).abs() < 1e-6);
//     }

//     #[test]
//     fn run_pixel_writes_through_the_bound_framebuffer() {
//         let mut renderer = Renderer::new();
//         let framebuffer_id = renderer.create_framebuffer(2, 2);
//         let mut pipeline = <PipelineForward>::new();
//         let renderer = RefCell::new(renderer);
//         let mut context = Context::new(&renderer);
//         context.bind_framebuffers_write(framebuffer_id).unwrap();

//         pipeline.run_pixel(&mut context, |(x, y), _| {
//             vec4(x as f32, y as f32, 0.25, 1.0)
//         });

//         let framebuffer = renderer.borrow_mut().take_framebuffer(framebuffer_id);
//         assert_eq!(framebuffer.read_pixel(0, 0), vec4(0.0, 0.0, 0.25, 1.0));
//         assert_eq!(framebuffer.read_pixel(1, 0), vec4(1.0, 0.0, 0.25, 1.0));
//         assert_eq!(framebuffer.read_pixel(0, 1), vec4(0.0, 1.0, 0.25, 1.0));
//         assert_eq!(framebuffer.read_pixel(1, 1), vec4(1.0, 1.0, 0.25, 1.0));
//     }
// }
