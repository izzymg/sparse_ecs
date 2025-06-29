pub mod component;
pub mod resource;
pub mod tags;
pub mod world;

#[cfg(feature = "macros")]
pub use sparse_ecs_macros::{Component, Resource};
