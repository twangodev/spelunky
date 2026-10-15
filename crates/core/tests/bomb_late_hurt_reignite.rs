//! Authored default Bomb stock and supported initial pose, then ordinary inputs.
//! The raised native ArrowTrap/Jump route follows the previously qualified Bow
//! hurt fixture. Recovery and recollection use ordinary inputs.
use spelunky_core::{
    Game, GameEvent, GenerationRevision, InputFrame, PlayerState, RulesPreset, RunState, Vec2,
    level::Level,
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

fn diagnostic(game: &Game, bomb: u64) -> String {
    format!(
        "tick={} hero={:?}/{:?} hp={} stun={} state={:?} held={:?} bomb={:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.health,
        game.player.stun,
        game.player.state,
        game.player.held,
        game.world.entity(bomb).map(|item| (
            item.pos,
            item.vel,
            item.sprite.as_str(),
            item.value("armed"),
            item.alarms[0],
            item.alarms[1],
            item.held,
        )),
    )
}

fn sounds(game: &Game, wanted: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == wanted))
        .count()
}

fn room(rules: RulesPreset, facing: i8, historical: bool, trap: bool, stock: bool) -> Game {
    let mut level = Level::default();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    level.set_tile(14, 8, '@').unwrap();
    if trap {
        level
            .set_tile(
                if facing < 0 { 16 } else { 14 },
                7,
                if facing < 0 { '<' } else { '>' },
            )
            .unwrap();
    }
    let mut game = if historical {
        Game::legacy_shell(19, rules)
    } else {
        Game::shell(19, rules)
    };
    game.load_custom(&level, "bomb-late-hurt-source", "");
    // Explicit initial pose/stock only; no actor/timer/resource writes after Step.
    game.player.pos = Vec2::new(264.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    if stock {
        game.spawn("oBomb", Vec2::new(264.0, 156.0));
    }
    assert_eq!(
        game.generation_revision,
        if historical {
            GenerationRevision::HistoricalV1
        } else {
            GenerationRevision::SourceClassicV2
        }
    );
    game
}

fn stock_id(game: &Game) -> u64 {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|item| item.alive && item.object == "oBomb")
        .map(|item| item.id)
        .collect();
    assert_eq!(ids.len(), 1, "FIXTURE exactly one default Bomb stock");
    ids[0]
}

fn assert_inert(game: &Game, id: u64) {
    let bomb = game
        .world
        .entity(id)
        .expect("FIXTURE same inert Bomb survives");
    assert_eq!(bomb.object, "oBomb");
    assert_eq!(bomb.sprite, "sBomb");
    assert_eq!(bomb.alarms[0], -1, "FIXTURE no ordinary fuse started");
    assert_eq!(bomb.alarms[1], -1, "FIXTURE no fast fuse started");
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|item| item.alive && item.object == "oExplosion")
    );
}

fn pickup(game: &mut Game, bomb: u64, hp: i32) {
    let resources = game.player.bombs;
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded);
    assert!(game.player.held.is_none());
    assert_inert(game, bomb);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(bomb),
        "FIXTURE sameID real pickup: {}",
        diagnostic(game, bomb)
    );
    assert_eq!(game.player.pickup_item_type, "Bomb");
    assert!(game.world.entity(bomb).unwrap().held);
    assert_eq!(game.player.health, hp);
    assert_eq!(
        game.player.bombs, resources,
        "pickup is not a resource reservation"
    );
    game.step(&InputFrame::default());
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.health, hp);
    assert_eq!(game.player.held, Some(bomb));
    assert_inert(game, bomb);
}

fn unsafe_arrow(game: &Game) -> Option<u64> {
    game.world
        .entities
        .iter()
        .find(|item| {
            item.alive
                && item.engine_active
                && item.object == "oArrow"
                && item.value("thrown_by_player") == 0.0
                && item.value("safe") == 0.0
                && item.vel.x.abs() > 3.0
        })
        .map(|item| item.id)
}

fn real_hurt(game: &mut Game, bomb: u64, facing: i8) {
    for _ in 0..3 {
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.pos, Vec2::new(264.0, 152.0));
        assert_eq!(game.player.held, Some(bomb));
        assert_inert(game, bomb);
    }
    assert!(unsafe_arrow(game).is_none());
    let mut launched = None;
    for tick in 0..8 {
        game.step(&InputFrame {
            jump: true,
            jump_pressed: tick == 0,
            ..Default::default()
        });
        if tick == 0 {
            assert!(game.player.pos.y < 152.0 && game.player.vel.y < 0.0);
            assert!(!game.player.grounded);
            assert_eq!(sounds(game, "sndJump"), 1);
        }
        if let Some(id) = unsafe_arrow(game) {
            let arrow = game.world.entity(id).unwrap();
            assert_eq!(
                arrow.pos,
                Vec2::new(if facing > 0 { 258.0 } else { 270.0 }, 132.0)
            );
            assert_eq!(
                arrow.vel,
                Vec2::new(if facing > 0 { 8.0 } else { -8.0 }, 0.0)
            );
            assert_eq!(arrow.age, 0);
            assert_eq!(
                game.player.health, 4,
                "FIXTURE trap birth follows Player hurt query"
            );
            assert_eq!(sounds(game, "sndArrowTrap"), 1);
            launched = Some(id);
            break;
        }
        assert_eq!(game.player.health, 4);
    }
    let arrow = launched.expect("FIXTURE native raised trap fired within8 Jump inputs");
    for _ in 0..100 {
        game.step(&InputFrame {
            jump: true,
            ..Default::default()
        });
        if game.player.health < 4 {
            assert_eq!(game.player.health, 2, "FIXTURE first real Arrow hurt");
            assert_eq!(game.player.stun, 20);
            assert!(game.player.is_stunned());
            assert!(game.world.entity(arrow).is_none());
            assert_eq!(sounds(game, "sndHurt"), 1);
            assert!(game.player.held.is_none());
            assert!(game.player.pickup_item_type.is_empty());
            let released = game
                .world
                .entity(bomb)
                .expect("FIXTURE original Bomb released alive");
            assert!(!released.held);
            assert_eq!(
                released.value("armed"),
                1.0,
                "FIXTURE literal late hurt write"
            );
            assert_inert(game, bomb);
            return;
        }
        assert_eq!(game.state, RunState::Playing);
    }
    panic!(
        "FIXTURE real Arrow hurt timeout: {}",
        diagnostic(game, bomb)
    );
}

fn recover_and_recollect(game: &mut Game, bomb: u64) {
    // Fixed recovery tape, no state/timer rewrites and no outcome-picked seed.
    for _ in 0..160 {
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 2, "FIXTURE no second injury");
        assert_eq!(game.state, RunState::Playing);
        assert_inert(game, bomb);
    }
    assert!(
        !game.player.is_stunned(),
        "FIXTURE full natural recovery: {}",
        diagnostic(game, bomb)
    );
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.world.entity(bomb).unwrap().vel, Vec2::default());
    // Predeclared finite feedback tape: three direction frames then12 neutral
    // frames per pulse. Reads positions to choose an ordinary Left/Right input;
    // never changes positions, velocity, inventory, actor fields or timers.
    for _ in 0..64 {
        let dx = game.world.entity(bomb).unwrap().pos.x - game.player.pos.x;
        if dx.abs() <= 4.0 {
            break;
        }
        for _ in 0..3 {
            game.step(&InputFrame {
                left: dx < 0.0,
                right: dx > 0.0,
                ..Default::default()
            });
            assert_eq!(game.player.health, 2);
            assert_inert(game, bomb);
        }
        for _ in 0..12 {
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 2);
            assert_inert(game, bomb);
        }
        assert_eq!(
            game.player.vel.x, 0.0,
            "FIXTURE walking pulse naturally settles"
        );
        assert_eq!(game.player.state, PlayerState::Standing);
    }
    let loose = game.world.entity(bomb).unwrap();
    assert!(
        (loose.pos.x - game.player.pos.x).abs() <= 4.0,
        "FIXTURE finite same-Bomb approach: {}",
        diagnostic(game, bomb)
    );
    assert!(!loose.held);
    assert_eq!(loose.value("armed"), 1.0);
    pickup(game, bomb, 2);
    assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 1.0);
    assert_inert(game, bomb);
}

#[test]
fn source_same_bomb_can_ignite_after_real_hurt_recovery_and_pickup() {
    for rules in PRESETS {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, false, true, true);
            let bomb = stock_id(&game);
            assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
            pickup(&mut game, bomb, 4);
            real_hurt(&mut game, bomb, facing);
            recover_and_recollect(&mut game, bomb);
            let resources = game.player.bombs;
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            // Primary visible first-use consequence precedes hidden fuse metadata.
            assert_eq!(
                game.world.entity(bomb).unwrap().sprite,
                "sBombArmed",
                "CONTRACT_SOURCE_SPRITE_GATED_BOMB_IGNITES_AFTER_REAL_HURT: {}",
                diagnostic(&game, bomb)
            );
            assert_eq!(game.player.held, Some(bomb));
            assert_eq!(game.world.entity(bomb).unwrap().alarms[0], 80);
            assert_eq!(game.world.entity(bomb).unwrap().alarms[1], -1);
            assert_eq!(game.world.entity(bomb).unwrap().image_speed(), 0.2);
            assert_eq!(game.player.bombs, resources);
            assert_eq!(game.player.health, 2);
            assert_eq!(sounds(&game, "sndThrow"), 0);
        }
    }
}

#[test]
fn historical_same_hurt_bomb_retains_existing_armed_property_guard() {
    for rules in PRESETS {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, true, true, true);
            let bomb = stock_id(&game);
            pickup(&mut game, bomb, 4);
            real_hurt(&mut game, bomb, facing);
            recover_and_recollect(&mut game, bomb);
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.world.entity(bomb).unwrap().sprite, "sBomb");
            assert_inert(&game, bomb);
            assert_eq!(game.player.held, Some(bomb));
        }
    }
}

#[test]
fn ordinary_bomb_first_use_then_throw_keeps_existing_live_fuse() {
    for historical in [false, true] {
        for rules in PRESETS {
            for facing in [-1, 1] {
                let mut game = room(rules, facing, historical, false, true);
                let bomb = stock_id(&game);
                pickup(&mut game, bomb, 4);
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert_eq!(game.world.entity(bomb).unwrap().sprite, "sBombArmed");
                assert_eq!(game.world.entity(bomb).unwrap().alarms[0], 80);
                assert_eq!(game.player.held, Some(bomb));
                game.step(&InputFrame::default());
                let remaining = game.world.entity(bomb).unwrap().alarms[0];
                assert_eq!(remaining, 79);
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert!(game.player.held.is_none(), "second use is a real throw");
                let thrown = game.world.entity(bomb).unwrap();
                assert!(!thrown.held);
                assert_eq!(thrown.sprite, "sBombArmed");
                assert_eq!(
                    thrown.alarms[0],
                    remaining - 1,
                    "running fuse is not restarted"
                );
                assert_eq!(sounds(&game, "sndThrow"), 1);
            }
        }
    }
}

#[test]
fn dedicated_bomb_button_keeps_its_existing_newborn_arming_contract() {
    for historical in [false, true] {
        for rules in PRESETS {
            for facing in [-1, 1] {
                let mut game = room(rules, facing, historical, false, false);
                let resources = game.player.bombs;
                game.step(&InputFrame {
                    bomb_pressed: true,
                    ..Default::default()
                });
                let bomb = game.world.entity(stock_id(&game)).unwrap();
                assert_eq!(bomb.sprite, "sBombArmed");
                assert_eq!(bomb.alarms[0], 80);
                assert_eq!(bomb.image_speed(), 0.2);
                assert_eq!(
                    bomb.timer, 120,
                    "existing dedicated compatibility bookkeeping"
                );
                assert_eq!(game.player.bombs, resources - 1);
                assert!(game.player.held.is_none());
                assert_eq!(sounds(&game, "sndThrow"), 1);
            }
        }
    }
}
