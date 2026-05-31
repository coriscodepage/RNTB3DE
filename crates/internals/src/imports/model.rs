use std::ops::{Add, Mul};

use glam::{Vec2, Vec3};
use renderer::{datatypes::Vertex, lerp::Lerp, mesh::Mesh};

#[derive(Debug, Clone, Copy)]
pub struct MeshData {
    pub texture_uv: Vec2,
}

impl Add<Self> for MeshData {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            texture_uv: self.texture_uv + rhs.texture_uv,
        }
    }
}

impl Mul<f32> for MeshData {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            texture_uv: self.texture_uv * rhs,
        }
    }
}

impl Lerp for MeshData {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self { texture_uv: self.texture_uv.lerp(other.texture_uv, t) }
    }
}

pub struct Model<T: Lerp> {
    pub mesh: Mesh<T>,
}

impl Model<MeshData> {
    pub fn from_obj_string(input: &str) -> Self {
        let mut vertices = Vec::new();
        let mut texture_vertices = Vec::new();
        let mut indices = Vec::new();
        let mut texture_indices = Vec::new();
        for line in input.lines() {
            let mut split = line.split_whitespace();
            match split.next() {
                Some(c) if c == "v" => {
                    let x = split.next().unwrap().parse::<f32>().unwrap();
                    let y = split.next().unwrap().parse::<f32>().unwrap();
                    let z = -split.next().unwrap().parse::<f32>().unwrap();
                    vertices.push(Vec3::new(x, y, z));
                }
                Some(c) if c == "f" => {
                    // We have to reverse tri0 with tri2 because otherwice it was CCW and culled.
                    let mut tri2 = split.next().unwrap().split('/');
                    let mut tri1 = split.next().unwrap().split('/');
                    let mut tri0 = split.next().unwrap().split('/');
                    indices.push(tri0.next().unwrap().parse::<u32>().unwrap() - 1);
                    indices.push(tri1.next().unwrap().parse::<u32>().unwrap() - 1);
                    indices.push(tri2.next().unwrap().parse::<u32>().unwrap() - 1);
                    texture_indices.push(tri0.next().unwrap().parse::<u32>().unwrap() - 1);
                    texture_indices.push(tri1.next().unwrap().parse::<u32>().unwrap() - 1);
                    texture_indices.push(tri2.next().unwrap().parse::<u32>().unwrap() - 1);
                }
                Some(c) if c == "vt" => {
                    let u: f32 = split.next().unwrap().parse().unwrap();
                    let v: f32 = split.next().unwrap().parse().unwrap();
                    texture_vertices.push(Vec2::new(u, 1.0 - v));
                }
                _ => {}
            }
        }
        let verts = indices
            .iter()
            .zip(texture_indices.iter())
            .map(|(v, t)| {
                Vertex::new(
                    vertices[*v as usize],
                    MeshData {
                        texture_uv: texture_vertices[*t as usize],
                    },
                )
            })
            .collect::<Vec<_>>();
        Self {
            mesh: Mesh::new(verts, None),
        }
    }
}
