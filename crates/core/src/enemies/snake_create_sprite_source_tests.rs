//! oSnake's declared sprite survives Create; own Step selects walking art.
//! Constructor coverage includes runtime, generator and bare World adapters.
use super::*;
use crate::{Area, GenerationRevision, InputFrame, RngAlgorithm, RulesPreset};

#[test]
fn snake_create_keeps_declared_left_sprite_bounds_and_rng_until_ordinary_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for revision in [
                GenerationRevision::HistoricalV1,
                GenerationRevision::SourceClassicV2,
            ] {
                for producer in [0, 1, 2] {
                    let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
                    // This controls the generation API version only. The
                    // original object/Create sprite contract is unconditional.
                    g.generation_revision = revision;
                    g.world = World::new(20, 16, Area::Mines, 1);
                    g.world.name = "rLevel".into();
                    for x in 0..20 {
                        g.world.set_tile(x, 10, "oHardBlock");
                    }
                    g.player.pos = Vec2::new(24.0, 152.0);
                    g.player.previous_pos = g.player.pos;
                    g.world.entrance = g.player.pos;
                    let rng = g.rng.state_bits();
                    let id = match producer {
                        0 => g.spawn("oSnake", Vec2::new(80.0, 144.0)),
                        1 => crate::creation::spawn_generated_entity(
                            &mut g.world,
                            &mut g.rng,
                            "oSnake",
                            Vec2::new(80.0, 144.0),
                        ),
                        _ => g.world.spawn("oSnake", Vec2::new(80.0, 144.0)),
                    };
                    let e = g.world.entity(id).unwrap();
                    assert_eq!(
                        e.sprite, "sSnakeLeft",
                        "SNAKE_CREATE_SPRITE {algorithm:?} {revision:?} producer{producer}"
                    );
                    assert_eq!(e.animation, 0.0);
                    assert_eq!(e.value("animation_speed"), 0.4);
                    assert_eq!(
                        e.collision_mask().point_bounds(),
                        Rect::new(81.0, 148.0, 14.0, 12.0)
                    );
                    assert_eq!(e.health, 1);
                    assert_eq!(e.vel, Vec2::new(2.5, 0.0));
                    assert_eq!(g.rng.state_bits(), rng, "Snake has no Create RNG calls");
                    g.step(&InputFrame::default());
                    let e = g
                        .world
                        .entity(id)
                        .expect("ordinary first Snake Step remains alive");
                    assert_eq!(
                        e.sprite, "sSnakeWalkL",
                        "only the actual own Step selects walking art"
                    );
                    assert_eq!(e.animation, 0.4);
                    assert_eq!(e.health, 1);
                    assert_eq!(g.player.health, 4);
                }
            }
        }
    }
}
