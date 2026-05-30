pub mod lerp;
pub mod mesh;
pub mod framebuffer;
pub mod renderer;
pub mod rasterizer;
pub mod forward_pipeline;
pub mod datatypes;
use std::{
    cmp::{max, min}, fmt::Debug, marker::PhantomData, ops::{Add, Mul}
};

use glam::{IVec2, UVec2, Vec3, Vec4};

use crate::lerp::Lerp;











