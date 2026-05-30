use glam::Vec4;

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
        let r = color.x;
        let g = color.y;
        let b = color.z;
        let a = color.w;

        for i in 0..self.r.len() {
            unsafe { *self.r.get_unchecked_mut(i) = r };
            unsafe { *self.g.get_unchecked_mut(i) = g };
            unsafe { *self.b.get_unchecked_mut(i) = b };
            unsafe { *self.a.get_unchecked_mut(i) = a };
            unsafe { *self.depth.get_unchecked_mut(i) = f32::INFINITY };
        }
    }

    pub fn write_fragment(&mut self, x: usize, y: usize, depth: f32, color: Vec4) {
        let index = y * self.width + x;
        if depth < self.depth[index] {
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
