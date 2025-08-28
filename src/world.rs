use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
};

use crate::{
    component::{self, Entity, SparseSet},
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


    /// Returns an iterator over the component SparseSet, or empty if not present.
    pub fn iter<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.get::<T>()
            .into_iter()  
            .flat_map(|set| set.iter())
    }

    /// Returns an iterator over the component SparseSet, or empty if not present.
    pub fn iter_mut<T: Component>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.get_mut::<T>()
            .into_iter()  
            .flat_map(|set| set.iter_mut())
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

pub trait QueryMut<'a> {
    type Output;
    fn query(world: &'a mut World) -> Self::Output;
}

impl<'a, A: Component> QueryMut<'a> for (A,) {
    type Output = Option<&'a mut SparseSet<A>>;
    fn query(world: &'a mut World) -> Self::Output {
        world.get_mut::<A>()
    }
}

impl<'a, A: Component, B: Component> QueryMut<'a> for (A, B) {
    type Output = (Option<&'a mut SparseSet<A>>, Option<&'a mut SparseSet<B>>);
    fn query(world: &'a mut World) -> Self::Output {
        world.get_two_mut::<A, B>()
    }
}

impl<'a, A: Component, B: Component, C: Component> QueryMut<'a> for (A, B, C) {
    type Output = (
        Option<&'a mut SparseSet<A>>,
        Option<&'a mut SparseSet<B>>,
        Option<&'a mut SparseSet<C>>,
    );
    fn query(world: &'a mut World) -> Self::Output {
        world.get_three_mut::<A, B, C>()
    }
}

impl<'a, A: Component, B: Component, C: Component, D: Component> QueryMut<'a>
    for (A, B, C, D)
{
    type Output = (
        Option<&'a mut SparseSet<A>>,
        Option<&'a mut SparseSet<B>>,
        Option<&'a mut SparseSet<C>>,
        Option<&'a mut SparseSet<D>>,
    );
    fn query(world: &'a mut World) -> Self::Output {
        world.get_four_mut::<A, B, C, D>()
    }
}

impl<'a, A: Component, B: Component, C: Component, D: Component, E: Component> QueryMut<'a>
    for (A, B, C, D, E)
{
    type Output = (
        Option<&'a mut SparseSet<A>>,
        Option<&'a mut SparseSet<B>>,
        Option<&'a mut SparseSet<C>>,
        Option<&'a mut SparseSet<D>>,
        Option<&'a mut SparseSet<E>>,
    );
    fn query(world: &'a mut World) -> Self::Output {
        world.get_five_mut::<A, B, C, D, E>()
    }
}

impl<'a, A: Component, B: Component, C: Component, D: Component, E: Component, F: Component>
    QueryMut<'a> for (A, B, C, D, E, F)
{
    type Output = (
        Option<&'a mut SparseSet<A>>,
        Option<&'a mut SparseSet<B>>,
        Option<&'a mut SparseSet<C>>,
        Option<&'a mut SparseSet<D>>,
        Option<&'a mut SparseSet<E>>,
        Option<&'a mut SparseSet<F>>,
    );
    fn query(world: &'a mut World) -> Self::Output {
        world.get_six_mut::<A, B, C, D, E, F>()
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
    fn test_querymut_single() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        let comp_opt = <(MyComponent,) as super::QueryMut>::query(&mut world);
        assert!(comp_opt.is_some());
    }

    #[test]
    fn test_querymut_double() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        world.add::<Other>();
        let (a,b) = <(MyComponent, Other) as super::QueryMut>::query(&mut world);
        assert!(a.is_some() && b.is_some());
    }

    #[test]
    fn test_querymut_triple() {
        let mut world = super::World::new(5);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        let (a,b,c) = <(MyComponent, Other, Third) as super::QueryMut>::query(&mut world);
        assert!(a.is_some() && b.is_some() && c.is_some());
    }

    #[test]
    fn test_querymut_quad() {
        let mut world = super::World::new(6);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        world.add::<Fourth>();
        let (a,b,c,d) = <(MyComponent, Other, Third, Fourth) as super::QueryMut>::query(&mut world);
        assert!(a.is_some() && b.is_some() && c.is_some() && d.is_some());
    }

    #[test]
    fn test_querymut_five() {
        let mut world = super::World::new(6);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        world.add::<Fourth>();
        world.add::<Fifth>();
        let (a,b,c,d,e) = <(MyComponent, Other, Third, Fourth, Fifth) as super::QueryMut>::query(&mut world);
        assert!(a.is_some() && b.is_some() && c.is_some() && d.is_some() && e.is_some());
    }

    #[test]
    fn test_querymut_six() {
        let mut world = super::World::new(6);
        world.add::<MyComponent>();
        world.add::<Other>();
        world.add::<Third>();
        world.add::<Fourth>();
        world.add::<Fifth>();
        world.add::<Sixth>();
        let (a,b,c,d,e,f) = <(MyComponent, Other, Third, Fourth, Fifth, Sixth) as super::QueryMut>::query(&mut world);
        assert!(a.is_some() && b.is_some() && c.is_some() && d.is_some() && e.is_some() && f.is_some());
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
