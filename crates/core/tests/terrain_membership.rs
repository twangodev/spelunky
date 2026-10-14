//! Actual-Step lifecycle controls; independent before/after traces supplement
//! these source-derived expectations. No original-executable parity claim.
use spelunky_core::{
    dispatcher::{self, Instance},
    level::Level,
    *,
};

pub fn fixture_game() -> Game {
    let mut level = Level::default();
    level.set_tile(6, 9, '@').unwrap();
    // Authored Level (3, 4) loads at raw (64, 80), World tile (4, 5).
    // oSpearTrapTop at World (4, 4) requires this solid support below.
    level.set_tile(3, 4, '1').unwrap();
    for x in 0..40 {
        level.set_tile(x, 10, '1').unwrap();
    }
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.load_custom(&level, "terrain-membership", "");
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game
}
pub fn fixture_change(game: &mut Game, tick: u32) {
    match tick {
        1 => {
            // Reverse birth serials relative to the grid-ordered Create pass.
            game.world.set_tile(12, 4, "oArrowRepeaterL");
            game.world.set_tile(4, 4, "oArrowRepeaterL");
        }
        2 => dispatcher::activate_object(game, "oArrowRepeaterL"),
        3 => game.world.set_tile(34, 4, "oArrowRepeaterR"),
        4 => dispatcher::activate_region(game, Rect::new(520., 48., 64., 64.)),
        5 => game.world.set_tile(4, 4, "oArrowTrapLeft"),
        6 => {
            game.world.set_tile(12, 4, "");
            let id = game.spawn("oSnake", Vec2::new(200., 72.));
            // Public/deserialized pending-Destroy control, not a play route.
            game.world.entity_mut(id).unwrap().alive = false;
        }
        7 => game.world.set_tile(12, 4, "oArrowRepeaterL"),
        8 => game.world.set_tile(4, 4, "oSpearTrapTop"),
        9 => game.world.set_tile(34, 4, ""),
        10 => {
            dispatcher::activate_region(game, Rect::new(0., 0., 80., 80.));
            // Same object but a new creation_serial at the existing grid handle.
            game.world.set_tile(12, 4, "oArrowRepeaterL");
        }
        11 => {
            game.spawn("oRock", Vec2::new(200., 72.));
        }
        12 => game.world.set_tile(6, 10, ""),
        13 => game.world.set_tile(25, 7, "oTree"),
        14 => game.world.set_tile(25, 7, ""),
        _ => {}
    }
}
// Original rand(100) omits argument1: 100 + rand(100) is
// 200 + floor(-99 * U). Compute its one LCG draw independently of range/source_rand.
fn source_repeater_delay_oracle(rng: &mut Rng) -> f64 {
    assert_eq!(rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    let state = u32::try_from(rng.state_bits()).expect("canonical raw u32 state");
    let next = state.wrapping_mul(0x08088405).wrapping_add(1);
    // Advance only this expected clone; the literal state is the value oracle.
    assert_eq!(rng.next_u64(), u64::from(next));
    200.0 + (-99.0 * f64::from(next) / 4_294_967_296.0).floor()
}
#[test]
fn terrain_birth_create_rng_follows_grid_order_not_tile_serials() {
    let mut game = fixture_game();
    let mut expected_rng = game.rng.clone();
    let first = source_repeater_delay_oracle(&mut expected_rng);
    let second = source_repeater_delay_oracle(&mut expected_rng);
    fixture_change(&mut game, 1);
    game.step(&InputFrame::default());
    let a = 4 * game.world.width + 4;
    let b = 4 * game.world.width + 12;
    assert_eq!(game.data[&format!("hazard:{a}:fired")], first - 1.);
    assert_eq!(game.data[&format!("hazard:{b}:fired")], second - 1.);
    assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
    assert!(game.world.tiles[a].creation_serial > game.world.tiles[b].creation_serial);
}
#[test]
fn inactive_registered_terrain_preserves_create_then_replacement_gets_fresh_create() {
    let mut game = fixture_game();
    let index = 4 * game.world.width + 34;
    game.world.set_tile(34, 4, "oArrowRepeaterL");
    game.step(&InputFrame::default());
    assert!(
        game.dispatcher
            .inactive_order
            .contains(&Instance::Terrain(index))
    );
    let initial = game.data[&format!("hazard:{index}:fired")];
    let rng = game.rng.clone().next_u64();
    game.step(&InputFrame::default());
    assert_eq!(game.data[&format!("hazard:{index}:fired")], initial);
    assert_eq!(game.rng.clone().next_u64(), rng);
    let serial = game.world.tiles[index].creation_serial;
    let mut expected_rng = game.rng.clone();
    let replacement_delay = source_repeater_delay_oracle(&mut expected_rng);
    game.world.set_tile(34, 4, "oArrowRepeaterR");
    game.step(&InputFrame::default());
    assert_eq!(
        game.data[&format!("hazard:{index}:fired")],
        replacement_delay
    );
    assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
    assert!(game.world.tiles[index].creation_serial > serial);
    assert_eq!(
        game.dispatcher
            .inactive_order
            .iter()
            .filter(|&&i| i == Instance::Terrain(index))
            .count(),
        1
    );
}
#[test]
fn entity_numeric_identity_does_not_hide_new_terrain_create() {
    let mut game = fixture_game();
    let index = 4 * game.world.width + 6;
    // Deliberately malformed public state still distinguishes Instance variants.
    game.dispatcher
        .active_order
        .push(Instance::Entity(index as u64));
    game.dispatcher
        .inactive_order
        .push(Instance::Entity(index as u64));
    game.world.set_tile(6, 4, "oArrowTrapLeft");
    game.step(&InputFrame::default());
    assert_eq!(game.data[&format!("traps_create_terrain:{index}")], 1.);
    assert_eq!(game.world.tiles[index].alarms[1], 0);
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oArrowTrapTest")
    );
}
#[test]
fn terrain_lifecycle_clear_rebirth_activation_and_pending_destroy_remain_finite() {
    let mut game = fixture_game();
    assert!(game.world.solid_at(64., 80.));
    for tick in 0..16 {
        fixture_change(&mut game, tick);
        game.step(&InputFrame::default());
        assert!(game.player.pos.x.is_finite() && game.player.pos.y.is_finite());
        assert!(game.world.entities.iter().all(|e| e.alive));
        assert_eq!(game.dispatcher.phase, dispatcher::Phase::Idle);
        let _ = game.state_hash();
        let _ = game.presentation().state_hash();
    }
    assert!(game.world.solid_at(64., 80.));
    assert_eq!(game.world.tile(4, 4).unwrap().object, "oSpearTrapTop");
    assert_eq!(game.world.tile(12, 4).unwrap().object, "oArrowRepeaterL");
    assert!(game.world.tile(34, 4).unwrap().object.is_empty());
}
