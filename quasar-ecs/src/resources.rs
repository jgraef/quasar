use std::fmt::Debug;

use downcast_rs::Downcast;

use crate::util::type_id_map::TypeIdMap;

pub trait Resource: 'static {}

#[derive(Default)]
pub struct Resources {
    resources: TypeIdMap<Box<dyn Resource>>,
}

impl Resources {
    pub fn insert<R: Resource>(&mut self, resource: R) -> &mut R {
        let resource = self
            .resources
            .entry::<R>()
            .insert_entry(Box::new(resource))
            .into_mut();
        resource.as_any_mut().downcast_mut().unwrap()
    }

    pub fn get<R: Resource>(&self) -> Option<&R> {
        Some(self.resources.get::<R>()?.as_any().downcast_ref().unwrap())
    }

    pub fn get_mut<R: Resource>(&mut self) -> Option<&mut R> {
        Some(
            self.resources
                .get_mut::<R>()?
                .as_any_mut()
                .downcast_mut()
                .unwrap(),
        )
    }

    pub fn get_mut_or_insert_with<R: Resource>(&mut self, default: impl FnOnce() -> R) -> &mut R {
        self.resources
            .entry::<R>()
            .or_insert_with(|| Box::new(default()))
            .as_any_mut()
            .downcast_mut()
            .unwrap()
    }

    pub fn get_mut_or_insert_default<R: Resource + Default>(&mut self) -> &mut R {
        self.get_mut_or_insert_with(Default::default)
    }

    pub fn clear(&mut self) {
        self.resources.clear();
    }
}

impl Debug for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resources").finish_non_exhaustive()
    }
}
