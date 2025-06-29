use sparse_ecs::{ecs_and, world::World, Component};

#[derive(Component, Copy, Clone)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

impl Default for Position {
    fn default() -> Self {
        Position { x: 0.0, y: 0.0, z: 0.0 }
    }
}

#[derive(Component, Copy, Clone)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

impl Default for Velocity {
    fn default() -> Self {
        Velocity { x: 1.0, y: 5.0, z: 0.0 }
    }
}

fn move_system(world: &mut World) {
    let (positions, velocities) = world.get_two_mut::<Position, Velocity>();
    if let (Some(positions), Some(velocities)) = (positions, velocities) {
        for (entity, pos) in positions.iter_mut() {
            ecs_and!(velocities, entity, velocity, { continue; });
            pos.x += velocity.x;
            pos.y += velocity.y;
            pos.z += velocity.z;
        }
    }
}

fn main() {
    let mut world = World::new(10);
    world.add::<Position>();
    world.add::<Velocity>();
    let entity = world.spawn();
    world.get_mut::<Position>().unwrap().add_entity(Position::default(), entity);
    world.get_mut::<Velocity>().unwrap().add_entity(Velocity::default(), entity);
    
    move_system(&mut world);

    // Print the position of the entity after moving
    if let Some(positions) = world.get::<Position>() {
        let (_, pos) = positions.iter().next().unwrap();
        println!("Entity Position: x={}, y={}, z={}", pos.x, pos.y, pos.z);
    }
}
