use sparse_ecs::{ecs_and, world::World, Component};

#[derive(Component, Copy, Clone)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Component, Copy, Clone)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
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
    move_system(&mut world);
}
