# Sparse ECS

A simple sparse-set ECS in Rust. I use this in small personal projects and cater it to a very specific use case.

### Features

- Resources for arbitrary thread-safe (rwlock) data access
- World (flexible component storage)
- Tags (static str entity hashset)
- Entity ID re-use
- Two storage backends: sparse set, and hashmap-indexed dense for very sparse components

### Does not do

- Systems/scheduling — systems are just functions. Write some functions.
- Complex queries — TODO. Some macros for mixed mutability access would be convenient.
- Inherently multi-threaded world access — TODO.

### Storage

Pick the storage per component type.

- SparseSet: fixed capacity, O(1) `has/get`, fast dense iteration.
- HashMapSet: maps `Entity -> dense index`, keeps data/ids in compact arrays for fast iteration without pre-allocating a big sparse vec.

When to use which:

- Use SparseSet for moderate/high density or frequent random access.
- Use HashMapSet when the component is very sparse or the entity ID space is large/unbounded; iterate a denser component and check this as a filter.

Add components:

```rust
use sparse_ecs::world::{World, ComponentStorageKind};

let mut world = World::new(10_000);
// Default is SparseSet
world.add::<Position>();
// Explicitly pick a backend
world.add_with_storage::<Velocity>(ComponentStorageKind::HashMap);
```
