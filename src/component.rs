// Sparse set component storage for the ecs

use std::{collections::HashMap, str::FromStr};

use std::fmt::Debug;

/// Represents a unique entity in the ECS.
/// Wraps a usize ID.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity(pub usize);

impl Entity {
    /// Szudzik pairing function to combine two entities into a single unique key.
    pub fn combine_key(self, other: Entity) -> usize {
        let a = self.0;
        let b = other.0;
        if a >= b { a * a + a + b } else { a + b * b }
    }
}

impl FromStr for Entity {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<usize>()
            .map(Entity)
            .map_err(|_| "Invalid entity string")
    }
}

/// Stores component data for entities using a sparse set.
/// - `sparse`: Maps entity IDs to indices in the dense array.
/// - `dense`: Stores the actual component data.
/// - `entities`: Stores the entity IDs corresponding to each dense index.
#[derive(Clone)]
pub struct SparseSet<T: Send + Sync + Copy + Clone> {
    // Tracks every entity that has been added to this sparseset
    pub added: Vec<Entity>,
    // Tracks every entity that has been removed from this sparseset
    pub removed: Vec<Entity>,
    sparse: Vec<Option<usize>>,
    dense: Vec<T>,
    entities: Vec<usize>,
}

impl<T> SparseSet<T>
where
    T: Send + Sync + Sized + Copy + Clone,
{
    /// Creates sparse set storage with the given max entity count.
    pub fn new(entity_count: usize) -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            sparse: vec![None; entity_count],
            dense: Vec::new(),
            entities: Vec::new(),
        }
    }

    /// Sets the data for the given entity, replacing any existing data.
    /// If the entity does not exist, it will be added.
    pub fn set(&mut self, data: T, entity: Entity) {
        if let Some(idx) = self.sparse[entity.0] {
            // Update existing entity
            self.dense[idx] = data;
        } else {
            // Add new entity
            self.add_entity(data, entity);
        }
    }

    /// Adds a new entity with the given component data.
    /// Panics if the entity already exists in this component.
    pub fn add_entity(&mut self, data: T, entity: Entity) {
        assert_eq!(self.sparse[entity.0], None);
        self.sparse[entity.0] = Some(self.dense.len());
        self.dense.push(data);
        self.entities.push(entity.0);
        self.added.push(entity);
    }

    /// Removes an entity and returns its component data, if present.
    pub fn remove_entity(&mut self, entity: Entity) -> Option<T> {
        match self.sparse[entity.0] {
            Some(idx) => {
                self.sparse[entity.0] = None;
                let last = self.dense.len() - 1;
                self.entities.swap_remove(idx);
                let removed = self.dense.swap_remove(idx);
                if idx != last {
                    // Update sparse for the entity that was moved
                    let moved_entity = self.entities[idx];
                    self.sparse[moved_entity] = Some(idx);
                }
                self.removed.push(entity);
                Some(removed)
            }
            None => None,
        }
    }

    /// Gets a reference to the component data for the given entity.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        match self.sparse[entity.0] {
            Some(idx) => Some(&self.dense[idx]),
            None => None,
        }
    }
    /// Gets a mutable reference to the component data for the given entity.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        match self.sparse[entity.0] {
            Some(idx) => Some(&mut self.dense[idx]),
            None => None,
        }
    }

    /// Returns true if the component contains data for the given entity.
    pub fn has(&self, entity: Entity) -> bool {
        self.sparse[entity.0].is_some()
    }

    /// Returns the number of entities with this component.
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Uses unsafe to iterate the ECS a bit faster.
    pub fn iter_unchecked(&self) -> impl Iterator<Item = (Entity, &T)> {
        // Safety: `entities` and `dense` are always the same length
        debug_assert_eq!(self.entities.len(), self.dense.len());
        unsafe {
            let entities_ptr = self.entities.as_ptr();
            let dense_ptr = self.dense.as_ptr();
            let len = self.entities.len();

            (0..len).map(move |i| (Entity(*entities_ptr.add(i)), &*dense_ptr.add(i)))
        }
    }

    /// Uses unsafe to iterate the ECS a bit faster (mutable ref to the component data).
    pub fn iter_mut_unchecked(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        // Safety: `entities` and `dense` are always the same length
        debug_assert_eq!(self.entities.len(), self.dense.len());
        unsafe {
            let entities_ptr = self.entities.as_ptr();
            let dense_ptr = self.dense.as_mut_ptr();
            let len = self.entities.len();

            (0..len).map(move |i| (Entity(*entities_ptr.add(i)), &mut *dense_ptr.add(i)))
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.entities
            .iter()
            .copied()
            .zip(self.dense.iter())
            .map(|(id, data)| (Entity(id), data))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.entities
            .iter()
            .copied()
            .zip(self.dense.iter_mut())
            .map(|(id, data)| (Entity(id), data))
    }

    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.entities.iter().map(|&id| Entity(id))
    }
}

/// HashMap based storage for very sparse components (e.g. only a handful of entities use it).
/// This avoids allocating a sparse array sized to the whole entity capacity.
#[derive(Clone, Default)]
pub struct HashMapSet<T: Send + Sync + Copy + Clone> {
    pub added: Vec<Entity>,
    pub removed: Vec<Entity>,
    map: HashMap<usize, T>,
}

impl<T> HashMapSet<T>
where
    T: Send + Sync + Sized + Copy + Clone,
{
    /// Creates empty map storage.
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            map: HashMap::new(),
        }
    }

    /// Sets or inserts the data for an entity.
    pub fn set(&mut self, data: T, entity: Entity) {
        if !self.map.contains_key(&entity.0) {
            self.added.push(entity);
        }
        self.map.insert(entity.0, data);
    }

    /// Adds a new entity with the given component data. Panics if already present.
    pub fn add_entity(&mut self, data: T, entity: Entity) {
        assert!(!self.map.contains_key(&entity.0));
        self.map.insert(entity.0, data);
        self.added.push(entity);
    }

    /// Removes an entity returning its data.
    pub fn remove_entity(&mut self, entity: Entity) -> Option<T> {
        let removed = self.map.remove(&entity.0);
        if removed.is_some() {
            self.removed.push(entity);
        }
        removed
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.map.get(&entity.0)
    }
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.map.get_mut(&entity.0)
    }
    pub fn has(&self, entity: Entity) -> bool {
        self.map.contains_key(&entity.0)
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.map.iter().map(|(id, v)| (Entity(*id), v))
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.map.iter_mut().map(|(id, v)| (Entity(*id), v))
    }
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.map.keys().copied().map(Entity)
    }
}

/// Trait abstraction over component storage backends (SparseSet / HashMapSet).
pub trait ComponentStore<T: Send + Sync + Sized + Copy + Clone> {
    fn set(&mut self, data: T, entity: Entity);
    fn add_entity(&mut self, data: T, entity: Entity);
    fn remove_entity(&mut self, entity: Entity) -> Option<T>;
    fn get(&self, entity: Entity) -> Option<&T>;
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T>;
    fn has(&self, entity: Entity) -> bool;
    fn len(&self) -> usize;
    fn iter(&self) -> Box<dyn Iterator<Item = (Entity, &T)> + '_>;
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (Entity, &mut T)> + '_>;
}

impl<T: Send + Sync + Sized + Copy + Clone> ComponentStore<T> for SparseSet<T> {
    fn set(&mut self, data: T, entity: Entity) {
        Self::set(self, data, entity);
    }
    fn add_entity(&mut self, data: T, entity: Entity) {
        Self::add_entity(self, data, entity);
    }
    fn remove_entity(&mut self, entity: Entity) -> Option<T> {
        Self::remove_entity(self, entity)
    }
    fn get(&self, entity: Entity) -> Option<&T> {
        Self::get(self, entity)
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        Self::get_mut(self, entity)
    }
    fn has(&self, entity: Entity) -> bool {
        Self::has(self, entity)
    }
    fn len(&self) -> usize {
        Self::len(self)
    }
    /// Iterates this component's storage. Slower than fetching the concrete component type and iterating it directly.
    fn iter(&self) -> Box<dyn Iterator<Item = (Entity, &T)> + '_> {
        Box::new(SparseSet::iter(self))
    }
    /// Iterates this component's storage. Slower than fetching the concrete component type and iterating it directly.
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (Entity, &mut T)> + '_> {
        Box::new(SparseSet::iter_mut(self))
    }
}

impl<T: Send + Sync + Sized + Copy + Clone> ComponentStore<T> for HashMapSet<T> {
    fn set(&mut self, data: T, entity: Entity) {
        HashMapSet::set(self, data, entity);
    }
    fn add_entity(&mut self, data: T, entity: Entity) {
        HashMapSet::add_entity(self, data, entity);
    }
    fn remove_entity(&mut self, entity: Entity) -> Option<T> {
        HashMapSet::remove_entity(self, entity)
    }
    fn get(&self, entity: Entity) -> Option<&T> {
        HashMapSet::get(self, entity)
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        HashMapSet::get_mut(self, entity)
    }
    fn has(&self, entity: Entity) -> bool {
        HashMapSet::has(self, entity)
    }
    fn len(&self) -> usize {
        HashMapSet::len(self)
    }

    /// Iterates this component's storage. Slower than fetching the concrete component type and iterating it directly.
    fn iter(&self) -> Box<dyn Iterator<Item = (Entity, &T)> + '_> {
        Box::new(HashMapSet::iter(self))
    }
    /// Iterates this component's storage. Slower than fetching the concrete component type and iterating it directly.
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (Entity, &mut T)> + '_> {
        Box::new(HashMapSet::iter_mut(self))
    }
}

/// Attempts to get a reference to a component. If not found, executes the fallback block.
/// Usage: and!(components, entity, comp, { continue; });
#[macro_export]
macro_rules! ecs_and {
    ($collection:expr, $key:expr, $var:ident, $fallback:block) => {
        let Some($var) = $collection.get($key) else $fallback;
    };
}

/// Attempts to get a mutable reference to a component. If not found, executes the fallback block.
#[macro_export]
macro_rules! ecs_and_mut {
    ($collection:expr, $key:expr, $var:ident, $fallback:block) => {
        let Some($var) = $collection.get_mut($key) else $fallback;
    };
}

/// Checks if a component exists for an entity. If not, executes the fallback block.
#[macro_export]
macro_rules! ecs_has {
    ($collection:expr, $key:expr, $fallback:block) => {
        if !$collection.has($key) $fallback
    };
}

#[allow(unused)]
#[cfg(test)]
mod tests {

    use std::{time::Instant, vec};

    use super::*;

    #[derive(Default, Copy, Clone)]
    struct Vec2 {
        x: i32,
        y: i32,
    }

    #[derive(Default)]
    struct SomethingElse(i32);

    #[test]
    fn joining() {
        let mut positions = SparseSet::<Vec2>::new(100);
        let mut velocities = SparseSet::<Vec2>::new(100);
        let mut colors = SparseSet::<u32>::new(100);
        positions.add_entity(Vec2 { x: 25, y: 35 }, Entity(0));
        positions.add_entity(Vec2 { x: 25, y: 35 }, Entity(1));
        positions.add_entity(Vec2 { x: 25, y: 35 }, Entity(6));
        positions.add_entity(Vec2 { x: 25, y: 35 }, Entity(4));
        velocities.add_entity(Vec2 { x: 1, y: 1 }, Entity(1));
        velocities.add_entity(Vec2 { x: 1, y: 1 }, Entity(6));
        colors.add_entity(100, Entity(6));

        let mut found = Vec::<Entity>::new();

        for (entity, _position) in positions.iter() {
            ecs_and_mut!(velocities, entity, _velocity, {
                continue;
            });
            ecs_has!(colors, entity, {
                continue;
            });
            found.push(entity);
        }

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, 6);
    }

    #[test]
    fn test_iter() {
        let mut component = SparseSet::<u32>::new(5);
        for i in 0..5 {
            component.add_entity(i, Entity(i.try_into().unwrap()));
        }
        for (_entity, data) in component.iter_mut() {
            *data = 5;
        }
        for (_entity, data) in component.iter() {
            assert_eq!(*data, 5);
        }
    }

    #[test]
    fn test_iter_big_safe() {
        let mut component = SparseSet::<u32>::new(1000);
        for i in 0..1000 {
            component.add_entity(i, Entity(i.try_into().unwrap()));
        }
        let i = Instant::now();
        for (_entity, data) in component.iter_mut() {
            *data = 5;
        }
        println!("mutation: {:?}", i.elapsed());
        let i = Instant::now();
        for (_entity, data) in component.iter() {
            assert_eq!(*data, 5);
        }
        println!("iteration: {:?}", i.elapsed());
    }

    #[test]
    fn test_iter_big_unsafe() {
        let mut component = SparseSet::<u32>::new(1000);
        for i in 0..1000 {
            component.add_entity(i, Entity(i.try_into().unwrap()));
        }
        let i = Instant::now();
        for (_entity, data) in component.iter_mut_unchecked() {
            *data = 5;
        }
        println!("mutation: {:?}", i.elapsed());
        let i = Instant::now();
        for (_entity, data) in component.iter_unchecked() {
            assert_eq!(*data, 5);
        }
        println!("iteration: {:?}", i.elapsed());
    }

    #[test]
    fn test_add_remove() {
        let mut component = SparseSet::<usize>::new(3);
        component.add_entity(1, Entity(0));
        component.add_entity(2, Entity(1));
        component.add_entity(3, Entity(2));
        let removed = component.remove_entity(Entity(1));
        assert_eq!(removed, Some(2));
        let c = component.get(Entity(2));
        assert_eq!(c, Some(&3));
        let removed_c = component.remove_entity(Entity(2));
        assert_eq!(removed_c, Some(3));
        assert_eq!(component.get(Entity(2)), None);
    }

    #[test]
    fn test_mutation() {
        let mut component = SparseSet::<u32>::new(5);
        let data1 = 10;
        let updated = 6;
        let data2 = 5;
        component.add_entity(data1, Entity(0));
        component.add_entity(data2, Entity(1));
        let data = component.get_mut(Entity(0)).unwrap();
        *data = updated;
        assert_eq!(*component.get(Entity(0)).unwrap(), updated);
        assert_eq!(*component.get(Entity(1)).unwrap(), data2);
    }
    #[test]
    fn test_key_pairing() {
        let entity1 = Entity(1);
        let entity2 = Entity(2);
        let combined_key = entity1.combine_key(entity2);
        let entity3 = Entity(combined_key);
        assert_ne!(entity3.combine_key(entity1), combined_key);
    }

    #[test]
    fn test_added_removed_tracking() {
        let mut component = SparseSet::<u32>::new(5);

        // Initially, both vectors should be empty
        assert!(component.added.is_empty());
        assert!(component.removed.is_empty());

        // Add some entities
        component.add_entity(10, Entity(0));
        component.add_entity(20, Entity(1));
        component.add_entity(30, Entity(2));

        // Check that added entities are tracked
        assert_eq!(component.added.len(), 3);
        assert!(component.added.contains(&Entity(0)));
        assert!(component.added.contains(&Entity(1)));
        assert!(component.added.contains(&Entity(2)));
        assert!(component.removed.is_empty());

        // Remove an entity
        let removed_data = component.remove_entity(Entity(1));
        assert_eq!(removed_data, Some(20));

        // Check that removed entity is tracked
        assert_eq!(component.removed.len(), 1);
        assert!(component.removed.contains(&Entity(1)));
        assert_eq!(component.added.len(), 3); // Added vector should remain unchanged

        // Remove another entity
        component.remove_entity(Entity(2));

        // Check that both removed entities are tracked
        assert_eq!(component.removed.len(), 2);
        assert!(component.removed.contains(&Entity(1)));
        assert!(component.removed.contains(&Entity(2)));

        // Try to remove non-existent entity
        let not_removed = component.remove_entity(Entity(3));
        assert_eq!(not_removed, None);

        // Removed vector should not change for non-existent entity
        assert_eq!(component.removed.len(), 2);

        // Add an entity that was previously removed
        component.add_entity(40, Entity(1));

        // Check that it's added to the added vector again
        assert_eq!(component.added.len(), 4);
        assert!(component.added.contains(&Entity(1))); // Should appear twice in added

        // Count occurrences of Entity(1) in added vector
        let entity1_count = component.added.iter().filter(|&&e| e == Entity(1)).count();
        assert_eq!(entity1_count, 2);
    }

    #[test]
    fn hashmap_basic() {
        let mut component = super::HashMapSet::<u32>::new();
        component.add_entity(10, Entity(1));
        assert_eq!(component.get(Entity(1)), Some(&10));
        component.set(15, Entity(1));
        assert_eq!(component.get(Entity(1)), Some(&15));
        let removed = component.remove_entity(Entity(1));
        assert_eq!(removed, Some(15));
        assert!(!component.has(Entity(1)));
    }

    #[test]
    fn hashmap_iter_mut() {
        let mut component = super::HashMapSet::<u32>::new();
        for i in 0..5 {
            component.add_entity(i as u32, Entity(i));
        }
        for (_e, v) in component.iter_mut() {
            *v += 1;
        }
        for (_e, v) in component.iter() {
            assert!(*v >= 1);
        }
    }

    #[test]
    fn trait_object_usage() {
        // Use trait object abstraction to manipulate either backend.
        let mut sparse: super::SparseSet<u32> = super::SparseSet::new(10);
        let mut map: super::HashMapSet<u32> = super::HashMapSet::new();
        let s_store: &mut dyn super::ComponentStore<u32> = &mut sparse;
        let m_store: &mut dyn super::ComponentStore<u32> = &mut map;
        s_store.add_entity(5, Entity(0));
        m_store.add_entity(6, Entity(1));
        assert_eq!(s_store.get(Entity(0)), Some(&5));
        assert_eq!(m_store.get(Entity(1)), Some(&6));
        for (_e, v) in s_store.iter_mut() {
            *v += 1;
        }
        for (_e, v) in m_store.iter_mut() {
            *v += 1;
        }
        assert_eq!(s_store.get(Entity(0)), Some(&6));
        assert_eq!(m_store.get(Entity(1)), Some(&7));
    }
}
