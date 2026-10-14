//! Source Shopkeeper Step460–464, before friction557–559.
//! Initial authored room/control fields are explicit. After setup, inputs only.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const SIDES: [i8; 2] = [-1, 1];
const EPS: f64 = 1e-9;

fn sound(game: &Game, wanted: &str) -> bool {
    game.events.iter().any(|event| {
        matches!(event,
        GameEvent::Sound { name, .. } if name == wanted)
    })
}

fn untouched_player(game: &Game) {
    assert_eq!(game.player.health, 4, "FIXTURE_NO_DAMAGE");
    assert_eq!(game.player.stun, 0, "FIXTURE_NO_STUN");
    assert!(!game.player.is_stunned());
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. }))
    );
}

fn room(rules: RulesPreset, side: i8, armed: bool) -> (Game, u64) {
    let mut game = Game::shell(37, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    // Negative controls use a stationary player24pixels from keeper center.
    // The armed positive below starts running14pixels away, then retreats.
    game.player.pos = Vec2::new(168.0 + 24.0 * f64::from(side), 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.facing = -side;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    if armed {
        // Authored ordinary running pose. Run+Attack produces the rear Whip
        // while the player keeps moving away, clearing first-shot pellets.
        game.player.pos.x = 168.0 + 14.0 * f64::from(side);
        game.player.previous_pos = game.player.pos;
        game.player.facing = side;
        game.player.state = PlayerState::Running;
        game.player.vel.x = 4.0 * f64::from(side);
    }
    game.world.entrance = game.player.pos;
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    let keeper = game.spawn("oShopkeeper", Vec2::new(160.0, 144.0));
    assert_eq!(game.world.entity(keeper).unwrap().state, 0);
    assert_eq!(game.world.entity(keeper).unwrap().health, 20);
    assert_eq!(game.world.entity(keeper).unwrap().value("has_gun"), 1.0);
    if !armed {
        // Explicit authored unarmed control, not a naturally earned disarm.
        game.world
            .entity_mut(keeper)
            .unwrap()
            .data
            .insert("has_gun".into(), 0.0);
    }
    // No invincibility shortcut: Bullet own collision bypasses it.
    assert_eq!(game.player.invincible, 0);
    (game, keeper)
}

fn whip_to_attack(game: &mut Game, keeper: u64, side: i8) {
    let first_birth = game.world.next_id;
    for frame in 0..16 {
        game.step(&InputFrame {
            left: side < 0,
            right: side > 0,
            run: true,
            attack: frame == 0,
            attack_pressed: frame == 0,
            ..Default::default()
        });
        untouched_player(game);
        let target = game.world.entity(keeper).unwrap();
        if target.value("whipped") == 0.0 {
            assert_eq!(target.state, 0, "FIXTURE_ATTACK_MUST_BE_FROM_WHIP");
            continue;
        }
        assert_eq!(target.state, 2);
        assert_eq!(target.health, 20, "plain Whip is not Machete damage");
        assert!(
            (target.vel.y + 2.0).abs() < EPS,
            "FIXTURE_REAL_WHIP_UPWARD_BUMP"
        );
        assert_eq!(target.alarms[0], 10);
        assert!(sound(game, "sndHit"));
        assert!(
            game.world
                .entities
                .iter()
                .any(|actor| actor.id >= first_birth
                    && matches!(actor.object.as_str(), "oWhip" | "oWhipPre"))
        );
        return;
    }
    panic!("FIXTURE_ORDINARY_WHIP_DID_NOT_HIT");
}

fn source_slowdown(game: &Game, keeper: u64, expected_facing: i8) {
    let target = game.world.entity(keeper).unwrap();
    assert_eq!(target.state, 2);
    assert!(!target.held);
    assert_eq!(target.facing, expected_facing);
    assert!(
        target.bounds().y + target.bounds().h < 160.0,
        "FIXTURE_CACHED_COLBOT_FALSE_ON_UNBROKEN_FLOOR"
    );
    assert!(
        game.player.pos.y > target.pos.y + 8.0,
        "FIXTURE_PLAYER_BELOW_AIRBORNE_KEEPER"
    );
    assert!(
        (target.vel.x - f64::from(expected_facing) * 1.4).abs() < EPS,
        "KEEPER_AIRBORNE_SOURCE_TAIL_REQUIRED"
    );
}

#[test]
fn default_shopkeeper2_airborne_birth_runs_source_tail_before_first_friction() {
    for rules in PRESETS {
        let mut game = Game::shell(37, rules);
        game.world = World::new(40, 24, Area::Mines, 1);
        game.world.name = "rLevel".into();
        game.player.pos = Vec2::new(80.0, 88.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.world.entrance = game.player.pos;
        for x in 0..40 {
            game.world.set_tile(x, 6, "oHardBlock");
        }
        // Exact object and sky origin used by the stock Stars alarm. Public
        // actor birth here is source-valid setup, not a selected challenge run.
        let keeper = game.spawn("oShopkeeper2", Vec2::new(160.0, 0.0));
        let newborn = game.world.entity(keeper).unwrap();
        assert_eq!(newborn.state, 2);
        assert_eq!(newborn.value("has_gun"), 0.0);
        assert_eq!(newborn.facing, -1);
        assert_eq!(newborn.vel, Vec2::default());
        game.step(&InputFrame::default());
        untouched_player(&game);
        let target = game.world.entity(keeper).unwrap();
        assert_eq!(target.pos.y, 0.0);
        assert!((target.vel.y - 0.6).abs() < EPS);
        assert!(!sound(&game, "sndShotgun"));
        source_slowdown(&game, keeper, -1);
    }
}

#[test]
fn ordinary_whip_default_gun_firing_recoil_precedes_airborne_slowdown() {
    for rules in PRESETS {
        for side in SIDES {
            let (mut game, keeper) = room(rules, side, true);
            whip_to_attack(&mut game, keeper, side);
            let first_birth = game.world.next_id;
            game.step(&InputFrame {
                left: side < 0,
                right: side > 0,
                run: true,
                ..Default::default()
            });
            // Stop on the first shot: do not pretend invincibility protects
            // the next advancing bullet. Clearance and no damage are required.
            untouched_player(&game);
            assert!(sound(&game, "sndShotgun"), "FIXTURE_DEFAULT_GUN_MUST_FIRE");
            assert_eq!(game.world.entity(keeper).unwrap().value("firing"), 30.0);
            assert!(
                game.world
                    .entities
                    .iter()
                    .any(|actor| actor.id >= first_birth && actor.object == "oBullet"),
                "FIXTURE_REAL_KEEPER_BULLETS"
            );
            source_slowdown(&game, keeper, side);
        }
    }
}

fn authored_negative(rules: RulesPreset, side: i8, grounded: bool) -> (Game, u64) {
    let (mut game, keeper) = room(rules, side, false);
    // Explicit source-statement controls, not earned ATTACK transitions.
    // All modifications precede the first Game.step.
    if grounded {
        for x in 0..40 {
            game.world.set_tile(x, 10, "");
            game.world.set_tile(x, 11, "oHardBlock");
        }
        game.world.set_tile(10, 10, "oHardBlock");
        game.player.pos.y = 168.0;
    } else {
        game.world
            .set_tile(if side < 0 { 9 } else { 11 }, 6, "oHardBlock");
        game.player.pos.y = 88.0;
        game.world.entity_mut(keeper).unwrap().pos.y = 112.0;
    }
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    let target = game.world.entity_mut(keeper).unwrap();
    target.previous_pos = target.pos;
    target.state = 2;
    target.facing = side;
    target.vel = Vec2::default();
    (game, keeper)
}

#[test]
fn grounded_attack_does_not_slow_even_when_player_is_below() {
    for rules in PRESETS {
        for side in SIDES {
            let (mut game, keeper) = authored_negative(rules, side, true);
            game.step(&InputFrame::default());
            untouched_player(&game);
            let target = game.world.entity(keeper).unwrap();
            assert_eq!(target.state, 2);
            assert!((target.pos.y - 144.0).abs() < EPS);
            assert!(game.player.pos.y > target.pos.y + 8.0);
            assert!(
                (target.vel.x - f64::from(side) * 2.9).abs() < EPS,
                "KEEPER_GROUNDED_NEGATIVE_CONTROL"
            );
        }
    }
}

#[test]
fn airborne_attack_does_not_slow_when_player_is_above() {
    for rules in PRESETS {
        for side in SIDES {
            let (mut game, keeper) = authored_negative(rules, side, false);
            game.step(&InputFrame::default());
            untouched_player(&game);
            let target = game.world.entity(keeper).unwrap();
            assert_eq!(target.state, 2);
            assert!(target.bounds().y + target.bounds().h < 160.0);
            assert!(game.player.pos.y < target.pos.y + 8.0);
            assert!(
                (target.vel.x - f64::from(side) * 2.9).abs() < EPS,
                "KEEPER_HIGH_PLAYER_NEGATIVE_CONTROL"
            );
        }
    }
}
