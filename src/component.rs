// Sparse set component storage for the ecs

use std::{
    slice::{Iter, IterMut},
    str::FromStr,
};

use std::fmt::Debug;

use crate::world::Component;

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
    sparse: Vec<Option<usize>>,
    dense: Vec<T>,
    entities: Vec<usize>,
    pub dirty: bool,
}

impl<T> SparseSet<T>
where
    T: Send + Sync + Sized + Copy + Clone,
{
    /// Creates a new component storage with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            sparse: vec![None; capacity],
            dense: Vec::with_capacity(capacity),
            entities: Vec::with_capacity(capacity),
            dirty: true,
        }
    }

    /// Adds a new entity with the given component data.
    /// Panics if the entity already exists in this component.
    pub fn add_entity(&mut self, data: T, entity: Entity) {
        assert_eq!(self.sparse[entity.0], None);
        self.sparse[entity.0] = Some(self.dense.len());
        self.dense.push(data);
        self.entities.push(entity.0);
        self.dirty = true;
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
                self.dirty = true;
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

/// Iterator over (Entity, &T) pairs for a component.
pub struct ComponentIter<'a, T, F>
where
    F: Fn(usize) -> Entity,
{
    entities: Iter<'a, usize>,
    components: Iter<'a, T>,
    mapper: F,
}

impl<'a, T: Send + Sync + Copy + Clone, F: Fn(usize) -> Entity> Iterator
    for ComponentIter<'a, T, F>
{
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.entities
            .next()
            .map(|id| (self.mapper)(*id))
            .zip(self.components.next())
    }
}

/// Iterator over (Entity, &mut T) pairs for a component.
pub struct ComponentIterMut<'a, T, F>
where
    F: Fn(usize) -> Entity,
{
    entities: Iter<'a, usize>,
    components: IterMut<'a, T>,
    mapper: F,
}

impl<'a, T: Send, F: Fn(usize) -> Entity> Iterator for ComponentIterMut<'a, T, F> {
    type Item = (Entity, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.entities
            .next()
            .map(|id| (self.mapper)(*id))
            .zip(self.components.next())
    }
}

/* Iterator over ref into the dense array, and entity */
impl<'a, T: Send + Sync + Copy + Clone> IntoIterator for &'a SparseSet<T> {
    type Item = (Entity, &'a T);
    type IntoIter = ComponentIter<'a, T, fn(usize) -> Entity>;

    fn into_iter(self) -> Self::IntoIter {
        ComponentIter {
            entities: self.entities.iter(),
            components: self.dense.iter(),
            mapper: |id| Entity(id),
        }
    }
}

/* Iterator over mutable ref into the dense array, and entity */
impl<'a, T: Send + Sync + Copy + Clone> IntoIterator for &'a mut SparseSet<T> {
    type Item = (Entity, &'a mut T);
    type IntoIter = ComponentIterMut<'a, T, fn(usize) -> Entity>;

    fn into_iter(self) -> Self::IntoIter {
        ComponentIterMut {
            entities: self.entities.iter(),
            components: self.dense.iter_mut(),
            mapper: |id| Entity(id),
        }
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

    use std::vec;

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

        for (entity, _position) in &positions {
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
        for (_entity, data) in &mut component {
            *data = 5;
        }
        for (_entity, data) in &component {
            assert_eq!(*data, 5);
        }
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
}
