use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
};

use crate::{
    component::{self, ComponentStore, Entity, HashMapSet, SparseSet},
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

/// Which backing storage to use for a component type.
pub enum ComponentStorageKind {
    Sparse,
    HashMap,
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
    /// Returns `false` if the component type already exists.
    pub fn add<T: Component>(&mut self) -> bool {
        let key = TypeId::of::<T>();
        let set = SparseSet::<T>::new(self.size);
        if self.map.contains_key(&key) {
            return false;
        }
        self.map.insert(key, Box::new(set));
        debug_assert!(self.map.contains_key(&key), "Component not added to World2");
        true
    }

    /// Adds a component type choosing storage backend.
    pub fn add_with_storage<T: Component>(&mut self, kind: ComponentStorageKind) -> bool {
        let key = TypeId::of::<T>();
        if self.map.contains_key(&key) {
            return false;
        }
        match kind {
            ComponentStorageKind::Sparse => {
                self.map
                    .insert(key, Box::new(SparseSet::<T>::new(self.size)));
            }
            ComponentStorageKind::HashMap => {
                self.map.insert(key, Box::new(HashMapSet::<T>::new()));
            }
        }
        true
    }

    /// Returns an iterator over the component SparseSet, or empty if not present.
    pub fn iter<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.get::<T>().into_iter().flat_map(|set| set.iter())
    }

    /// Returns an iterator over the component SparseSet, or empty if not present.
    pub fn iter_mut<T: Component>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.get_mut::<T>()
            .into_iter()
            .flat_map(|set| set.iter_mut())
    }

    /// Retrieves a `SparseSet` for the component type from the world, if present.
    pub fn get_sparse<T: Component>(&self) -> Option<&SparseSet<T>> {
        let key = TypeId::of::<T>();
        let comp = self.map.get(&key);

        comp?.downcast_ref::<SparseSet<T>>()
    }

    /// Retrieves a `SparseSet` for the component type from the world, if present.
    pub fn get_sparse_mut<T: Component>(&mut self) -> Option<&mut SparseSet<T>> {
        let comp = self.map.get_mut(&TypeId::of::<T>());

        comp.as_ref()?;
        let comp = comp.unwrap();
        comp.downcast_mut::<SparseSet<T>>()
    }

    /// Try to get a HashMapSet for the component type.
    pub fn get_hashmap<T: Component>(&self) -> Option<&HashMapSet<T>> {
        self.map
            .get(&TypeId::of::<T>())?
            .downcast_ref::<HashMapSet<T>>()
    }

    /// Mutable access to HashMapSet storage if used.
    pub fn get_hashmap_mut<T: Component>(&mut self) -> Option<&mut HashMapSet<T>> {
        let comp = self.map.get_mut(&TypeId::of::<T>());
        comp.as_ref()?;
        comp.unwrap().downcast_mut::<HashMapSet<T>>()
    }

    /// Returns a dynamic trait object to the component storage, regardless of backend.
    pub fn get<T: Component>(&self) -> Option<&dyn ComponentStore<T>> {
        let any = self.map.get(&TypeId::of::<T>())?;
        // Try sparse first then hashmap
        if let Some(s) = any.downcast_ref::<SparseSet<T>>() {
            return Some(s as &dyn ComponentStore<T>);
        }
        if let Some(h) = any.downcast_ref::<HashMapSet<T>>() {
            return Some(h as &dyn ComponentStore<T>);
        }
        None
    }

    /// Mutable variant of `get_store`.
    pub fn get_mut<T: Component>(&mut self) -> Option<&mut dyn ComponentStore<T>> {
        let any = self.map.get_mut(&TypeId::of::<T>())?;
        // We can attempt downcast in sequence without re-borrowing by using raw pointer casts.
        if any.is::<SparseSet<T>>() {
            let ptr = any.downcast_mut::<SparseSet<T>>().unwrap();
            return Some(ptr as &mut dyn ComponentStore<T>);
        }
        if any.is::<HashMapSet<T>>() {
            let ptr = any.downcast_mut::<HashMapSet<T>>().unwrap();
            return Some(ptr as &mut dyn ComponentStore<T>);
        }
        None
    }
}

pub trait Component: Sync + Send + 'static + Sized + Copy + Clone {}

macro_rules! impl_get_mut {
    ($name:ident, $( $ty:ident ),+) => {
        pub fn $name<$($ty: Component),+>(
            &mut self
        ) -> ( $( Option<&mut SparseSet<$ty>> ),+ ) {
            let keys = [ $( &TypeId::of::<$ty>() ),+ ];
            let slots = self.map.get_disjoint_mut(keys);

            // zip the slots with the types in order
            let mut it = slots.into_iter();
            (
                $(
                    it.next().unwrap()
                        .and_then(|s| s.downcast_mut::<SparseSet<$ty>>()),
                )+
            )
        }
    };
}

impl World {
    // Distinct generic identifiers for each arity
    impl_get_mut!(get_two_mut, A, B);
    impl_get_mut!(get_three_mut, A, B, C);
    impl_get_mut!(get_four_mut, A, B, C, D);
    impl_get_mut!(get_five_mut, A, B, C, D, E);
    impl_get_mut!(get_six_mut, A, B, C, D, E, F);
}

pub trait FetchMut<'a> {
    type Output;
    fn fetch(world: &'a mut World) -> Option<Self::Output>;
}

impl<'a, A: Component> FetchMut<'a> for (A,) {
    type Output = &'a mut dyn ComponentStore<A>;
    fn fetch(world: &'a mut World) -> Option<Self::Output> {
        world.get_mut::<A>()
    }
}

impl<'a, A: Component, B: Component> FetchMut<'a> for (A, B) {
    type Output = (&'a mut dyn ComponentStore<A>, &'a mut dyn ComponentStore<B>);
    fn fetch(world: &'a mut World) -> Option<Self::Output> {
        let (a, b) = world.get_two_mut::<A, B>();
        Some((a?, b?))
    }
}

impl<'a, A: Component, B: Component, C: Component> FetchMut<'a> for (A, B, C) {
    type Output = (
        &'a mut dyn ComponentStore<A>,
        &'a mut dyn ComponentStore<B>,
        &'a mut dyn ComponentStore<C>,
    );
    fn fetch(world: &'a mut World) -> Option<Self::Output> {
        let (a, b, c) = world.get_three_mut::<A, B, C>();
        Some((a?, b?, c?))
    }
}

impl<'a, A: Component, B: Component, C: Component, D: Component> FetchMut<'a> for (A, B, C, D) {
    type Output = (
        &'a mut dyn ComponentStore<A>,
        &'a mut dyn ComponentStore<B>,
        &'a mut dyn ComponentStore<C>,
        &'a mut dyn ComponentStore<D>,
    );
    fn fetch(world: &'a mut World) -> Option<Self::Output> {
        let (a, b, c, d) = world.get_four_mut::<A, B, C, D>();
        Some((a?, b?, c?, d?))
    }
}

impl<'a, A: Component, B: Component, C: Component, D: Component, E: Component> FetchMut<'a>
    for (A, B, C, D, E)
{
    type Output = (
        &'a mut dyn ComponentStore<A>,
        &'a mut dyn ComponentStore<B>,
        &'a mut dyn ComponentStore<C>,
        &'a mut dyn ComponentStore<D>,
        &'a mut dyn ComponentStore<E>,
    );
    fn fetch(world: &'a mut World) -> Option<Self::Output> {
        let (a, b, c, d, e) = world.get_five_mut::<A, B, C, D, E>();
        Some((a?, b?, c?, d?, e?))
    }
}

impl<'a, A: Component, B: Component, C: Component, D: Component, E: Component, F: Component>
    FetchMut<'a> for (A, B, C, D, E, F)
{
    type Output = (
        &'a mut dyn ComponentStore<A>,
        &'a mut dyn ComponentStore<B>,
        &'a mut dyn ComponentStore<C>,
        &'a mut dyn ComponentStore<D>,
        &'a mut dyn ComponentStore<E>,
        &'a mut dyn ComponentStore<F>,
    );
    fn fetch(world: &'a mut World) -> Option<Self::Output> {
        let (a, b, c, d, e, f) = world.get_six_mut::<A, B, C, D, E, F>();
        Some((a?, b?, c?, d?, e?, f?))
    }
}

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

    #[derive(Copy, Clone)]
    struct Third;
    impl super::Component for Third {}

    #[derive(Copy, Clone)]
    struct Fourth;
    impl super::Component for Fourth {}

    #[derive(Copy, Clone)]
    struct Fifth;
    impl super::Component for Fifth {}

    #[derive(Copy, Clone)]
    struct Sixth;
    impl super::Component for Sixth {}

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

    #[test]
    fn test_fetchmut_single() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        let comp_opt = <(MyComponent,) as super::FetchMut>::fetch(&mut world);
        assert!(comp_opt.is_some());
    }

    #[test]
    fn test_fetchmut_double() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        world.add::<Other>();
        let fetched = <(MyComponent, Other) as super::FetchMut>::fetch(&mut world);
        assert!(fetched.is_some());
    }

    #[test]
    fn test_fetchmut_triple() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        let fetched = <(MyComponent, Other, Third) as super::FetchMut>::fetch(&mut world);
        assert!(fetched.is_some());
    }

    #[test]
    fn test_fetchmut_quad() {
        let mut world = super::World::new(6);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        world.add::<Fourth>();
        let fetched = <(MyComponent, Other, Third, Fourth) as super::FetchMut>::fetch(&mut world);
        assert!(fetched.is_some());
    }

    #[test]
    fn test_fetchmut_five() {
        let mut world = super::World::new(6);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        world.add::<Fourth>();
        world.add::<Fifth>();
        let fetched =
            <(MyComponent, Other, Third, Fourth, Fifth) as super::FetchMut>::fetch(&mut world);
        assert!(fetched.is_some());
    }

    #[test]
    fn test_fetchmut_six() {
        let mut world = super::World::new(6);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        world.add::<Fourth>();
        world.add::<Fifth>();
        world.add::<Sixth>();
        let fetched = <(MyComponent, Other, Third, Fourth, Fifth, Sixth) as super::FetchMut>::fetch(
            &mut world,
        );
        assert!(fetched.is_some());
    }

    #[test]
    fn test_entity_id_reuse() {
        let mut world = super::World::new(5);

        // Spawn first entity
        let entity1 = world.spawn();
        let first_id = entity1.0;

        // Spawn second entity
        let entity2 = world.spawn();
        let second_id = entity2.0;

        // Despawn first entity
        world.despawn(entity1);

        // Spawn third entity - should reuse first entity's ID
        let entity3 = world.spawn();
        let third_id = entity3.0;

        assert_eq!(
            first_id, third_id,
            "Entity ID should be reused after despawn"
        );
        assert_ne!(
            second_id, third_id,
            "Third entity should not have same ID as active entity"
        );
    }
}
