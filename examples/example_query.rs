use sparse_ecs::{Component, world::{World, FetchMut}};

#[derive(Component, Copy, Clone)]
struct Position { x: f32, y: f32 }

#[derive(Component, Copy, Clone)]
struct Velocity { dx: f32, dy: f32 }

#[derive(Component, Copy, Clone)]
struct Health(u32);

#[derive(Component, Copy, Clone)]
struct Mana(u32);

#[derive(Component, Copy, Clone)]
struct Damage(u32);

#[derive(Component, Copy, Clone)]
struct Armor(u32);

fn spawn_world() -> World {
    let mut world = World::new(32);
    world.add::<Position>();
    world.add::<Velocity>();
    world.add::<Health>();
    world.add::<Mana>();
    world.add::<Damage>();
    world.add::<Armor>();

    // Create 10 entities with varying component sets
    for i in 0..10 {
        let e = world.spawn();
        world.get_mut::<Position>().unwrap().add_entity(Position { x: i as f32, y: 0.0 }, e);
        if i % 2 == 0 {
            world.get_mut::<Velocity>().unwrap().add_entity(Velocity { dx: 1.0, dy: 0.5 }, e);
        }
        if i % 3 == 0 {
            world.get_mut::<Health>().unwrap().add_entity(Health(100), e);
            world.get_mut::<Damage>().unwrap().add_entity(Damage(5), e);
        }
        if i % 4 == 0 {
            world.get_mut::<Mana>().unwrap().add_entity(Mana(50), e);
        }
        if i % 5 == 0 {
            world.get_mut::<Armor>().unwrap().add_entity(Armor(10), e);
        }
    }

    world
}

fn system_move(world: &mut World) {
    // 2-component fetch (Position, Velocity)
    let (positions_opt, velocities_opt) = <(Position, Velocity) as FetchMut>::fetch(world);
    if let (Some(positions), Some(velocities)) = (positions_opt, velocities_opt) {
        for (entity, pos) in positions.iter_mut() {
            if let Some(vel) = velocities.get(entity) {
                pos.x += vel.dx;
                pos.y += vel.dy;
            }
        }
    }
}

fn system_apply_damage(world: &mut World) {
    // 3-component fetch (Health, Damage, Armor)
    let (health_opt, damage_opt, armor_opt) = <(Health, Damage, Armor) as FetchMut>::fetch(world);
    if let (Some(healths), Some(damages), Some(armors)) = (health_opt, damage_opt, armor_opt) {
        for (entity, hp) in healths.iter_mut() {
            let dmg = damages.get(entity).map(|d| d.0).unwrap_or(0);
            let armor = armors.get(entity).map(|a| a.0).unwrap_or(0);
            let mitigated = dmg.saturating_sub(armor / 2);
            hp.0 = hp.0.saturating_sub(mitigated);
        }
    }
}

fn sys_so_many_components(world: &mut World) {
    let (pos_opt, vel_opt, hp_opt, mana_opt, dmg_opt, armor_opt) =
        <(Position, Velocity, Health, Mana, Damage, Armor) as FetchMut>::fetch(world);
    if let (Some(_p), Some(_v), Some(_hps), Some(manas), Some(_d), Some(_a)) = (pos_opt, vel_opt, hp_opt, mana_opt, dmg_opt, armor_opt) {
        for (_entity, mana) in manas.iter_mut() {
            mana.0 = (mana.0 + 1).min(100);
        }
    }
}

fn main() {
    let mut world = spawn_world();

    println!("-- Before systems --");
    if let Some(positions) = world.get::<Position>() { println!("Positions: {}", positions.len()); }

    system_move(&mut world);
    system_apply_damage(&mut world);
    sys_so_many_components(&mut world);

    // Show state of first entity (which has all components)
    let first_entity = sparse_ecs::component::Entity(0);
    if let Some(pos_set) = world.get::<Position>() {
        if let Some(pos) = pos_set.get(first_entity) {
            println!("Entity0 Position: ({:.1}, {:.1})", pos.x, pos.y);
        }
    }
    if let Some(hp_set) = world.get::<Health>() { if let Some(h) = hp_set.get(first_entity) { println!("Entity0 Health: {}", h.0); }}
    if let Some(mana_set) = world.get::<Mana>() { if let Some(m) = mana_set.get(first_entity) { println!("Entity0 Mana: {}", m.0); }}
}
