use glam::{IVec2, Vec2, Vec4};
use smallvec::SmallVec;

pub static TILE_SIZE: (usize, usize) = (128, 64);

pub struct Framebuffer {
    r: Vec<f32>,
    g: Vec<f32>,
    b: Vec<f32>,
    a: Vec<f32>,
    depth: Vec<f32>,
    width: i32,
    height: i32,
    generation: Vec<u32>,
    current_generation: u32,
    tiles: Vec<Tile>,
    tiles_list: Vec<usize>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let cols = (width + TILE_SIZE.0 - 1) / TILE_SIZE.0;
        let rows = (height + TILE_SIZE.1 - 1) / TILE_SIZE.1;
        let tiles_dim = cols.max(rows).next_power_of_two();
        let mut tiles = vec![Tile::default(); tiles_dim * tiles_dim];
        let mut tiles_list = Vec::with_capacity(cols * rows);
        for yt in 0..rows {
            let y = yt * TILE_SIZE.1;
            for xt in 0..cols {
                let x = xt * TILE_SIZE.0;
                let i = morton(xt as u32, yt as u32) as usize;
                tiles[i] = Tile {
                    x: x as i32,
                    y: y as i32,
                    width: TILE_SIZE.0.min(width - x) as i32,
                    height: TILE_SIZE.1.min(height - y) as i32,
                };
                tiles_list.push(i);
            }
        }

        Self {
            r: vec![0.0; width * height],
            g: vec![0.0; width * height],
            b: vec![0.0; width * height],
            a: vec![0.0; width * height],
            depth: vec![f32::INFINITY; width * height],
            width: width as i32,
            height: height as i32,
            generation: vec![0; width * height],
            current_generation: 0,
            tiles,
            tiles_list,
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
    pub fn get_tiles(&self) -> (&[Tile], &[usize]) {
        (&self.tiles, &self.tiles_list)
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

    pub unsafe fn write_fragment(&mut self, x: i32, y: i32, depth: f32, color: Vec4) {
        let index = unsafe { y.unchecked_mul(self.width).unchecked_add(x) as usize };
        if depth < self.depth[index] {
            unsafe { *self.depth.get_unchecked_mut(index) = depth };
            unsafe { *self.r.get_unchecked_mut(index) = color.x };
            unsafe { *self.g.get_unchecked_mut(index) = color.y };
            unsafe { *self.b.get_unchecked_mut(index) = color.z };
            unsafe { *self.a.get_unchecked_mut(index) = color.w };
            unsafe { *self.generation.get_unchecked_mut(index) = self.current_generation };
        }
    }

    pub fn get_color(&self) -> (&Vec<f32>, &Vec<f32>, &Vec<f32>, &Vec<f32>, &Vec<u32>) {
        (
            self.r.as_ref(),
            self.g.as_ref(),
            self.b.as_ref(),
            self.a.as_ref(),
            self.generation.as_ref(),
        )
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

#[derive(Debug, Clone, Default)]
pub struct TileBin {
    pub(crate) tile: Tile,
    pub(crate) indices: SmallVec<[usize; 64]>,
}

pub(crate) fn morton(mut x: u32, mut y: u32) -> u32 {
    // TODO: Either expand the morton or do a bounds check before. Usize is 64 bit while x and y got restrictions well bellow that
    // https://graphics.stanford.edu/~seander/bithacks.html#InterleaveBMN
    // x and y must initially be less than 65536.

    x = (x | (x << 8)) & 0x00FF00FF;
    x = (x | (x << 4)) & 0x0F0F0F0F;
    x = (x | (x << 2)) & 0x33333333;
    x = (x | (x << 1)) & 0x55555555;

    y = (y | (y << 8)) & 0x00FF00FF;
    y = (y | (y << 4)) & 0x0F0F0F0F;
    y = (y | (y << 2)) & 0x33333333;
    y = (y | (y << 1)) & 0x55555555;

    x | (y << 1)
}
