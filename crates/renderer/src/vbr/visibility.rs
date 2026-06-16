use crate::datatypes::DrawTri;

#[derive(Debug)]
pub struct VBuffer {
    visibility: Vec<DrawTri>,
    width: usize,
    height: usize,
}

impl VBuffer {
    #[inline]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            visibility: vec![DrawTri::default(); width * height],
            width,
            height,
        }
    }

    #[inline]
    pub fn put(&mut self, tri: DrawTri, x: usize, y: usize) {
        self.visibility[y * self.width + x] = tri // TODO: Bounds check?
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> &DrawTri {
        &self.visibility[y * self.width + x] // TODO: Bounds check?
    }
}
