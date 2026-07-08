use glam::{IVec2, Vec2, Vec4, vec4};
use smallvec::SmallVec;
use wide::bytemuck;

pub static TILE_SIZE: (usize, usize) = (64, 64);
pub static MAX_BINDS: usize = 16;

#[derive(Debug, Clone)]
pub struct Framebuffer {
    color: Vec<Vec4>,
    depth: Vec<f32>,
    width: i32,
    height: i32,
    generation: Vec<u32>,
    current_generation: u32,
    tiles: TilesInfo,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let cols = (width + TILE_SIZE.0 - 1) / TILE_SIZE.0;
        let rows = (height + TILE_SIZE.1 - 1) / TILE_SIZE.1;
        let tiles_dim = cols.max(rows).next_power_of_two();
        let mut tiles = vec![Tile::default(); tiles_dim * tiles_dim];
        for yt in 0..rows {
            let y = yt * TILE_SIZE.1;
            for xt in 0..cols {
                let x = xt * TILE_SIZE.0;
                let i = yt * cols + xt;
                // let i = morton(xt as u32, yt as u32) as usize;
                tiles[i] = Tile {
                    x: x as i32,
                    y: y as i32,
                    width: TILE_SIZE.0.min(width - x) as i32,
                    height: TILE_SIZE.1.min(height - y) as i32,
                };
            }
        }
        let tiles = TilesInfo { cols, rows, tiles };
        Self {
            color: vec![Vec4::default(); width * height],
            depth: vec![f32::INFINITY; width * height],
            width: width as i32,
            height: height as i32,
            generation: vec![0; width * height],
            current_generation: 0,
            tiles,
            // tiles_list,
        }
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.height
    }

    #[inline]
    pub fn current_generation(&self) -> u32 {
        self.current_generation
    }

    #[inline]
    pub fn get_tiles(&self) -> &TilesInfo {
        &self.tiles
    }

    #[inline]
    pub fn clear(&mut self, color: Vec4) {
        // self.r = vec![color.x; self.width * self.height];
        // self.g = vec![color.y; self.width * self.height];
        // self.b = vec![color.z; self.width * self.height];
        // self.a = vec![color.w; self.width * self.height];
        // self.depth = vec![f32::INFINITY; self.width * self.height];
        let r = color.x;
        let g = color.y;
        let b = color.z;
        let a = color.w;

        self.current_generation = self.current_generation.wrapping_add(1);
        // self.r.fill(r);
        // self.g.fill(g);
        // self.b.fill(b);
        // self.a.fill(a);
        // self.depth.fill(f32::INFINITY);
        // for i in 0..self.r.len() {
        //     self.r[i] = r;
        //     self.g[i] = g;
        //     self.b[i] = b;
        //     self.a[i] = a;
        //     self.depth[i] = f32::INFINITY;
        // }
    }

    #[inline]
    pub unsafe fn write_fragment(&mut self, x: i32, y: i32, depth: f32, color: Vec4) {
        let index = unsafe { y.unchecked_mul(self.width).unchecked_add(x) as usize };
        // if depth < self.depth[index] {
        unsafe { *self.depth.get_unchecked_mut(index) = depth };
        unsafe { *self.color.get_unchecked_mut(index) = color };
        // unsafe { *self.r.get_unchecked_mut(index) = color.x };
        // unsafe { *self.g.get_unchecked_mut(index) = color.y };
        // unsafe { *self.b.get_unchecked_mut(index) = color.z };
        // unsafe { *self.a.get_unchecked_mut(index) = color.w };
        unsafe { *self.generation.get_unchecked_mut(index) = self.current_generation };
        // }
    }

    #[inline]
    pub fn depth_test(&self, x: i32, y: i32, depth: f32) -> bool {
        let index = unsafe { y.unchecked_mul(self.width).unchecked_add(x) as usize };
        if self.generation[index] != self.current_generation {
            true
        } else {
            depth < self.depth[index]
        }
    }

    pub fn get_color(&self) -> (&[Vec4], &[u32]) {
        (self.color.as_ref(), self.generation.as_ref())
    }

    pub fn read_pixel(&self, x: i32, y: i32) -> Vec4 {
        let index = unsafe { y.unchecked_mul(self.width).unchecked_add(x) as usize };
        if self.generation[index] != self.current_generation {
            vec4(0.0, 0.0, 0.0, 1.0)
        } else {
            self.color[index]
        }
    }

    pub fn read_depth(&self, x: i32, y: i32) -> f32 {
        let index = unsafe { y.unchecked_mul(self.width).unchecked_add(x) as usize };
        if self.generation[index] != self.current_generation {
            f32::INFINITY
        } else {
            self.depth[index]
        }
    }

    // #[inline]
    // pub fn transfer(&mut self, source: &Framebuffer) {
    //     rayon::scope(|s| {
    //         s.spawn(|_| self.color.copy_from_slice(&source.color));
    //         s.spawn(|_| self.depth.copy_from_slice(&source.depth));
    //         s.spawn(|_| self.generation.copy_from_slice(&source.generation));
    //     });
    //     self.current_generation = source.current_generation;
    // }

    #[inline]
    pub fn buffer_to_u8(&self, out: &mut [i32]) {
        let (color, generation) = (&self.color, &self.generation);
        let current_gen = self.current_generation;
        let conv = wide::f32x4::from([
            255.0 * (1 << 16) as f32,
            255.0 * (1 << 8) as f32,
            255.0,
            0.0,
        ]);
        for (i, (color, &generation)) in color.iter().zip(generation).enumerate() {
            if generation == current_gen {
                let color = wide::f32x4::from(color.to_array()) * conv;
                let combined = bytemuck::cast(color.fast_round_int().reduce_add());
                unsafe { *out.get_unchecked_mut(i) = combined };
            } else {
                unsafe { *out.get_unchecked_mut(i) = 0 };
            }
        }
        // const CHUNK: usize = 4096;
        // out.par_chunks_mut(CHUNK)
        //     .zip(color.par_chunks(CHUNK))
        //     .zip(generation.par_chunks(CHUNK))
        //     .for_each(|((out, color), generation)| {
        //         for ((out, color), &generation) in out.iter_mut().zip(color).zip(generation) {
        //             if generation == current_gen {
        //                 let color = wide::f32x4::from(color.to_array()) * conv;
        //                 let combined = bytemuck::cast(color.fast_round_int().reduce_add());
        //                 *out = combined;
        //             } else {
        //                 *out = 0;
        //             }
        //         }
        //     });
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl Tile {
    pub fn min(&self) -> IVec2 {
        IVec2::new(self.x, self.y)
    }

    pub fn max(&self) -> IVec2 {
        IVec2::new(self.x + self.width, self.y + self.height)
    }
}

#[derive(Debug, Clone)]
pub struct TilesInfo {
    pub cols: usize,
    pub rows: usize,
    pub tiles: Vec<Tile>,
}

#[derive(Debug, Clone, Default)]
pub struct TileBin<'a> {
    pub(crate) tile: Tile,
    pub(crate) indices: &'a [usize],
    pub(crate) morton_key: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{IVec2, vec4};

    #[test]
    fn clear_resets_visible_state_via_generation() {
        let mut framebuffer = Framebuffer::new(2, 2);
        let initial_generation = framebuffer.current_generation();

        framebuffer.clear(vec4(0.25, 0.5, 0.75, 1.0));
        assert_eq!(framebuffer.current_generation(), initial_generation + 1);
        assert_eq!(framebuffer.read_pixel(0, 0), vec4(0.0, 0.0, 0.0, 1.0));
        assert_eq!(framebuffer.read_depth(0, 0), f32::INFINITY);
        assert!(framebuffer.depth_test(0, 0, 0.5));

        unsafe {
            framebuffer.write_fragment(1, 1, 0.25, vec4(1.0, 0.0, 0.0, 1.0));
        }

        assert_eq!(framebuffer.read_pixel(1, 1), vec4(1.0, 0.0, 0.0, 1.0));
        assert_eq!(framebuffer.read_depth(1, 1), 0.25);
        assert!(!framebuffer.depth_test(1, 1, 0.5));

        framebuffer.clear(vec4(0.0, 0.0, 0.0, 0.0));
        assert_eq!(framebuffer.read_pixel(1, 1), vec4(0.0, 0.0, 0.0, 1.0));
        assert_eq!(framebuffer.read_depth(1, 1), f32::INFINITY);
        assert!(framebuffer.depth_test(1, 1, 0.5));
    }

    #[test]
    fn partial_framebuffer_tiles_are_clipped_to_the_framebuffer_bounds() {
        let framebuffer = Framebuffer::new(65, 65);
        let tiles = framebuffer.get_tiles();

        assert_eq!(tiles.cols, 2);
        assert_eq!(tiles.rows, 2);
        assert_eq!(tiles.tiles.len(), 4);
        assert_eq!(tiles.tiles[0].min(), IVec2::new(0, 0));
        assert_eq!(tiles.tiles[0].max(), IVec2::new(64, 64));
        assert_eq!(tiles.tiles[3].min(), IVec2::new(64, 64));
        assert_eq!(tiles.tiles[3].max(), IVec2::new(65, 65));
    }
}
