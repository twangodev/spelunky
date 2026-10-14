//! Native default actors in an authored initial room; one neutral public Step.
//! PushBlock's real first gravity/sink pass supplies the fractional surface.
//! No configured masks/scale/state/velocity or writes after the first Step.
use spelunky_core::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Roof {
    SinkingNearMiss,
    IntegerNearMiss,
    SinkingContact,
}

fn sound_count(game: &Game, sound: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == sound))
        .count()
}

fn check(object: &str, roof: Roof) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            game.world = World::new(24, 20, Area::Jungle, 1);
            game.world.name = "rLevel".into();
            // Player stays dry/supported; the enemy is initially unsupported.
            for x in 8..game.world.width {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            game.player.pos = Vec2::new(136.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            game.world.entrance = game.player.pos;
            let monkey = object == "oMonkey";
            // Monkey controller top150 queries149; Frog top148 queries147.
            let top_y = if monkey { 149.0 } else { 147.0 };
            let block_y = top_y - 17.0 + f64::from(roof == Roof::SinkingContact);
            let block = game.spawn("oPushBlock", Vec2::new(107.0, block_y));
            if roof != Roof::IntegerNearMiss {
                // Offset native Lava covers only the block centre115. Enemy
                // centre104 and Player132..139 remain outside its112..127 bbox.
                game.spawn(
                    "oLava",
                    Vec2::new(112.0, if monkey { 144.0 } else { 140.0 }),
                );
            }
            let id = game.spawn(object, Vec2::new(96.0, 144.0));
            let born = game.world.entity(id).unwrap();
            assert_eq!(
                (born.health, born.state, born.timer),
                (1, if monkey { 5 } else { 0 }, 0)
            );
            assert_eq!(born.vel, Vec2::default());
            assert_eq!(
                born.sprite,
                if monkey { "sMonkeyLeft" } else { "sFrogLeft" }
            );
            assert_eq!(
                born.bounds(),
                Rect::new(
                    100.0,
                    if monkey { 150.0 } else { 148.0 },
                    8.0,
                    if monkey { 10.0 } else { 12.0 }
                )
            );
            assert_eq!(game.world.entity(block).unwrap().vel, Vec2::default());
            assert_eq!(game.world.entity(block).unwrap().value("push_gravity"), 0.6);
            assert_eq!(game.world.entity(block).unwrap().sprite, "sBlock");
            let original_rng = game.rng.state_bits();

            game.step(&InputFrame::default());
            let e = game.world.entity(id).expect("native actor survives");
            let solid = game.world.entity(block).expect("native block survives");
            assert_eq!(game.engine_time(), 2);
            assert_eq!(e.pos, Vec2::new(96.0, 144.0));
            assert_eq!(e.previous_pos, e.pos);
            assert!(e.alive && e.engine_active && !e.held);
            assert_eq!(e.health, 1);
            assert_eq!((e.state, e.timer), (if monkey { 6 } else { 1 }, 0));
            assert_eq!(e.vel.x, 0.0);
            assert_eq!(e.value("swimming"), 0.0);
            assert_eq!(e.value("burning"), 0.0);
            assert_eq!(e.image_speed(), 0.4);
            assert_eq!(e.value("inherited_gravity"), if monkey { 0.6 } else { 0.2 });
            assert_eq!(game.player.health, 4);
            assert_eq!(game.player.pos, Vec2::new(136.0, 152.0));
            assert!(game.player.grounded && game.player.held.is_none());
            assert_eq!(game.player.stun, 0);
            let view = game.simulation_view_rect();
            assert!(e.pos.x > view.x - 20.0 && e.pos.x < view.x + view.w + 4.0);
            assert!(e.pos.y > view.y - 20.0 && e.pos.y < view.y + view.h + 4.0);
            assert_eq!(
                game.rng.state_bits(),
                original_rng,
                "SOURCE_FIRST_STATE_BRANCH_NO_DRAW"
            );
            let sink = if roof == Roof::IntegerNearMiss {
                0.0
            } else {
                0.05
            };
            assert!((solid.pos.y - (block_y + 1.0 + sink)).abs() < 1e-10);
            assert_eq!(solid.pos.x, 107.0);
            assert_eq!(
                solid.value("push_gravity"),
                if roof == Roof::IntegerNearMiss {
                    0.6
                } else {
                    0.0
                }
            );
            assert_eq!(
                solid.vel.y,
                if roof == Roof::IntegerNearMiss {
                    0.6
                } else {
                    0.0
                }
            );
            assert!(
                !game.world.solid_at(104.0, 152.0),
                "SOURCE_OWN_CRUSH_CENTRE_CLEAR"
            );
            assert!(!game.world.solid_line_except(
                Vec2::new(108.0, top_y + 1.0),
                Vec2::new(108.0, 159.0),
                id
            ));
            assert!(!game.world.solid_line_except(
                Vec2::new(99.0, top_y + 1.0),
                Vec2::new(99.0, 159.0),
                id
            ));
            assert!(!game.world.solid_line_except(
                Vec2::new(100.0, 160.0),
                Vec2::new(107.0, 160.0),
                id
            ));
            assert_eq!(sound_count(&game, "sndFrog"), usize::from(!monkey));
            assert_eq!(sound_count(&game, "sndMonkey"), 0);
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| matches!(e.object.as_str(), "oBlood" | "oSplash" | "oBurn"))
            );
            // Source final top script recalculates these exact current raw
            // controller endpoints. Source image bounds never select this line.
            assert_eq!(
                game.world
                    .solid_line_except(Vec2::new(100.0, top_y), Vec2::new(107.0, top_y), id),
                roof == Roof::SinkingContact,
                "SOURCE_NATIVE_TOP_CONTACT_PREREQUISITE"
            );
            assert_eq!(
                solid.mask_intersects(
                    Rect::new(100.0, top_y, 8.0, 1.0),
                    CollisionPrecision::Precise
                ),
                roof != Roof::IntegerNearMiss,
                "OLDER_FLOAT_TOP_RECTANGLE_PREREQUISITE"
            );
            // All source admission/branch/RNG/geometry guards precede effect.
            assert_eq!(
                e.vel.y,
                if roof == Roof::SinkingContact {
                    1.0
                } else if monkey {
                    0.0
                } else {
                    0.2
                },
                "SOURCE_JUNGLE_FINAL_NATIVE_TOP_CLAMP object={object} roof={roof:?} rules={rules:?} algorithm={algorithm:?}"
            );
            assert_eq!(
                e.sprite,
                if monkey {
                    "sMonkeyClimbL"
                } else {
                    "sFrogJumpL"
                }
            );
        }
    }
}

#[test]
fn default_monkey_sinking_roof_near_miss_keeps_climb_velocity_zero() {
    check("oMonkey", Roof::SinkingNearMiss);
}
#[test]
fn default_monkey_integer_roof_near_miss_agrees() {
    check("oMonkey", Roof::IntegerNearMiss);
}
#[test]
fn default_monkey_native_roof_contact_clamps_downward() {
    check("oMonkey", Roof::SinkingContact);
}
#[test]
fn default_frog_sinking_roof_near_miss_keeps_first_bounce_gravity() {
    check("oFrog", Roof::SinkingNearMiss);
}
#[test]
fn default_frog_integer_roof_near_miss_agrees() {
    check("oFrog", Roof::IntegerNearMiss);
}
#[test]
fn default_frog_native_roof_contact_clamps_downward() {
    check("oFrog", Roof::SinkingContact);
}
