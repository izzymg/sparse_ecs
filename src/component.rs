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

#[derive(Clone)]
enum SparseIndex {
    Vec(Vec<Option<usize>>),
    Map(HashMap<usize, usize>),
}

/// Unified component storage that can use either a sparse vector index or a hashmap index.
/// This allows a single concrete storage type to be used throughout the World API while
/// still choosing an indexing strategy per component type.
#[derive(Clone)]
pub struct Storage<T: Send + Sync + Copy + Clone> {
    pub added: Vec<Entity>,
    pub removed: Vec<Entity>,
    index: SparseIndex,
    dense: Vec<T>,
    entities: Vec<usize>,
}



impl<T> Storage<T>
where
    T: Send + Sync + Sized + Copy + Clone,
{
    /// Create storage backed by a sparse vector of size `entity_count`.
    pub fn new_sparse(entity_count: usize) -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            index: SparseIndex::Vec(vec![None; entity_count]),
            dense: Vec::new(),
            entities: Vec::new(),
        }
    }

    /// Create storage backed by a hashmap index.
    pub fn new_hashmap() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            index: SparseIndex::Map(HashMap::new()),
            dense: Vec::new(),
            entities: Vec::new(),
        }
    }

    /// Sets the data for the given entity, replacing any existing data.
    /// If the entity does not exist, it will be added.
    pub fn set(&mut self, data: T, entity: Entity) {
        match &mut self.index {
            SparseIndex::Vec(sparse) => match sparse[entity.0] {
                Some(idx) => self.dense[idx] = data,
                None => self.add_entity(data, entity),
            },
            SparseIndex::Map(index) => {
                if let Some(&idx) = index.get(&entity.0) {
                    self.dense[idx] = data;
                } else {
                    self.add_entity(data, entity);
                }
            }
        }
    }

    /// Adds a new entity with the given component data. Panics if the entity already exists.
    pub fn add_entity(&mut self, data: T, entity: Entity) {
        let idx = self.dense.len();
        match &mut self.index {
            SparseIndex::Vec(sparse) => {
                assert_eq!(sparse[entity.0], None);
                sparse[entity.0] = Some(idx);
            }
            SparseIndex::Map(index) => {
                assert!(!index.contains_key(&entity.0));
                index.insert(entity.0, idx);
            }
        }
        self.dense.push(data);
        self.entities.push(entity.0);
        self.added.push(entity);
    }

    /// Removes an entity and returns its component data, if present.
    pub fn remove_entity(&mut self, entity: Entity) -> Option<T> {
        let idx_opt = match &mut self.index {
            SparseIndex::Vec(sparse) => {
                let idx = sparse[entity.0]?;
                sparse[entity.0] = None;
                Some(idx)
            }
            SparseIndex::Map(index) => index.remove(&entity.0),
        };

        let idx = idx_opt?;

        let last = self.dense.len() - 1;
        self.entities.swap_remove(idx);
        let removed = self.dense.swap_remove(idx);
        if idx != last {
            // Update index for the entity that was moved
            let moved_entity = self.entities[idx];
            match &mut self.index {
                SparseIndex::Vec(sparse) => {
                    sparse[moved_entity] = Some(idx);
                }
                SparseIndex::Map(index) => {
                    index.insert(moved_entity, idx);
                }
            }
        }
        self.removed.push(entity);
        Some(removed)
    }

    /// Gets a reference to the component data for the given entity.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        match &self.index {
            SparseIndex::Vec(sparse) => Some(&self.dense[sparse[entity.0]?]),
            SparseIndex::Map(index) => Some(&self.dense[*index.get(&entity.0)?]),
        }
    }

    /// Gets a mutable reference to the component data for the given entity.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let idx = match &self.index {
            SparseIndex::Vec(sparse) => {
                 sparse[entity.0]?
            }
            SparseIndex::Map(index) => {
                 *index.get(&entity.0)?
            }
        };
        self.dense.get_mut(idx)
    }


    /// Gets a mutable reference to the component data for the given entity. Unsafe/unchecked.
    pub fn get_mut_unchecked(&mut self, entity: Entity) -> Option<&mut T> {

        let idx = match &self.index {
            SparseIndex::Vec(sparse) => {
                 sparse[entity.0]?
            }
            SparseIndex::Map(index) => {
                 *index.get(&entity.0)?
            }
        };
        // Safety: index was checked above
        unsafe { Some(self.dense.get_unchecked_mut(idx)) }
    }

    /// Returns true if the component contains data for the given entity.
    pub fn has(&self, entity: Entity) -> bool {
        match &self.index {
            SparseIndex::Vec(sparse) => sparse[entity.0].is_some(),
            SparseIndex::Map(index) => index.contains_key(&entity.0),
        }
    }

    /// Returns the number of entities with this component.
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Uses unsafe to iterate the ECS a bit faster.
    pub fn iter_unchecked(&self) -> impl Iterator<Item = (Entity, &T)> {
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
        let mut positions = Storage::<Vec2>::new_sparse(100);
        let mut velocities = Storage::<Vec2>::new_sparse(100);
        let mut colors = Storage::<u32>::new_sparse(100);
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
        let mut component = Storage::<u32>::new_sparse(5);
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
        let mut component = Storage::<u32>::new_sparse(1000);
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
        let mut component = Storage::<u32>::new_sparse(1000);
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
        let mut component = Storage::<usize>::new_sparse(3);
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
        let mut component = Storage::<u32>::new_sparse(5);
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
        let mut component = Storage::<u32>::new_sparse(5);

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
        let mut component = super::Storage::<u32>::new_hashmap();
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
        let mut component = super::Storage::<u32>::new_hashmap();
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
    fn hashmap_iter() {
        let mut component = super::Storage::<u32>::new_hashmap();
        for i in 0..5 {
            component.add_entity(i as u32, Entity(i));
        }
        for (_entity, data) in component.iter_mut() {
            *data = 5;
        }
        for (_entity, data) in component.iter() {
            assert_eq!(*data, 5);
        }
    }

    #[test]
    fn hashmap_add_remove() {
        let mut component = super::Storage::<usize>::new_hashmap();
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
    fn hashmap_mutation() {
        let mut component = super::Storage::<u32>::new_hashmap();
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
    fn hashmap_added_removed_tracking() {
        let mut component = super::Storage::<u32>::new_hashmap();

        assert!(component.added.is_empty());
        assert!(component.removed.is_empty());

        component.add_entity(10, Entity(0));
        component.add_entity(20, Entity(1));
        component.add_entity(30, Entity(2));

        assert_eq!(component.added.len(), 3);
        assert!(component.added.contains(&Entity(0)));
        assert!(component.added.contains(&Entity(1)));
        assert!(component.added.contains(&Entity(2)));
        assert!(component.removed.is_empty());

        let removed_data = component.remove_entity(Entity(1));
        assert_eq!(removed_data, Some(20));
        assert_eq!(component.removed.len(), 1);
        assert!(component.removed.contains(&Entity(1)));
        assert_eq!(component.added.len(), 3);

        component.remove_entity(Entity(2));
        assert_eq!(component.removed.len(), 2);
        assert!(component.removed.contains(&Entity(1)));
        assert!(component.removed.contains(&Entity(2)));

        let not_removed = component.remove_entity(Entity(3));
        assert_eq!(not_removed, None);
        assert_eq!(component.removed.len(), 2);

        component.add_entity(40, Entity(1));
        assert_eq!(component.added.len(), 4);
        assert!(component.added.contains(&Entity(1)));

        let entity1_count = component
            .added
            .iter()
            .filter(|&&e| e == Entity(1))
            .count();
        assert_eq!(entity1_count, 2);
    }

    #[test]
    fn bench_iter_compare_sparse_vs_hashmap() {
        use std::time::Instant;
        const N: usize = 50_000;

        // Sparse backend setup
        let mut sparse = super::Storage::<u32>::new_sparse(N.max(1));
        for i in 0..N {
            sparse.add_entity(i as u32, Entity(i));
        }

        // HashMap backend setup
        let mut map = super::Storage::<u32>::new_hashmap();
        for i in 0..N {
            map.add_entity(i as u32, Entity(i));
        }

        // Mutation pass timing
        let t0 = Instant::now();
        for (_e, v) in sparse.iter_mut() {
            *v = v.wrapping_add(1);
        }
        let sparse_mut = t0.elapsed();

        let t0 = Instant::now();
        for (_e, v) in map.iter_mut() {
            *v = v.wrapping_add(1);
        }
        let map_mut = t0.elapsed();

        // Read pass timing
        let t0 = Instant::now();
        let mut sum = 0u64;
        for (_e, v) in sparse.iter() {
            sum = sum.wrapping_add(*v as u64);
        }
        let _ = sum;
        let sparse_read = t0.elapsed();

        let t0 = Instant::now();
        let mut sum2 = 0u64;
        for (_e, v) in map.iter() {
            sum2 = sum2.wrapping_add(*v as u64);
        }
        let _ = sum2;
        let map_read = t0.elapsed();

        println!(
            "N={N} sparse_mut={sparse_mut:?} map_mut={map_mut:?} sparse_read={sparse_read:?} map_read={map_read:?}"
        );

        assert_eq!(sparse.len(), map.len());
    }

    #[test]
    fn trait_object_usage() {
        // Manipulate either backend via unified Storage type.
        let mut sparse: super::Storage<u32> = super::Storage::new_sparse(10);
        let mut map: super::Storage<u32> = super::Storage::new_hashmap();
        let s_store = &mut sparse;
        let m_store = &mut map;
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
