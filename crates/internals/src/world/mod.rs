use hecs::Entity;
use renderer::lerp::Lerp;

use crate::{
    imports::model::{MeshData, Model}, world::{material::{Material, MaterialHandle}, transform::Transform},
};
use std::fmt::Debug;
pub mod material;
pub mod transform;

pub struct World {
    world: hecs::World,
}

impl World {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
        }
    }

    pub fn place_model_with_transform(
        &mut self,
        model: Model<MeshData>,
        transform: Transform,
        material: MaterialHandle,
    ) -> Entity {
        self.world.spawn((model, transform, material))
    }

    pub fn with_world<F: FnOnce(&hecs::World) -> R, R>(&self, f: F) -> R {
        f(&self.world)
    }
}
