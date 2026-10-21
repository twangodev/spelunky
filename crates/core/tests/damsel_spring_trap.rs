use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

#[test]
fn spring_trap_keeps_its_horizontal_impulse_through_the_damsel_idle_step() {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "spring-damsel-parity".into();
    game.player.pos = Vec2::new(160.0, 120.0);

    game.world.set_tile(5, 6, "oBlock");
    game.spawn("oSpringTrap", Vec2::new(80.0, 80.0));
    let damsel = game.spawn("oDamsel", Vec2::new(88.0, 88.0));

    game.step(&InputFrame::default());
    let donor = game.world.entity(damsel).expect("spring-trapped Damsel");
    assert_eq!(donor.vel.x, -1.0, "Classic applies the trap impulse");
    assert_eq!(donor.pos.x, 88.0, "the collision happens after this Step");

    game.step(&InputFrame::default());
    let donor = game.world.entity(damsel).expect("Damsel remains alive");
    assert_eq!(donor.pos.x, 87.0, "inherited oItem Step moves the Damsel");
    assert_eq!(
        donor.vel.x, -1.0,
        "oDamsel's IDLE branch leaves the inherited spring impulse intact"
    );

    game.step(&InputFrame::default());
    let donor = game.world.entity(damsel).expect("Damsel remains alive");
    assert_eq!(donor.pos.x, 86.0, "the impulse continues on the next Step");
}
