//! Player spear collision's source rand(4,6) precedes synchronous Blood Create.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

#[test]
fn spear_hit_draws_knockback_before_blood_create() {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Mines, 1);
    for x in 0..24 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(80.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    let spear = game.spawn("oSpearsLeft", Vec2::new(72.0, 152.0));
    game.world.entity_mut(spear).unwrap().animation = 21.0;
    let mut state = game.rng.state_bits() as u32;
    let mut draw = || {
        state = state.wrapping_mul(134775813).wrapping_add(1);
        u64::from(state)
    };
    // Player Step 1699-1715: rand(4,6), then scrCreateBlood. Blood Create:
    // random(4)-random(4), -1-random(2), rand(1,6)*0.1.
    let speed = 4.0 + ((draw() * 3) >> 32) as f64;
    let vx = draw() as f64 / 4294967296.0 * 4.0 - draw() as f64 / 4294967296.0 * 4.0;
    let vy = -1.0 - draw() as f64 / 4294967296.0 * 2.0;
    let gravity = (1 + ((draw() * 6) >> 32)) as f64 * 0.1;

    game.step(&InputFrame::default());

    assert_eq!(game.player.health, 0.0);
    assert_eq!(game.player.vel.x, -speed);
    let blood: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oBlood")
        .collect();
    assert_eq!(blood.len(), 1);
    assert_eq!(blood[0].vel, Vec2::new(vx, vy));
    assert_eq!(blood[0].value("gravity"), gravity);
    assert_eq!(game.rng.state_bits(), u64::from(state));
}
