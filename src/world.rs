use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
};

use crate::{
    component::{self, SparseSet},
    tags,
};

/// Storage for components and tags, as well as basic entity management.
pub struct World {
    pub tags: tags::EntityTags,
    map: HashMap<TypeId, Box<dyn Any>>,
    entities: HashSet<usize>,
    dead_entities: HashSet<usize>,
    next_entity_id: usize,

    size: usize,
}

#[allow(dead_code)]
impl World {
    /// Creates a new world.
    pub fn new(size: usize) -> Self {
        World {
            map: HashMap::new(),
            entities: HashSet::new(),
            dead_entities: HashSet::new(),
            next_entity_id: 0,
            tags: tags::EntityTags::new(),
            size,
        }
    }

    /// Spawns a new entity.
    /// If there are dead entities, it reuses one of their IDs.
    pub fn spawn(&mut self) -> component::Entity {
        if let Some(dead_id) = self.dead_entities.iter().next().cloned() {
            self.dead_entities.remove(&dead_id);
            let entity = component::Entity(dead_id);
            self.entities.insert(dead_id);
            entity
        } else {
            self.entities.insert(self.next_entity_id);
            let entity = component::Entity(self.next_entity_id);
            self.next_entity_id += 1;
            entity
        }
    }

    /// Removes an entity from all component storage and tags.
    /// This will panic if the entity does not exist.
    /// The ID may be reused in the future.
    pub fn despawn(&mut self, entity: component::Entity) {
        if self.entities.remove(&entity.0) {
            self.dead_entities.insert(entity.0);
            self.tags.remove_all_tags(&entity);
        } else {
            panic!("attempted to despawn non-existent entity ID: {:?}", entity);
        }
    }

    /// Adds a component type to the world.
    /// This will create a new `SparseSet` for the component type.
    pub fn add<T: Component>(&mut self) {
        let key = TypeId::of::<T>();
        let set = SparseSet::<T>::new(self.size);
        if self.map.contains_key(&key) {
            todo!("not sure how to handle this")
        }
        self.map.insert(key, Box::new(set));
        assert!(self.map.contains_key(&key), "Component not added to World2");
    }

    /// Retrieves a `SparseSet` for the component type from the world, if present.
    pub fn get<T: Component>(&self) -> Option<&SparseSet<T>> {
        let key = TypeId::of::<T>();
        let comp = self.map.get(&key);

        comp?.downcast_ref::<SparseSet<T>>()
    }

    /// Retrieves a `SparseSet` for the component type from the world, if present.
    pub fn get_mut<T: Component>(&mut self) -> Option<&mut SparseSet<T>> {
        let comp = self.map.get_mut(&TypeId::of::<T>());

        comp.as_ref()?;
        let comp = comp.unwrap();
        comp.downcast_mut::<SparseSet<T>>()
    }

    /// Retrieves two `SparseSet` for the component type from the world, if present.
    pub fn get_two_mut<T: Component, K: Component>(
        &mut self,
    ) -> (Option<&mut SparseSet<T>>, Option<&mut SparseSet<K>>) {
        let [Some(a), Some(b)] = self
            .map
            .get_disjoint_mut([&TypeId::of::<T>(), &TypeId::of::<K>()])
        else {
            return (None, None);
        };
        (
            a.downcast_mut::<SparseSet<T>>(),
            b.downcast_mut::<SparseSet<K>>(),
        )
    }

    /// Retrieves three `SparseSet` for the component type from the world, if present.
    pub fn get_three_mut<T: Component, K: Component, L: Component>(
        &mut self,
    ) -> (
        Option<&mut SparseSet<T>>,
        Option<&mut SparseSet<K>>,
        Option<&mut SparseSet<L>>,
    ) {
        let [Some(a), Some(b), Some(c)] =
            self.map
                .get_disjoint_mut([&TypeId::of::<T>(), &TypeId::of::<K>(), &TypeId::of::<L>()])
        else {
            return (None, None, None);
        };
        (
            a.downcast_mut::<SparseSet<T>>(),
            b.downcast_mut::<SparseSet<K>>(),
            c.downcast_mut::<SparseSet<L>>(),
        )
    }
}

pub trait Component: Sync + Send + 'static + Sized + Copy + Clone {}

#[cfg(test)]
#[allow(dead_code)]
mod test {

    #[derive(Copy, Clone)]
    struct MyComponent {
        value: u32,
    }
    impl super::Component for MyComponent {}

    #[derive(Copy, Clone)]
    struct Other;
    impl super::Component for Other {}

    #[test]
    fn test_world2_creation() {
        let world = super::World::new(5);
        assert!(world.get::<MyComponent>().is_none());
    }

    #[test]
    fn test_world2_add_component() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        assert!(world.get::<MyComponent>().is_some());
        assert!(world.get::<Other>().is_none());
    }

    #[test]
    fn test_world2_get_mut_several() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        world.add::<Other>();

        let (my_component, other_component) = world.get_two_mut::<MyComponent, Other>();
        assert!(my_component.is_some());
        assert!(other_component.is_some());
    }
}
