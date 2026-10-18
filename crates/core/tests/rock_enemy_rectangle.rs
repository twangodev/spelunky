//! Ordinary pickup/throw against an animated enemy, without actor mutation.
use spelunky_core::*;

fn scene(rules: RulesPreset, bat_pos: Vec2) -> (Game, u64, u64) {
    let mut level = level::Level::default();
    level.set_tile(6, 7, '@').unwrap();
    for x in 0..level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "thrown-rock-source-rectangle", "");
    // Initial authored actor uses an ordinary source instance origin. The
    // two-pixel lower primary Bat reaches the frame1 aggregate-only edge.
    let bat = game.spawn("oBat", bat_pos);
    // Initial authored item overlaps the entrance, as source room instances may.
    let rock = game.spawn(
        "oRock",
        Vec2::new(game.player.pos.x, game.player.pos.y + 2.0),
    );
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert_eq!(game.player.health, 4.0);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(rock),
        "normal Down+Attack must pick up Rock"
    );
    game.step(&InputFrame {
        down: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(rock));
    (game, bat, rock)
}

fn throw_input(age: usize) -> InputFrame {
    InputFrame {
        attack: age == 0,
        attack_pressed: age == 0,
        ..Default::default()
    }
}

#[test]
fn naturally_thrown_rock_hits_frame_one_bat_at_source_aggregate_edge() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, bat_id, rock_id) = scene(rules, Vec2::new(160.0, 114.0));
        let mut saw_aggregate_only_contact = false;
        for age in 0..16 {
            let before = game.world.entity(bat_id).cloned().unwrap();
            game.step(&throw_input(age));
            let Some(bat) = game.world.entity(bat_id) else {
                break;
            };
            let rock = game.world.entity(rock_id).unwrap();
            assert!(!rock.held, "normal Attack must throw the held Rock");
            if before.health != 1 || bat.sprite != "sBatLeft" || before.sprite != "sBatLeft" {
                continue;
            }
            // Bat352 Step precedes Rock450. Its pose assignment preserves a
            // valid index; automatic image_speed0.5 advances only at endframe.
            let query_frame = before.animation.floor().rem_euclid(3.0) as usize;
            if query_frame != 1 || rock.vel.x.abs().max(rock.vel.y.abs()) <= 2.0 {
                continue;
            }
            assert_eq!(bat.pos.x.fract(), 0.0);
            assert_eq!(bat.pos.y.fract(), 0.0);
            // Independent original sprite/engine oracle: sBatLeft aggregate
            // [0,3,15,13], origin0,0. Frame1 is [0,4,15,11]. Prec0 queries
            // use the aggregate and raw inclusive Rock corners (+/-2).
            let source_hit = bat.pos.x + 16.0 > rock.pos.x - 2.0
                && bat.pos.x <= rock.pos.x + 2.0
                && bat.pos.y + 14.0 > rock.pos.y - 2.0
                && bat.pos.y + 3.0 <= rock.pos.y + 2.0;
            let legacy_frame_hit = bat.pos.x < rock.pos.x + 3.0
                && bat.pos.x + 16.0 > rock.pos.x - 2.0
                && bat.pos.y + 4.0 < rock.pos.y + 3.0
                && bat.pos.y + 12.0 > rock.pos.y - 2.0;
            if source_hit && !legacy_frame_hit {
                saw_aggregate_only_contact = true;
                assert_eq!(
                    bat.health, 0,
                    "{rules:?}/age{age}: source frame1 aggregate edge must damage; bat={:?}, rock={:?}",
                    bat.pos, rock.pos
                );
                break;
            }
            if bat.health <= 0 {
                break;
            }
        }
        assert!(
            saw_aggregate_only_contact,
            "{rules:?}: authored ordinary throw must exercise frame1 aggregate-only edge"
        );
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(bat_id).is_none(),
            "next Bat Step completes source death"
        );
    }
}

#[test]
fn ordinary_throw_still_hits_the_shared_enemy_body() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, bat, rock) = scene(rules, Vec2::new(160.0, 128.0));
        let mut hit = false;
        for age in 0..16 {
            game.step(&throw_input(age));
            assert!(!game.world.entity(rock).unwrap().held);
            if game.world.entity(bat).is_some_and(|e| e.health == 0) {
                hit = true;
                break;
            }
        }
        assert!(
            hit,
            "{rules:?}: shared-body ordinary Rock contact still damages"
        );
    }
}

#[test]
fn distant_bat_survives_the_same_ordinary_throw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, bat, rock) = scene(rules, Vec2::new(336.0, 112.0));
        for age in 0..12 {
            game.step(&throw_input(age));
            assert!(!game.world.entity(rock).unwrap().held);
            assert_eq!(game.world.entity(bat).unwrap().health, 1);
        }
    }
}
