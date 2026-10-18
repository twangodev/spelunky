//! Configured initial Lava/contact and population controls; every actor runs
//! default Create, then a complete Game::step. No fuse, callback or mid-run
//! actor mutation is injected. These are not natural input-route witnesses.
use super::*;

fn room(rules: RulesPreset, low_graphics: bool, population: usize) -> (Game, u64) {
    let mut game = Game::shell(7, rules);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..24 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    if low_graphics {
        game.flags.insert("low_graphics".into());
    }
    // The bare parent Create draws no RNG; all instances are active and away
    // from solids, Lava, the player and the eventual Explosion collision mask.
    for i in 0..population {
        game.spawn("oDetritus", Vec2::new(200.0 + i as f64, 120.0));
    }
    game.spawn("oLava", Vec2::new(80.0, 80.0));
    let bomb = game.spawn("oBomb", Vec2::new(88.0, 88.0));
    assert_eq!(game.world.entity(bomb).unwrap().sprite, "sBomb");
    assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
    assert_eq!(game.world.entity(bomb).unwrap().vel, Vec2::default());
    assert!(!game.world.fluid_dirty);
    (game, bomb)
}

// Literal canonical random/rand expressions over raw words. This oracle never
// calls production unit/range, Flame payload, collision or movement helpers.
fn flame_create(rng: &mut Rng) -> Vec2 {
    let first = rng.next_u64() as f64 / 4_294_967_296.0;
    let second = rng.next_u64() as f64 / 4_294_967_296.0;
    let third = rng.next_u64() as f64 / 4_294_967_296.0;
    let _gravity = (1 + ((rng.next_u64() * 6) >> 32)) as f64 * 0.1;
    Vec2::new(first * 4.0 - second * 4.0, -1.0 - third * 2.0)
}

fn source_displacement(velocity: f64, time: u64) -> f64 {
    let magnitude = velocity.abs();
    let fraction = magnitude.fract();
    let extra = fraction != 0.0 && time % (1.0 / fraction).round_ties_even().max(1.0) as u64 == 0;
    (magnitude.floor() + f64::from(extra)) * velocity.signum()
}

#[test]
fn configured_lava_bomb_direct_flame_creates_ignore_graphics_and_script_cap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for low_graphics in [false, true] {
            for population in [0, 16, 30, 32] {
                let (mut game, bomb) = room(rules, low_graphics, population);
                let first_birth = game.world.next_id;
                let mut expected_rng = game.rng.clone();
                let velocities: Vec<_> = (0..3).map(|_| flame_create(&mut expected_rng)).collect();
                // Parent body moves zero pixels, adds gravity, then sinks .05
                // and clears velocity before its destruction-point query.
                let origin = Vec2::new(88.0, 88.0 + 0.05);
                let accepted = 3.min(32 - population);
                let mut expected_smoke = Vec::new();
                // Every rejected direct Create still executes oFlame Destroy
                // synchronously before its four child Create random draws.
                for rejected in 0..3 - accepted {
                    expected_smoke.push((
                        first_birth + 1 + accepted as u64 + 2 * rejected as u64 + 1,
                        origin,
                    ));
                }
                let after_creates = first_birth + 4 + (3 - accepted) as u64;
                // Accepted Flames (resource456) get their own Step after Bomb
                // resource454 in this same frame. Their postmove y-4 probes
                // remain inside this configured Lava and each emits one Smoke.
                for (index, velocity) in velocities.iter().take(accepted).enumerate() {
                    let pos = Vec2::new(
                        origin.x + source_displacement(velocity.x, 2),
                        origin.y + source_displacement(velocity.y, 2),
                    );
                    assert!(pos.y - 4.0 >= 80.0 && pos.y - 4.0 < 96.0);
                    assert!(pos.x >= 80.0 && pos.x < 96.0);
                    expected_smoke.push((after_creates + index as u64, pos));
                }
                game.step(&InputFrame::default());
                assert!(game.world.entity(bomb).is_none());
                let explosion = game.world.entity(first_birth).unwrap();
                assert_eq!(explosion.object, "oExplosion");
                assert_eq!(explosion.pos, origin);
                assert_eq!(explosion.image_speed(), 0.8);
                let mut actual_smoke: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|entity| entity.id >= first_birth && entity.object == "oSmokePuff")
                    .map(|entity| (entity.id, entity.previous_pos))
                    .collect();
                actual_smoke.sort_by_key(|(id, _)| *id);
                expected_smoke.sort_by_key(|(id, _)| *id);
                assert_eq!(actual_smoke, expected_smoke);
                assert_eq!(game.world.next_id, first_birth + 7);
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|entity| entity.object == "oFlame")
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|entity| entity.object == "oDetritus")
                        .count(),
                    population
                );
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
                let sound_positions: Vec<_> = game
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Sound { name, pos } if name == "sndExplosion" => Some(*pos),
                        _ => None,
                    })
                    .collect();
                assert_eq!(sound_positions, vec![origin, origin]);
                assert_eq!(game.player.health, 4.0);
                assert_eq!(game.player.bombs, 4.0);
                assert_eq!(game.engine_time(), 2);
            }
        }
    }
}

#[test]
fn configured_lava_contact_rope_burn_keeps_the_source_sprite_and_animation() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(7, rules);
        game.world = World::new(24, 20, Area::Mines, 1);
        game.player.pos = Vec2::new(40.0, 152.0);
        game.player.previous_pos = game.player.pos;
        for x in 0..24 {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        // Initial climbable geometry, no authored Flame or burn timer. The
        // actual Rope Step creates RopeBurn at (88,96), then its own Step rises.
        game.spawn("oRope", Vec2::new(80.0, 96.0));
        game.spawn("oLava", Vec2::new(88.0, 96.0));
        let flame = game.world.next_id;
        game.step(&InputFrame::default());
        let burning = game.world.entity(flame).unwrap();
        assert_eq!(burning.object, "oRopeBurn");
        assert_eq!(burning.sprite, "sBurn");
        assert_eq!(burning.pos, Vec2::new(88.0, 95.0));
        assert_eq!(burning.vel.y, -1.0);
        assert_eq!(burning.image_speed(), 0.8);
        assert_eq!(burning.animation, 0.8);
        assert!(burning.sprite_bounds().is_valid());
        assert!(crate::catalog::sprite_frame_count(&burning.sprite) > 1);
        assert_eq!(game.player.health, 4.0);
    }
}

fn graphical_crop(game: &mut Game, pos: Vec2, depth: f64) -> u64 {
    let id = game.spawn("oGenerationDecoration", pos);
    let crop = game.world.entity_mut(id).unwrap();
    crop.set_sprite("bgCaveTop");
    for (key, value) in [
        ("decoration", 1.0),
        ("depth", depth),
        ("source_x", 0.0),
        ("source_y", 0.0),
        ("source_w", 16.0),
        ("source_h", 16.0),
    ] {
        crop.data.insert(key.into(), value);
    }
    id
}

#[test]
fn configured_lava_blast_deletes_two_upper_and_one_lower_graphical_crops_before_immunity() {
    // Bounded represented graphics controls, not a native tile-ledger claim.
    // The default Bomb's real Lava Step produces the Explosion; no effect,
    // fuse, alarm or manual Collision is injected.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [false, true] {
            for protected in [false, true] {
                let (mut game, bomb) = room(rules, false, 0);
                let origin = Vec2::new(80.0, 96.0);
                let object = if protected { "oHardBlock" } else { "oBrick" };
                let solid = if dynamic {
                    Some(game.spawn(object, origin))
                } else {
                    game.world.set_tile(5, 6, object);
                    None
                };
                let upper: Vec<_> = (0..3)
                    .map(|_| graphical_crop(&mut game, Vec2::new(80.0, 80.0), 3.0))
                    .collect();
                let lower: Vec<_> = (0..2)
                    .map(|_| graphical_crop(&mut game, Vec2::new(80.0, 112.0), 3.0))
                    .collect();
                let other_depth = graphical_crop(&mut game, Vec2::new(80.0, 80.0), 2.0);
                let remote = graphical_crop(&mut game, Vec2::new(160.0, 80.0), 3.0);
                game.step(&InputFrame::default());
                assert!(game.world.entity(bomb).is_none());
                assert!(game.world.entities.iter().any(|e| e.object == "oExplosion"));
                if let Some(solid) = solid {
                    assert_eq!(game.world.entity(solid).is_some(), protected);
                } else {
                    assert_eq!(!game.world.tile(5, 6).unwrap().object.is_empty(), protected);
                }
                assert!(game.world.entity(upper[0]).is_none());
                assert!(game.world.entity(upper[1]).is_none());
                assert!(game.world.entity(upper[2]).is_some());
                assert!(game.world.entity(lower[0]).is_none());
                assert!(game.world.entity(lower[1]).is_some());
                assert!(game.world.entity(other_depth).is_some());
                assert!(game.world.entity(remote).is_some());
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}
