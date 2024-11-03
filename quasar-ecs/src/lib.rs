mod archetype;
mod bundle;
mod command;
mod component;
mod entity;
mod resources;
mod storage;
mod util;
mod world;

extern crate alloc;

// hack to get the proc-macro working from this crate
extern crate self as quasar_ecs;

pub use crate::{
    bundle::Bundle,
    component::Component,
    storage::StorageType,
    world::{
        EntityIter,
        EntityMut,
        EntityRef,
        World,
        WorldId,
    },
};

#[doc(hidden)]
pub mod __private {
    pub use crate::bundle::{
        ForEachComponent,
        IntoEachComponent,
    };
}
