//! Ordinary Rock pickup/throw at a naturally reached default Damsel Yell pose.
//! Initial room/actors only: no timer, animation, velocity, HP or state writes
//! to the Damsel. Item450 queries before Damsel556 advances its own Step.
use spelunky_core::{Area, Entity, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn first_throw_query(preset: RulesPreset, facing: i8, distance: f64) -> (Game, Entity, u64, u64) {
    let mut game = Game::shell(23, preset);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..24 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(if facing > 0 { 80.0 } else { 128.0 }, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    let damsel = game.spawn(
        "oDamsel",
        Vec2::new(game.player.pos.x + f64::from(facing) * distance, 152.0),
    );
    let rock = game.spawn("oRock", Vec2::new(game.player.pos.x, 156.0));
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(rock), "natural Rock pickup required");
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    for _ in 0..220 {
        let target = game.world.entity(damsel).unwrap();
        if target.sprite == "sDamselYellL" && target.animation.floor() == 0.0 {
            break;
        }
        game.step(&InputFrame::default());
    }
    let before = game.world.entity(damsel).unwrap().clone();
    assert_eq!(
        before.sprite, "sDamselYellL",
        "default idle must naturally reach YELL"
    );
    assert_eq!(
        before.animation.floor(),
        0.0,
        "throw must sample natural frame0"
    );
    assert_eq!(before.state, 3);
    assert_eq!(before.health, 4);
    assert!(!before.held);
    assert_eq!(before.value("invincible"), 0.0);
    assert_eq!(before.value("hit_cooldown"), 0.0);
    assert_eq!(
        before.value("image_xscale"),
        1.0,
        "Damsel keeps its own LEFT facing"
    );
    assert_eq!(before.pos.x.fract(), 0.0);
    assert_eq!(before.pos.y.fract(), 0.0);
    assert_eq!(game.player.held, Some(rock));
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.attack_timer, 0);
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.held.is_none(), "natural Attack must throw Rock");
    let thrown = game.world.entity(rock).unwrap();
    assert!(!thrown.held);
    assert!(thrown.vel.x * f64::from(facing) > 2.0);
    assert_eq!(game.player.health, 4.0);
    (game, before, damsel, rock)
}

fn source_and_legacy_query(before: &Entity, rock: &Entity) -> (bool, bool) {
    // Preserved sprite AUTO/RECTANGLE: origin(8,8), aggregate[1,0,15,15],
    // frame0[2,0,13,15]. Canonical0x4AC834 compares RAW inclusive query
    // corners to cached aggregate integer edges, independent of subimage.
    // All instance positions here are integers and Draw scale is +1.
    let x = before.pos.x;
    let y = before.pos.y;
    let first = Vec2::new(rock.pos.x - 2.0, rock.pos.y - 2.0);
    let second = Vec2::new(rock.pos.x + 2.0, rock.pos.y + 2.0);
    let source =
        x - 7.0 <= second.x && x + 7.0 >= first.x && y - 8.0 <= second.y && y + 7.0 >= first.y;
    // Original port's frame0 sprite_bounds intersected a half-open radius2
    // rectangle whose maximum was Rock+3, rather than the raw max Rock+2.
    let legacy = x - 6.0 < rock.pos.x + 3.0
        && x + 6.0 > first.x
        && y - 8.0 < rock.pos.y + 3.0
        && y + 8.0 > first.y;
    (source, legacy)
}

#[test]
fn naturally_thrown_rock_hits_yell_frame_zero_aggregate_only_damsel_edge() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (game, before, damsel, rock) = first_throw_query(preset, facing, 21.0);
            assert_eq!(
                source_and_legacy_query(&before, game.world.entity(rock).unwrap()),
                (true, false)
            );
            assert_eq!(
                game.world.entity(damsel).unwrap().health,
                3,
                "aggregate-only source query must damage"
            );
        }
    }
}

#[test]
fn naturally_thrown_rock_misses_damsel_one_pixel_outside_first_query() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (game, before, damsel, rock) = first_throw_query(preset, facing, 22.0);
            assert_eq!(
                source_and_legacy_query(&before, game.world.entity(rock).unwrap()),
                (false, false)
            );
            assert_eq!(
                game.world.entity(damsel).unwrap().health,
                4,
                "first throw query must leave outside target healthy"
            );
        }
    }
}

#[test]
fn naturally_thrown_rock_still_hits_damsel_shared_frame_body() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (game, before, damsel, rock) = first_throw_query(preset, facing, 19.0);
            assert_eq!(
                source_and_legacy_query(&before, game.world.entity(rock).unwrap()),
                (true, true)
            );
            assert_eq!(game.world.entity(damsel).unwrap().health, 3);
        }
    }
}
