use glam::Vec3;

use crate::{datatypes::Vertex, lerp::Lerp};

pub struct Mesh<T: Lerp> {
    pub(crate) positions: Vec<Vec3>,
    pub(crate) data: Vec<T>,
}

impl<T> Mesh<T>
where
    T: Lerp,
{
    pub fn new(vertices: Vec<Vertex<T>>, indices: Option<Vec<u32>>) -> Self {
        let n = indices.as_ref().map_or(vertices.len(), |i| i.len());
        let mut positions = Vec::with_capacity(n);
        let mut data = Vec::with_capacity(n);
        for i in 0..n {
            let vertex = if let Some(ref indices) = indices {
                vertices[indices[i] as usize]
            } else {
                vertices[i]
            };
            positions.push(vertex.position);
            data.push(vertex.data);
        }
        Self { positions, data }
    }
}
