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
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use smallvec::SmallVec;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct PipelineForward<T: Lerp + Send + Sync, VS, FS> {
    vertex_shader: VS,
    fragment_shader: FS,
    render_buffer: Option<usize>,
    triangles: Vec<Triangle<T>>,
    tiles: Vec<Tile>,
    bins: Vec<TileBin>,
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
            triangles: Vec::with_capacity(1000), // FIXME: Just straight up guessing.
            tiles: Vec::with_capacity(100),
            bins: Vec::with_capacity(100),
        }
    }

    pub fn attach_render_buffer(&mut self, buffer_id: usize) {
        self.render_buffer = Some(buffer_id);
    }

    pub fn detach_render_buffer(&mut self) {
        self.render_buffer = None;
    }

    pub fn assemble_and_run(&mut self, renderer: &mut Renderer, mesh: &Mesh<T>) {
        let Some(render_buffer) = self.render_buffer else {
            panic!("No render buffer attached to the pipeline.");
        };

        let mut framebuffer = renderer.take_framebuffer(render_buffer);
        self.tiles.clear();
        self.triangles.clear();
        let screen_width = framebuffer.width();
        let screen_height = framebuffer.height();

        let cols = (screen_width + TILE_SIZE.0 - 1) / TILE_SIZE.0;
        let rows = (screen_height + TILE_SIZE.1 - 1) / TILE_SIZE.1;
        self.tiles.extend((0..rows).flat_map(|row| {
            (0..cols).map(move |col| {
                let x = col * TILE_SIZE.0;
                let y = row * TILE_SIZE.1;
                Tile {
                    x,
                    y,
                    width: TILE_SIZE.0.min(screen_width - x) as i32,
                    height: TILE_SIZE.1.min(screen_height -  y) as i32,
                }
            })
        }));
        // .collect::<Vec<_>>();

        self.triangles.extend(
            mesh.positions
                .chunks_exact(3)
                .zip(mesh.data.chunks_exact(3))
                .map(|(pos, data)| {
                    let v0 = (self.vertex_shader)(Vertex::new(pos[0], data[0]));
                    let v1 = (self.vertex_shader)(Vertex::new(pos[1], data[1]));
                    let v2 = (self.vertex_shader)(Vertex::new(pos[2], data[2]));
                    Triangle {
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
                    }
                }),
        );
        // .collect::<Vec<_>>();
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

        let fb_ptr = (&mut framebuffer as *mut Framebuffer) as usize;

        self.bin_triangles();
        let fragment_shader = &self.fragment_shader;
        // black_box(bins);
        self.bins.par_iter().for_each(|bin| {
            // let mut frags = vec![];
            for &index in &bin.indices {
                Rasterizer::rasterize(&self.triangles[index], &bin.tile, |fragment| {
                    let frag_color = fragment_shader(fragment);
                    let fb_ptr = fb_ptr as *mut Framebuffer; // Safety: This is whack
                    unsafe {
                        (*fb_ptr).write_fragment(
                            fragment.position.x,
                            fragment.position.y,
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
        // for triangle in &triangles {
        //     Rasterizer::rasterize(triangle, screen_width, screen_height, |fragment| {
        //         let frag_color = (self.fragment_shader)(fragment);
        //         unsafe {
        //             framebuffer.write_fragment(
        //                 fragment.position.x as usize,
        //                 fragment.position.y as usize,
        //                 0.5, // dummy depth value
        //                 frag_color,
        //             )
        //         };
        //     });
        // }
        renderer.put_framebuffer(render_buffer, framebuffer);
    }

    fn bin_triangles(
        &mut self,
        // triangles: &[Triangle<T>],
        // tiles: &[Tile],
        // screen_width: usize,
        // screen_height: usize,
    ) {
        if self.bins.len() < self.tiles.len() {
            self.bins.extend((self.bins.len()..self.tiles.len()).map(|i| TileBin {
                tile: self.tiles[i],
                indices: SmallVec::new(),
            }));
        }
        for i in 0..self.bins.len() {
            self.bins[i].tile = self.tiles[i];
            self.bins[i].indices.clear();
        }
        // .collect();
        self.triangles.iter().enumerate().for_each(|(i, triangle)| {
            // let IVec2 { x: ax, y: ay } =
            //     Rasterizer::to_screen_space(triangle.position[0], screen_width, screen_height);
            // let IVec2 { x: bx, y: by } =
            //     Rasterizer::to_screen_space(triangle.position[1], screen_width, screen_height);
            // let IVec2 { x: cx, y: cy } =
            //     Rasterizer::to_screen_space(triangle.position[2], screen_width, screen_height);

            let IVec2 { x: ax, y: ay } = triangle.position[0];
            let IVec2 { x: bx, y: by } = triangle.position[1];
            let IVec2 { x: cx, y: cy } = triangle.position[2];

            let bb_min_x = min(min(ax, bx), cx);
            let bb_min_y = min(min(ay, by), cy);
            let bb_max_x = max(max(ax, bx), cx);
            let bb_max_y = max(max(ay, by), cy);
            for bin in self.bins.iter_mut() {
                let tile_min = bin.tile.min();
                let tile_max = bin.tile.max();
                if tile_max.x < bb_min_x
                    || tile_min.x > bb_max_x
                    || tile_max.y < bb_min_y
                    || tile_min.y > bb_max_y
                {
                    continue;
                }

                if tile_min.x <= bb_min_x
                    && tile_max.x >= bb_max_x
                    && tile_min.y <= bb_max_y
                    && tile_max.y >= bb_max_y
                {
                    bin.indices.push(i);
                    continue;
                }

                let (x_normal_a, y_normal_a) = (-(by - ay), bx - ax);
                let (x_normal_b, y_normal_b) = (-(cy - by), cx - bx);
                let (x_normal_c, y_normal_c) = (-(ay - cy), ax - cx);
                let a = Self::furthest_point(tile_min, tile_max, x_normal_a, y_normal_a);
                let b = Self::furthest_point(tile_min, tile_max, x_normal_b, y_normal_b);
                let c = Self::furthest_point(tile_min, tile_max, x_normal_c, y_normal_c);
                if Self::edge_function(triangle.position[0], triangle.position[1], a) < 0
                    || Self::edge_function(triangle.position[1], triangle.position[2], b) < 0
                    || Self::edge_function(triangle.position[2], triangle.position[0], c) < 0
                {
                    continue;
                }
                bin.indices.push(i);
            }
        });
        self.bins.retain(|b| !b.indices.is_empty());
        // dbg!(binned);
        // panic!();
        // binned
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
