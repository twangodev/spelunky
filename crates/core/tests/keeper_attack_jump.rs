//! Source Shopkeeper Step436–458. All fixture writes precede simulation.
//! The ordinary-input recovery route is prepared and UNRUN; guards are mandatory.
use spelunky_core::CollisionPrecision;
use spelunky_core::catalog;
use spelunky_core::{
    Area, Entity, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const EPS: f64 = 1e-9;

fn sound(game: &Game, wanted: &str) -> bool {
    game.events
        .iter()
        .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == wanted))
}
fn unharmed(game: &Game) {
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
fn bbox_solid_point(game: &Game, p: Vec2) -> bool {
    game.world.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && catalog::is_a(&tile.object, "oSolid")
            && game
                .world
                .tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
                .contains(p, CollisionPrecision::BoundingBox)
    }) || game.world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && catalog::is_a(&e.object, "oSolid")
            && e.mask_contains(p, CollisionPrecision::BoundingBox)
    })
}
fn top_four(game: &Game, e: &Entity) -> bool {
    let b = e.bounds();
    let y = (b.y - 4.0).round_ties_even();
    game.world.solid_line_except(
        Vec2::new(b.x.round_ties_even(), y),
        Vec2::new((b.x + b.w - 1.0).round_ties_even(), y),
        e.id,
    )
}
fn bottom(game: &Game, e: &Entity) -> bool {
    let b = e.bounds();
    let y = (b.y + b.h).round_ties_even();
    game.world.solid_line_except(
        Vec2::new(b.x.round_ties_even(), y),
        Vec2::new((b.x + b.w - 1.0).round_ties_even(), y),
        e.id,
    )
}
fn shell(rules: RulesPreset) -> Game {
    let mut g = Game::shell(37, rules);
    g.world = World::new(40, 24, Area::Mines, 1);
    g.world.name = "rLevel".into();
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    g.player.facing = -1;
    g
}

// Stock Stars uses this exact ATTACK/no-gun object; this is a controlled legal
// public birth, not a selected Stars replay. Non-grid player floors are authored
// Solid instances and provide exact threshold coordinates without later writes.
fn birth_control(
    rules: RulesPreset,
    x: f64,
    y: f64,
    wall: bool,
    support: bool,
    ceiling: bool,
) -> (Game, u64) {
    let mut g = shell(rules);
    g.player.pos = Vec2::new(x, y);
    g.player.previous_pos = g.player.pos;
    g.world.entrance = g.player.pos;
    g.spawn("oHardBlock", Vec2::new(x - 8.0, y + 8.0));
    g.world.set_tile(10, 10, "oHardBlock");
    if wall {
        g.world.set_tile(9, 9, "oHardBlock");
    }
    if support {
        g.world.set_tile(9, 10, "oHardBlock");
    }
    if ceiling {
        g.world.set_tile(10, 8, "oHardBlock");
    }
    let id = g.spawn("oShopkeeper2", Vec2::new(160.0, 144.0));
    let e = g.world.entity(id).unwrap();
    assert_eq!(e.state, 2);
    assert_eq!(e.value("has_gun"), 0.0);
    assert_eq!(e.facing, -1);
    assert!(bottom(&g, e));
    assert_eq!(top_four(&g, e), ceiling);
    assert_eq!(bbox_solid_point(&g, Vec2::new(144.0, 144.0)), wall);
    assert_eq!(bbox_solid_point(&g, Vec2::new(144.0, 160.0)), support);
    (g, id)
}
fn first_callback(mut g: Game, id: u64, should_jump: bool, x: f64, y: f64) {
    let mut expected_rng = g.rng.clone();
    let expected_vy = if should_jump {
        -f64::from(7 + expected_rng.range(2))
    } else {
        0.0
    };
    g.step(&InputFrame::default());
    unharmed(&g);
    assert_eq!(
        g.player.pos,
        Vec2::new(x, y),
        "FIXTURE_THRESHOLD_PLAYER_STATIONARY"
    );
    let e = g.world.entity(id).unwrap();
    assert_eq!(
        e.pos,
        Vec2::new(160.0, 144.0),
        "FIXTURE_FIRST_CALLBACK_NO_DISPLACEMENT"
    );
    assert_eq!(e.state, 2);
    assert_eq!(e.facing, -1);
    assert_eq!(e.health, 20);
    assert_eq!(e.value("has_gun"), 0.0);
    assert!(!sound(&g, "sndShotgun"));
    assert!(
        (e.vel.y - expected_vy).abs() < EPS,
        "KEEPER_SOURCE_JUMP_CHAIN"
    );
    assert_eq!(
        g.rng.state_bits(),
        expected_rng.state_bits(),
        "KEEPER_EXACT_ZERO_OR_ONE_JUMP_DRAW"
    );
}

#[test]
fn close_lower_player_suppresses_both_wall_and_ledge_without_rng() {
    for rules in PRESETS {
        for wall in [false, true] {
            let (g, id) = birth_control(rules, 120.0, 152.0, wall, false, false);
            first_callback(g, id, false, 120.0, 152.0);
        }
    }
}
#[test]
fn strict_close_and_height_boundaries_preserve_wall_then_ledge_order() {
    for rules in PRESETS {
        // |dx|==64 is outside suppression. y==origin is also outside it.
        for (x, y, wall, jump) in [
            (104.0, 152.0, false, true),
            (120.0, 144.0, false, true),
            (104.0, 160.0, false, true),
            (104.0, 161.0, false, false),
            (104.0, 161.0, true, true),
        ] {
            let (g, id) = birth_control(rules, x, y, wall, false, false);
            first_callback(g, id, jump, x, y);
        }
    }
}
#[test]
fn exact_far_ledge_and_top_four_block_the_jump_and_rng() {
    for rules in PRESETS {
        // x-16 is supported while the old x-24 point lies in an empty cell.
        let (g, id) = birth_control(rules, 104.0, 136.0, false, true, false);
        assert!(!bbox_solid_point(&g, Vec2::new(136.0, 160.0)));
        first_callback(g, id, false, 104.0, 136.0);
        let (g, id) = birth_control(rules, 104.0, 152.0, true, false, true);
        first_callback(g, id, false, 104.0, 152.0);
    }
}

#[test]
fn ordinary_rear_whip_recovers_to_close_lower_ledge_without_jump() {
    for rules in PRESETS {
        // One finite source-derived right-side route. Initial Running pose is
        // authored; ATTACK, hit cooldown, shot, and recovery must be earned.
        let mut g = shell(rules);
        g.player.pos = Vec2::new(182.0, 152.0);
        g.player.previous_pos = g.player.pos;
        g.player.facing = 1;
        g.player.state = PlayerState::Running;
        g.player.vel.x = 4.0;
        g.world.entrance = g.player.pos;
        for x in 0..=12 {
            g.world.set_tile(x, 10, "oHardBlock");
        }
        // Same-height landing island across the single16pixel gap. This
        // leaves both keeper forward probes empty without trapping the hero.
        for x in 14..40 {
            g.world.set_tile(x, 10, "oHardBlock");
        }
        for x in 0..40 {
            g.world.set_tile(x, 14, "oHardBlock");
        }
        let id = g.spawn("oShopkeeper", Vec2::new(160.0, 144.0));
        assert_eq!(g.world.entity(id).unwrap().state, 0);
        let first_birth = g.world.next_id;
        let mut hit = false;
        for frame in 0..16 {
            g.step(&InputFrame {
                right: true,
                run: true,
                attack: frame == 0,
                attack_pressed: frame == 0,
                ..Default::default()
            });
            unharmed(&g);
            let e = g.world.entity(id).unwrap();
            if e.value("whipped") == 0.0 {
                assert_eq!(e.state, 0, "FIXTURE_ATTACK_EARNED_ONLY_BY_WHIP");
                continue;
            }
            assert_eq!(e.state, 2);
            assert_eq!(e.health, 20);
            assert_eq!(e.alarms[0], 10);
            assert!((e.vel.y + 2.0).abs() < EPS);
            assert!(sound(&g, "sndHit"));
            assert!(
                g.world
                    .entities
                    .iter()
                    .any(|e| e.id >= first_birth
                        && matches!(e.object.as_str(), "oWhip" | "oWhipPre"))
            );
            hit = true;
            break;
        }
        assert!(hit, "FIXTURE_REAL_REAR_WHIP_REQUIRED");
        // Same proven first-shot retreat, plus a tap jump to leave pellet height.
        let first_shot_birth = g.world.next_id;
        g.step(&InputFrame {
            right: true,
            run: true,
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        unharmed(&g);
        assert!(sound(&g, "sndShotgun"), "FIXTURE_REAL_FIRST_SHOT_REQUIRED");
        assert_eq!(g.world.entity(id).unwrap().value("firing"), 30.0);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.id >= first_shot_birth && e.object == "oBullet")
        );
        let mut reached = false;
        for frame in 0..24 {
            let rng_before = g.rng.state_bits();
            let keeper_alarm_before = g.world.entity(id).unwrap().alarms[0];
            // Keep the first3 recovery inputs running in the proven escape
            // direction; then walk right without changing Whip facing. Native
            // airborne motion retains its stored horizontal limit until landing.
            g.step(&InputFrame {
                right: true,
                run: frame < 3,
                ..Default::default()
            });
            unharmed(&g);
            let e = g.world.entity(id).unwrap();
            // Bullet-to-Solid also plays sndHit; only this receiver's cooldown
            // identifies a renewed Keeper Whip contact in the isolated route.
            assert!(
                e.alarms[0] <= keeper_alarm_before,
                "FIXTURE_NO_SECOND_WHIP_COOLDOWN_REFRESH"
            );
            assert_eq!(e.state, 2);
            assert_eq!(e.health, 20);
            assert!(!e.held);
            if !bottom(&g, e) || !g.player.grounded {
                continue;
            }
            // Landing on supported ground is not yet the jump decision under
            // test. Wait only for the fixed room's forward ledge predicate;
            // all survival guards remain mandatory on every callback.
            if bbox_solid_point(&g, Vec2::new(e.pos.x + 24.0, e.pos.y + 16.0)) {
                continue;
            }
            assert!(e.value("firing") > 0.0, "FIXTURE_RECOVERY_BEFORE_NEXT_SHOT");
            assert!(!sound(&g, "sndShotgun"));
            assert_eq!(e.facing, 1, "FIXTURE_FACING_UNCHANGED");
            assert!(
                (g.player.pos.x - (e.pos.x + 8.0)).abs() > 8.0,
                "FIXTURE_OUTSIDE_KEEPER_GRAB_RANGE"
            );
            assert!(g.player.pos.y > e.pos.y, "FIXTURE_SOURCE_NEAR_BELOW");
            assert!(
                (g.player.pos.x - (e.pos.x + 8.0)).abs() < 64.0,
                "FIXTURE_SOURCE_CLOSE"
            );
            assert!(g.player.pos.y <= e.pos.y + 8.0, "FIXTURE_OLD_HEIGHT_GATE");
            assert!(!top_four(&g, e), "FIXTURE_SOURCE_TOP4_CLEAR");
            let far_x = e.pos.x + 32.0;
            let source_wall = bbox_solid_point(&g, Vec2::new(far_x, e.pos.y));
            let source_ledge = g.player.pos.y <= e.pos.y + 16.0
                && !bbox_solid_point(&g, Vec2::new(far_x, e.pos.y + 16.0));
            assert!(
                source_wall || source_ledge,
                "FIXTURE_OTHERWISE_SOURCE_JUMP_BRANCH"
            );
            assert!(
                !bbox_solid_point(&g, Vec2::new(e.pos.x + 24.0, e.pos.y + 16.0)),
                "FIXTURE_OLD_LEDGE_BRANCH"
            );
            assert!(
                (e.vel.y).abs() < EPS,
                "KEEPER_NATURAL_NEAR_BELOW_SUPPRESSES_JUMP"
            );
            assert_eq!(
                g.rng.state_bits(),
                rng_before,
                "KEEPER_NATURAL_SUPPRESSION_NO_JUMP_DRAW"
            );
            reached = true;
            break;
        }
        assert!(
            reached,
            "FIXTURE_GROUNDED_RECOVERY_WITHIN_FINITE_ROUTE_REQUIRED"
        );
    }
}
