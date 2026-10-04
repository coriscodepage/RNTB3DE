use crate::{resources::Resources, world::World};

pub trait System<A: std::hash::Hash + std::cmp::Eq + Copy> {
    fn update(&mut self, world: &mut World, resources: &Resources<A>);
}

pub struct SystemHandler<A: std::hash::Hash + std::cmp::Eq + Copy> {
    systems: Vec<Box<dyn System<A>>>,
}

impl<A> SystemHandler<A>
where
    A: std::hash::Hash + std::cmp::Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn update(&mut self, world: &mut World, resources: &mut Resources<A>) {
        self.systems
            .iter_mut()
            .for_each(|s: &mut Box<dyn System<A> + 'static>| s.update(world, resources));
    }

    pub fn register<S: System<A> + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }
}
