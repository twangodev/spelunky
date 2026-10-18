//! Literal AlienBoss death-entry depth and independent post-death counter tail.
//! Requires the separately accepted constructor and cached-distance fixes.
//! Same ordinary Boulder/Right9 producer; no post-start actor or RNG writes.
use spelunky_core::{Area, CollisionPrecision, Game, InputFrame, RulesPreset, Vec2, World};

fn room(rules: RulesPreset, falling_boulder: bool) -> (Game, u64, Option<u64>, f64) {
    let mut game = Game::shell(7, rules);
    game.world = World::new(48, 16, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBlock");
    }
    // Authored Player birth position. Its source-default Falling state and
    // zero velocity settle naturally on floor160; no state/phase override.
    game.player.pos = Vec2::new(208.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let boss = game.spawn("oAlienBoss", Vec2::new(352.0, 128.0));
    let actor = game.world.entity(boss).unwrap();
    assert_eq!(actor.health, 10);
    assert_eq!(actor.state, 0);
    assert_eq!(actor.facing, -1);
    assert_eq!(actor.sprite, "sAlienBoss");
    assert_eq!(actor.image_speed(), 0.25);
    assert_eq!(actor.value("psychic_recover"), 100.0);
    assert_eq!(actor.vel.y, 0.0);
    // Permit the recorded old missing constructor value here so the old
    // executable must reach the actual crush before the causal assertion.
    let birth_x_velocity = actor.vel.x;
    assert!(birth_x_velocity == 0.0 || birth_x_velocity == 2.5);
    let boulder = falling_boulder.then(|| game.spawn("oBoulder", Vec2::new(352.0, 96.0)));
    if let Some(id) = boulder {
        let block = game.world.entity(id).unwrap();
        assert_eq!(block.pos, Vec2::new(352.0, 96.0));
        assert_eq!(block.vel, Vec2::default());
        assert_eq!(block.value("bounced"), 0.0);
        assert_eq!(block.value("invincible"), 1.0);
    }
    (game, boss, boulder, birth_x_velocity)
}

fn approach_first_boss_step(game: &mut Game, boss: u64, boulder: Option<u64>, birth_vx: f64) {
    // Independently read source gameStepEvent + Boulder own moveTo both move
    // the same velocity before own gravity. Normal object IDs Player319,
    // Boulder521, Boss658 keep the falling Solid ahead of the Boss.
    let player_x = [208.0, 208.0, 210.0, 213.0, 216.0, 219.0, 222.0, 225.0];
    let boulder_y = [96.0, 96.0, 98.0, 102.0, 108.0, 114.0, 122.0, 130.0];
    let camera_x = [16.0, 16.0, 18.0, 21.0, 24.0, 27.0, 30.0, 33.0];
    for tick in 0..8 {
        let actor = game.world.entity(boss).unwrap();
        assert!(
            actor.pos.x >= game.camera.x + 320.0,
            "Boss own Step is still outside its strict viewport on entry"
        );
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(game.player.pos, Vec2::new(player_x[tick], 152.0));
        assert_eq!(game.camera.x, camera_x[tick]);
        assert_eq!(game.player.health, 4.0);
        assert_eq!(game.player.stun, 0);
        assert_eq!(game.player.invincible, 0);
        let actor = game.world.entity(boss).unwrap();
        assert!(actor.alive && actor.engine_active);
        assert_eq!(actor.pos, Vec2::new(352.0, 128.0));
        assert_eq!(actor.health, 10);
        assert_eq!(actor.state, 0);
        assert_eq!(
            actor.value("psychic_recover"),
            100.0,
            "no Boss own Step has run"
        );
        assert_eq!(
            actor.vel.x, birth_vx,
            "the initial incoming velocity remains untouched"
        );
        assert_eq!(actor.vel.y, 0.0);
        if let Some(id) = boulder {
            let block = game.world.entity(id).unwrap();
            assert!(block.alive && block.engine_active);
            assert_eq!(block.pos, Vec2::new(352.0, boulder_y[tick]));
            assert_eq!(block.vel.x, 0.0);
            assert_eq!(
                block.value("bounced"),
                0.0,
                "no earlier floor bounce supplies a different producer"
            );
        }
    }
    assert_eq!(game.engine_time(), 9);
    assert_eq!(game.camera.x, 33.0);
    let actor = game.world.entity(boss).unwrap();
    assert!(
        actor.pos.x < game.camera.x + 320.0,
        "ordinary camera following now opens the first Boss own Step"
    );
    if let Some(id) = boulder {
        assert!(
            game.world
                .entity(id)
                .unwrap()
                .mask_contains(Vec2::new(360.0, 136.0), CollisionPrecision::BoundingBox)
        );
        assert!(game.world.solid_point_except(Vec2::new(360.0, 136.0), boss));
    } else {
        assert!(!game.world.solid_point_except(Vec2::new(360.0, 136.0), boss));
    }
}

fn effective_depth(game: &Game, id: u64) -> f64 {
    let actor = game.world.entity(id).unwrap();
    actor
        .data
        .get("depth")
        .copied()
        .unwrap_or_else(|| f64::from(spelunky_core::catalog::object(&actor.object).unwrap().depth))
}

fn ordinary_crush(rules: RulesPreset) -> (Game, u64) {
    let (mut game, boss, boulder, birth_vx) = room(rules, true);
    assert_eq!(birth_vx, 2.5, "SEPARATELY_QUALIFIED_CONSTRUCTOR_DEPENDENCY");
    assert_eq!(effective_depth(&game, boss), 60.0);
    approach_first_boss_step(&mut game, boss, boulder, birth_vx);
    game.step(&InputFrame {
        right: true,
        ..Default::default()
    });
    assert_eq!(game.engine_time(), 10);
    assert_eq!(game.player.pos, Vec2::new(228.0, 152.0));
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.invincible, 0);
    let block = game.world.entity(boulder.unwrap()).unwrap();
    assert_eq!(block.pos, Vec2::new(352.0, 140.0));
    assert_eq!(block.vel.x, 0.0);
    assert_eq!(block.value("bounced"), 0.0);
    assert!(block.mask_contains(Vec2::new(360.0, 136.0), CollisionPrecision::BoundingBox));
    let actor = game.world.entity(boss).unwrap();
    assert_eq!(actor.health, 0, "ACTUAL_ORDINARY_BOULDER_CRUSH");
    assert_eq!(actor.state, 99);
    assert_eq!(actor.sprite, "sAlienBossDie");
    assert_eq!(actor.pos, Vec2::new(354.0, 126.0));
    assert_eq!(actor.vel.x, 2.4);
    assert_eq!(game.data.get("alienbosses"), Some(&1.0));
    assert_eq!(game.data.get("kills"), Some(&1.0));
    (game, boss)
}

#[test]
fn ordinary_boulder_crush_sets_alien_boss_death_depth_before_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (game, boss) = ordinary_crush(rules);
        assert_eq!(
            effective_depth(&game, boss),
            101.0,
            "SOURCE_ALIEN_BOSS_DEATH_DEPTH_101"
        );
        assert_eq!(
            game.world.entity(boss).unwrap().data.get("depth"),
            Some(&101.0)
        );
        // Scene consumes this same dynamic depth before its larger-first sort.
        // Draw is frozen before Animation; the presented corpse retains it.
        let drawn = game.presentation().world.entity(boss).unwrap();
        assert_eq!(drawn.data.get("depth"), Some(&101.0));
    }
}

#[test]
fn ordinary_boulder_crush_continues_alien_boss_recovery_counter_while_dead() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, boss) = ordinary_crush(rules);
        let actor = game.world.entity(boss).unwrap();
        assert_eq!(
            actor.value("psychic_recover"),
            99.0,
            "SOURCE_ALIEN_BOSS_DEAD_COUNTER_CONTINUES"
        );
        // Previous Right Draw supplies signed -1. At this source Step, Player
        // Running aggregate ends at235, and moved Boss Die starts at355.
        assert_eq!(game.player.sprite, "sRunLeft");
        assert_eq!(game.player.data.get("image_xscale"), Some(&-1.0));
        assert_eq!(
            actor.value("dist"),
            120.0,
            "source scratch distance is computed after movement"
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oPsychicCreate" | "oPsychicWave"))
        );
        assert!(actor.pos.x < game.camera.x + 320.0);
        game.step(&InputFrame::default());
        let actor = game.world.entity(boss).unwrap();
        assert_eq!(actor.health, 0);
        assert_eq!(actor.state, 99);
        assert_eq!(actor.value("psychic_recover"), 98.0);
        assert_eq!(effective_depth(&game, boss), 101.0);
        assert_eq!(game.player.health, 4.0);
        assert_eq!(game.data.get("alienbosses"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oPsychicCreate" | "oPsychicWave"))
        );
    }
}

#[test]
fn ordinary_healthy_alien_boss_retains_live_depth_and_positive_recovery_timing() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, boss, boulder, birth_vx) = room(rules, false);
        assert_eq!(birth_vx, 2.5, "SEPARATELY_QUALIFIED_CONSTRUCTOR_DEPENDENCY");
        approach_first_boss_step(&mut game, boss, boulder, birth_vx);
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        let actor = game.world.entity(boss).unwrap();
        assert_eq!(actor.health, 10);
        assert_eq!(actor.state, 0);
        assert_eq!(actor.pos, Vec2::new(352.0, 128.0));
        assert_eq!(actor.vel, Vec2::default());
        assert_eq!(actor.sprite, "sAlienBoss");
        assert_eq!(actor.value("psychic_recover"), 99.0);
        assert_eq!(effective_depth(&game, boss), 60.0);
        assert_eq!(game.data.get("alienbosses").copied().unwrap_or(0.0), 0.0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oPsychicCreate" | "oPsychicWave"))
        );
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(boss).unwrap().value("psychic_recover"),
            98.0
        );
        assert_eq!(effective_depth(&game, boss), 60.0);
        assert_eq!(game.player.health, 4.0);
    }
}
