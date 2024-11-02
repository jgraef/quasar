use crate::world::World;

pub trait Command: 'static {
    fn apply(self, world: &mut World);
}
