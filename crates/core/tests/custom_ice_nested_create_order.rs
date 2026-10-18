//! Custom Ice synchronously releases a CaveMan during its Create callback.
use spelunky_core::{Game, InputFrame, RulesPreset, level::Level};

#[test]
fn ice_released_caveman_keeps_its_source_birth_slot_and_rng_order() {
    let mut level = Level::default();
    level.set_tile(5, 3, 'i').unwrap();
    level.set_tile(3, 5, 'h').unwrap();
    level.set_tile(3, 6, '1').unwrap();

    let mut game = Game::shell(0, RulesPreset::Classic);
    game.load_custom(&level, "nested-ice-create-order.lvl", "");
    let released = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oCaveman" && e.state == 98)
        .expect("seed zero releases the FrozenCaveman as a stunned CaveMan");
    let released_id = released.id;
    let released_serial = released.creation_serial;
    let authored_id = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oCaveman" && e.state == 0)
        .expect("the h symbol creates an ordinary CaveMan")
        .clone();
    let authored_serial = authored_id.creation_serial;
    let authored_id = authored_id.id;

    assert!(
        released_serial < authored_serial,
        "oIce's synchronous Destroy-created CaveMan is born before later map cells"
    );

    let mut classic = game.rng.clone();
    let after_load = classic.state_bits();
    let mut blood = Vec::new();
    for _ in 0..3 {
        let x = classic.unit() * 4.0 - classic.unit() * 4.0;
        let y = -1.0 - classic.unit() * 2.0;
        let gravity = f64::from(classic.range(6) + 1) * 0.1;
        blood.push((x, y, gravity));
    }
    let classic_facing = if classic.range(2) == 0 { -1 } else { 1 };
    let classic_after_step = classic.state_bits();

    game.step(&InputFrame::default());
    let authored = game.world.entity(authored_id).unwrap();
    let actual_blood: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .map(|e| (e.vel.x, e.vel.y, e.value("gravity")))
        .collect();
    assert!(game.world.entity(released_id).is_none());
    assert_eq!(authored.facing, classic_facing);
    assert_eq!(
        actual_blood, blood,
        "the released CaveMan's three synchronous blood Creates precede the authored CaveMan Step"
    );
    assert_eq!(
        game.rng.state_bits(),
        classic_after_step,
        "the source and Rust paths consume the same 13 draws across these callbacks"
    );
    assert_ne!(after_load, classic_after_step);
}
