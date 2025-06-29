use sparse_ecs::{world::World, Component};

#[derive(Component, Copy, Clone)]
struct CoolThing {
    sickness: i32,
}

mod my_tags {
    pub const PLAYER: &str = "player";
}

fn sickness_system(world: &mut World) {
    let moveable_objects = world.get::<CoolThing>().unwrap();
    let player = world.tags.expect_one(my_tags::PLAYER);

    let player_sickness = moveable_objects
        .get(player).unwrap();

    if player_sickness.sickness == 100 {
        println!(
            "Player is fully sick",
        );
    }

}

fn main() {
    let mut world = World::new(10);
    world.add::<CoolThing>();
    let player_entity = world.spawn();
    world.get_mut::<CoolThing>().unwrap().add_entity(CoolThing { sickness: 100 }, player_entity);
    world.tags.add_tag(my_tags::PLAYER, player_entity);
    sickness_system(&mut world);
}
