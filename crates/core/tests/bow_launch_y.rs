//! scrFireBow adds four pixels only for the two literal stationary duck sprites.
//! Authored room/character/actors, then public pickup, draw and release inputs.
use spelunky_core::characters::{self, Character};
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

#[derive(Clone, Copy)]
enum Pose {
    Standing,
    Duck,
    Crawl,
}

fn pickup_and_draw(rules: RulesPreset, character: Character, facing: i8) -> (Game, u64) {
    let mut game = Game::legacy_shell(42, rules);
    characters::apply(&mut game, character);
    game.world = World::new(80, 40, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(256.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    game.camera = Vec2::new(64.0, 0.0);
    game.previous_camera = game.camera;
    let bow = game.spawn("oBow", game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(bow),
        "ordinary Down+Attack picks up Bow"
    );
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.data["arrows"], 6.0);
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(bow));
    assert_eq!(game.player.data["bow_armed"], 1.0);
    assert!(game.player.data["bow_strength"] > 0.0);
    assert!(game.world.entities.iter().all(|e| e.object != "oArrow"));
    (game, bow)
}

fn pose_sprite(character: Character, pose: Pose) -> &'static str {
    match (character, pose) {
        (Character::Explorer, Pose::Standing) => "sStandLeft",
        (Character::Damsel, Pose::Standing) => "sDamselLeft",
        (Character::TunnelMan, Pose::Standing) => "sTunnelLeft",
        (Character::Explorer, Pose::Duck) => "sDuckLeft",
        (Character::Damsel, Pose::Duck) => "sDamselDuckL",
        (Character::TunnelMan, Pose::Duck) => "sTunnelDuckL",
        (Character::Explorer, Pose::Crawl) => "sCrawlLeft",
        (Character::Damsel, Pose::Crawl) => "sDamselCrawlL",
        (Character::TunnelMan, Pose::Crawl) => "sTunnelCrawlL",
    }
}

fn check_release(rules: RulesPreset, character: Character, facing: i8, pose: Pose) {
    let (mut game, bow) = pickup_and_draw(rules, character, facing);
    let moving = matches!(pose, Pose::Crawl);
    let down = !matches!(pose, Pose::Standing);
    let direction = InputFrame {
        down,
        left: moving && facing < 0,
        right: moving && facing > 0,
        ..Default::default()
    };
    let preparation = match pose {
        Pose::Standing => 0,
        Pose::Duck => 1,
        Pose::Crawl => 3,
    };
    for _ in 0..preparation {
        game.step(&InputFrame {
            attack: true,
            ..direction
        });
    }
    assert_eq!(
        game.player.sprite,
        pose_sprite(character, pose),
        "natural pre-release pose prerequisite"
    );
    assert_eq!(
        game.player.state,
        if down {
            PlayerState::Ducking
        } else {
            PlayerState::Standing
        }
    );
    if moving {
        assert!(game.player.vel.x.abs() > 0.0 && game.player.vel.x.abs() < 3.0);
    } else {
        assert_eq!(game.player.vel.x, 0.0);
    }
    assert_eq!(game.player.attack_timer, 0);
    let health = game.player.health;
    let expected_y_offset = if matches!(pose, Pose::Duck) && character != Character::TunnelMan {
        4.0
    } else {
        0.0
    };
    game.step(&InputFrame {
        attack_released: true,
        ..direction
    });
    assert_eq!(
        game.player.sprite,
        pose_sprite(character, pose),
        "release keeps the literal source pose"
    );
    assert_eq!(
        game.player.state,
        if down {
            PlayerState::Ducking
        } else {
            PlayerState::Standing
        }
    );
    assert_eq!(game.player.facing, facing);
    assert_eq!(game.player.held, Some(bow));
    assert_eq!(game.player.health, health);
    assert_eq!(game.data["arrows"], 5.0);
    assert_eq!(game.player.data["bow_armed"], 0.0);
    assert_eq!(game.player.data["bow_strength"], 0.0);
    let arrows: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oArrow")
        .collect();
    assert_eq!(arrows.len(), 1, "one ordinary release fires one real Arrow");
    let arrow = arrows[0];
    // A newborn's previous_pos records Create coordinates before its later
    // oArrow Step moves it horizontally and applies inherited gravity.
    assert_eq!(arrow.age, 0);
    assert_eq!(
        arrow.previous_pos.x,
        game.player.pos.x + f64::from(facing) * 14.0
    );
    assert_eq!(
        arrow.previous_pos.y,
        game.player.pos.y + expected_y_offset,
        "literal source duck-sprite launch height"
    );
    assert!(arrow.vel.x * f64::from(facing) > 0.0);
    assert_eq!(game.events.iter().filter(|e| matches!(e,
        GameEvent::Sound { name, .. } | GameEvent::SoundIfIdle { name, .. } if name == "sndArrowTrap"
    )).count(), 1);
}

#[test]
fn naturally_drawn_bow_stationary_duck_lowers_only_explorer_and_damsel_arrows() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for facing in [-1, 1] {
                check_release(rules, character, facing, Pose::Duck);
            }
        }
    }
}

#[test]
fn naturally_drawn_bow_crawling_release_keeps_arrow_at_player_height() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for facing in [-1, 1] {
                check_release(rules, character, facing, Pose::Crawl);
            }
        }
    }
}

#[test]
fn naturally_drawn_bow_standing_release_keeps_arrow_at_player_height() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for facing in [-1, 1] {
                check_release(rules, character, facing, Pose::Standing);
            }
        }
    }
}
