use std::collections::{HashMap, HashSet};

use crate::component::Entity;

/// List of entities associated with a specific tag.
#[derive(Debug, Default)]
pub struct TagList {
    set: HashSet<Entity>,
}

impl TagList {
    pub fn add_entity(&mut self, entity: Entity) {
        self.set.insert(entity);
    }

    pub fn remove_entity(&mut self, entity: &Entity) {
        self.set.remove(entity);
    }

    pub fn contains(&self, entity: &Entity) -> bool {
        self.set.contains(entity)
    }

    pub fn expect_one(&self) -> Entity {
        assert!(
            self.set.len() == 1,
            "Expected exactly one entity with this tag"
        );
        self.set
            .iter()
            .next()
            .cloned()
            .expect("Expected exactly one entity with this tag")
    }
}

/// Tag collection management, each tag is associated with a set of entities.
/// It allows adding, removing, and querying entities by their tags.
#[derive(Debug)]
pub struct EntityTags {
    tags: HashMap<&'static str, TagList>,
}

impl EntityTags {
    pub fn new() -> Self {
        Self {
            tags: HashMap::new(),
        }
    }

    /// Adds a tag to the given entity.
    pub fn add_tag(&mut self, tag: &'static str, entity: Entity) {
        self.tags.entry(tag).or_default().add_entity(entity);
    }

    /// Adds multiple tags to the given entity.
    pub fn add_tags(&mut self, tags: &[&'static str], entity: Entity) {
        for &tag in tags {
            self.add_tag(tag, entity);
        }
    }

    /// Removes a tag from the given entity.
    pub fn remove_tag(&mut self, tag: &'static str, entity: &Entity) {
        if let Some(entities) = self.tags.get_mut(&tag) {
            entities.remove_entity(entity);
        }
    }

    /// Removes all tags from the given entity.
    pub fn remove_all_tags(&mut self, entity: &Entity) {
        for tag in self.tags.keys().cloned().collect::<Vec<&'static str>>() {
            self.remove_tag(tag, entity);
        }
    }

    /// Returns a list of all entities that have the given tag.
    pub fn get_entities_with_tag(&self, tag: &'static str) -> Option<Vec<Entity>> {
        self.tags
            .get(&tag)
            .map(|list| list.set.iter().cloned().collect::<Vec<Entity>>())
    }

    /// Returns the number of entities that have the given tag.
    pub fn count(&self, tag: &'static str) -> usize {
        self.tags.get(&tag).map_or(0, |list| list.set.len())
    }

    /// Returns the entity with the given tag, panics if there isn't one.
    /// Does *not* assert there is only one entity with the tag.
    pub fn expect_one(&self, tag: &'static str) -> Entity {
        if let Some(list) = self.tags.get(&tag) {
            list.expect_one()
        } else {
            panic!("Expected exactly one entity with tag: {tag:?}");
        }
    }

    /// Returns a single entity with the given tag.
    /// Does *not* assert there is only one entity with the tag.
    pub fn want_one(&self, tag: &'static str) -> Option<Entity> {
        if let Some(list) = self.tags.get(&tag) {
            if list.set.len() == 1 {
                return self
                    .tags
                    .get(&tag)
                    .and_then(|list| list.set.iter().next().cloned());
            }
        }
        None
    }

    /// Returns true if the given entity has the given tag.
    pub fn has_tag(&self, tag: &'static str, entity: &Entity) -> bool {
        self.tags.get(tag).map_or(false, |l| {
            l.contains(entity)
        })
    } 
}
