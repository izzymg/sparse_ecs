// Resources for ECS

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// A read-only handle to a resource.
/// Provides shared access to the underlying resource.
pub struct ResourceHandle<'a, T: Resource> {
    resource: MappedRwLockReadGuard<'a, T>,
}

impl<'a, T: Resource> Deref for ResourceHandle<'a, T> {
    type Target = T;

    /// Dereferences the handle to access the underlying resource.
    fn deref(&self) -> &Self::Target {
        self.resource.deref()
    }
}

/// A mutable handle to a resource.
/// Provides exclusive access to the underlying resource.
pub struct ResourceMutHandle<'a, T: Resource> {
    resource: MappedRwLockWriteGuard<'a, T>,
}

impl<'a, T: Resource> Deref for ResourceMutHandle<'a, T> {
    type Target = T;

    /// Dereferences the handle to access the underlying resource.
    fn deref(&self) -> &Self::Target {
        self.resource.deref()
    }
}

impl<'a, T: Resource> DerefMut for ResourceMutHandle<'a, T> {
    /// Dereferences the handle to access the underlying resource mutably.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.resource.deref_mut()
    }
}

/// Trait representing a resource in the ECS.
/// Resources must be thread-safe and have a unique key for identification.
pub trait Resource: Send + Sync + 'static {}

/// Container for managing resources in the ECS.
/// Provides methods to add, retrieve, and remove resources.
pub struct Resources {
    resources: std::collections::HashMap<TypeId, RwLock<Box<dyn Any + Send + Sync + 'static>>>,
}

impl Debug for Resources {
    /// Formats the resources for debugging, showing their keys.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resources")
            .field("resources", &self.resources.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[allow(dead_code)]
impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

impl Resources {
    /// Creates a new, empty resource container.
    pub fn new() -> Self {
        Self {
            resources: std::collections::HashMap::new(),
        }
    }

    /// Adds a resource to the container.
    /// The resource is stored using its unique key.
    pub fn add<T: Resource>(&mut self, resource: T) {
        let key = TypeId::of::<T>();
        self.resources.insert(key, RwLock::new(Box::new(resource)));
    }

    /// Retrieves a read-only handle to a resource by its type.
    /// Returns `None` if the resource is not found.
    pub fn get<T: Resource>(&self) -> Option<ResourceHandle<T>> {
        let key = TypeId::of::<T>();
        self.resources.get(&key).map(|item| {
            item.try_read().map(|lock| {
                let guard = RwLockReadGuard::map(lock, |b| b.downcast_ref::<T>().unwrap());
                ResourceHandle { resource: guard }
            })
        })?
    }

    /// Retrieves a mutable handle to a resource by its type.
    /// Returns `None` if the resource is not found.
    pub fn get_mut<T: Resource>(&self) -> Option<ResourceMutHandle<T>> {
        let key = TypeId::of::<T>();
        self.resources.get(&key).map(|item| {
            item.try_write().map(|lock| {
                let guard = RwLockWriteGuard::map(lock, |b| b.downcast_mut::<T>().unwrap());
                ResourceMutHandle { resource: guard }
            })
        })?
    }

    /// Removes a resource from the container by its key.
    pub fn remove<T: Resource>(&mut self) {
        let key = TypeId::of::<T>();
        self.resources.remove(&key);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    /// A test resource for verifying the functionality of the `Resources` container.
    #[allow(dead_code)]
    pub struct TestResource {
        pub value: i32,
    }

    impl Resource for TestResource {}

    #[test]
    /// Tests adding and retrieving a resource from the container.
    fn test_add_and_get_resource() {
        let mut resources = Resources::new();
        resources.add(TestResource { value: 42 });
        let res = resources.get::<TestResource>();
        assert!(res.is_some());
        assert_eq!(res.unwrap().resource.value, 42);
    }

    #[test]
    /// Tests thread-safe access to resources in the container.
    fn test_thread_access() {
        use std::sync::Arc;
        use std::thread;

        let mut resources = super::Resources::new();
        resources.add(TestResource { value: 42 });

        let resources_1 = Arc::new(resources);
        let resources_2 = resources_1.clone();

        thread::spawn(move || {
            assert!(resources_2.get::<TestResource>().is_some());
            resources_2
                .get_mut::<TestResource>()
                .unwrap()
                .resource
                .value += 1;
        })
        .join()
        .unwrap();

        assert!(resources_1.get::<TestResource>().is_some());
        assert!(resources_1.get::<TestResource>().unwrap().value == 43);
    }
}
