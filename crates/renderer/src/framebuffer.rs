use glam::{IVec2, Vec2, Vec4};

pub static TILE_SIZE: (usize, usize) = (128, 64);

pub struct Framebuffer {
    r: Vec<f32>,
    g: Vec<f32>,
    b: Vec<f32>,
    a: Vec<f32>,
    depth: Vec<f32>,
    width: usize,
    height: usize,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            r: vec![0.0; width * height],
            g: vec![0.0; width * height],
            b: vec![0.0; width * height],
            a: vec![0.0; width * height],
            depth: vec![f32::INFINITY; width * height],
            width,
            height,
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
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

        self.r.fill(r);
        self.g.fill(g);
        self.b.fill(b);
        self.a.fill(a);
        self.depth.fill(f32::INFINITY);
        // for i in 0..self.r.len() {
        //     unsafe { *self.r.get_unchecked_mut(i) = r };
        //     unsafe { *self.g.get_unchecked_mut(i) = g };
        //     unsafe { *self.b.get_unchecked_mut(i) = b };
        //     unsafe { *self.a.get_unchecked_mut(i) = a };
        //     unsafe { *self.depth.get_unchecked_mut(i) = f32::INFINITY };
        // }
    }

    pub unsafe fn write_fragment(&mut self, x: usize, y: usize, depth: f32, color: Vec4) {
        let index = unsafe { y.unchecked_mul(self.width).unchecked_add(x) };
        if depth < *unsafe { self.depth.get_unchecked(index) } {
            unsafe { *self.depth.get_unchecked_mut(index) = depth };
            unsafe { *self.r.get_unchecked_mut(index) = color.x };
            unsafe { *self.g.get_unchecked_mut(index) = color.y };
            unsafe { *self.b.get_unchecked_mut(index) = color.z };
            unsafe { *self.a.get_unchecked_mut(index) = color.w };
        }
    }

    pub fn get_color(&self) -> (&Vec<f32>, &Vec<f32>, &Vec<f32>, &Vec<f32>) {
        (
            self.r.as_ref(),
            self.g.as_ref(),
            self.b.as_ref(),
            self.a.as_ref(),
        )
    }
}



#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) x: usize,
    pub(crate) y: usize,
}

impl Tile {
    pub fn min(&self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }

    pub fn max(&self) -> Vec2 {
        Vec2::new((self.x + self.width) as f32, (self.y + self.height) as f32)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TileBin {
    pub(crate) tile: Tile,
    pub(crate) indices: Vec<usize>
}