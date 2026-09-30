use crate::world::World;

pub trait System {
    fn update(&mut self, world: &mut World, dt: u32);
}

pub struct SystemHandler {
    systems: Vec<Box<dyn System>>,
}

impl SystemHandler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn update(&mut self, world: &mut World) {
        let dt = 0;
        self.systems.iter_mut().for_each(|s| s.update(world, dt));
    }

    pub fn register<S: System + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }
}
