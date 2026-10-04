use std::mem;

#[derive(Default)]
pub struct Events<T: Clone> {
    prev: Vec<T>,
    curr: Vec<T>,
}

impl<T> Events<T>
where
    T: Clone,
{
    pub fn send(&mut self, event: T) {
        self.curr.push(event);
    }

    pub fn read(&self) -> &[T] {
        &self.prev
    }

    pub fn swap(&mut self) {
        self.prev.clear();
        mem::swap(&mut self.prev, &mut self.curr);
    }
}
