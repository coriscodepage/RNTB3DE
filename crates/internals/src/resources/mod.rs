use crate::input::{InputMap, Inputstate};

pub struct Resources<A>
where
    A: std::hash::Hash + std::cmp::Eq + Copy,
{
    pub input_state: Inputstate,
    pub input_map: InputMap<A>,
    pub dt: usize,
}

impl<A> Resources<A>
where
    A: std::hash::Hash + std::cmp::Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            input_state: Inputstate::new(),
            input_map: InputMap::<A>::new(),
            dt: 0,
        }
    }
}
