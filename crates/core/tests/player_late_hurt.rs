//! Source-derived fixtures, UNEXECUTED. No runtime or timing claim.
//! The Arrow route authors only room objects; all projectiles come from traps.
use spelunky_core::*;

fn room(rules: RulesPreset, character: characters::Character, trap: bool, wall: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(10, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    if wall {
        for x in 11..=13 {
            for y in 7..=8 {
                level.set_tile(x, y, '1').unwrap();
            }
        }
    }
    if trap {
        level.set_tile(6, 7, '>').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "late-player-hurt", "");
    characters::apply(&mut game, character);
    game
}

fn diagnostic(game: &Game) -> String {
    format!(
        "tick={} pos={:?} vel={:?} hp={} state={:?} sprite={} frame={} stun={} duck={:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.health,
        game.player.state,
        game.player.sprite,
        game.player.animation,
        game.player.stun,
        game.player.data.get("duck_to_hang")
    )
}

fn hanging_sprite(game: &Game) -> bool {
    matches!(
        game.player.sprite.as_str(),
        "sHangLeft" | "sDamselHangL" | "sTunnelHangL"
    )
}

fn attack_sprite(game: &Game) -> bool {
    matches!(
        game.player.sprite.as_str(),
        "sAttackLeft" | "sDamselAttackL" | "sTunnelAttackL"
    )
}

fn settle(game: &mut Game) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "FIXTURE_GROUND: {}", diagnostic(game));
    assert!(!game.player.is_stunned());
}

fn natural_trap_hits_hanging_player(game: &mut Game) {
    settle(game);
    let health = game.player.health;
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.object == "oArrowTrapRight")
            || game
                .world
                .tiles
                .iter()
                .any(|tile| tile.object == "oArrowTrapRight")
    );
    let mut saw_hanging = false;
    let mut saw_trap_arrow = false;
    let mut trace = Vec::new();
    for tick in 0..60 {
        let was_hanging = game.player.state == PlayerState::Hanging;
        game.step(&InputFrame {
            right: true,
            jump: true,
            jump_pressed: tick == 0,
            ..Default::default()
        });
        trace.push(diagnostic(game));
        saw_trap_arrow |= game
            .world
            .entities
            .iter()
            .any(|entity| entity.object == "oArrow");
        if game.player.health < health {
            assert!(
                saw_trap_arrow,
                "FIXTURE_PRODUCER: no observed natural Arrow birth: {trace:#?}"
            );
            assert!(
                saw_hanging && was_hanging,
                "FIXTURE_TIMING: real hit must interrupt a live natural hang: {trace:#?}"
            );
            assert_eq!(
                game.player.health,
                health - 2,
                "real Arrow hurt: {trace:#?}"
            );
            assert_eq!(game.player.stun, 20, "real Arrow stun: {trace:#?}");
            assert!(
                !hanging_sprite(game),
                "CONTRACT: own-Step hurt must select knockback art on the hit frame: {trace:#?}"
            );
            assert!(!attack_sprite(game));
            return;
        }
        assert_eq!(
            game.state,
            RunState::Playing,
            "FIXTURE_SURVIVAL: {trace:#?}"
        );
        saw_hanging |= game.player.state == PlayerState::Hanging;
    }
    panic!("FIXTURE_TIMING: authored trap never hit a naturally hanging player: {trace:#?}");
}

#[test]
fn trap_produced_arrow_changes_hanging_art_on_its_own_player_step_then_controls_recover() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            let mut game = room(rules, character, true, true);
            assert_eq!(game.player.health, 4);
            natural_trap_hits_hanging_player(&mut game);
            assert_eq!(game.player.health, 2);
            for _ in 0..240 {
                if !game.player.is_stunned() && game.player.grounded && game.player.vel.y == 0.0 {
                    break;
                }
                game.step(&InputFrame::default());
                assert_eq!(
                    game.state,
                    RunState::Playing,
                    "recovery: {}",
                    diagnostic(&game)
                );
            }
            assert!(!game.player.is_stunned(), "recovery: {}", diagnostic(&game));
            assert!(game.player.grounded);
            game.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert!(
                game.player.vel.y < 0.0,
                "real jump after recovery: {}",
                diagnostic(&game)
            );
        }
    }
}

#[test]
fn source_two_health_tunnelman_uses_late_hurt_art_before_fatal_dead_latch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, characters::Character::TunnelMan, true, true);
        assert_eq!(game.player.health, 2, "natural selected-character stats");
        natural_trap_hits_hanging_player(&mut game);
        assert_eq!(game.player.health, 0);
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.player.state, PlayerState::Dead);
    }
}

#[test]
fn no_trap_natural_grab_control_keeps_hanging_art() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = room(rules, character, false, true);
            settle(&mut game);
            let health = game.player.health;
            let mut grabbed = false;
            for tick in 0..40 {
                game.step(&InputFrame {
                    right: true,
                    jump: true,
                    jump_pressed: tick == 0,
                    ..Default::default()
                });
                assert_eq!(game.player.health, health);
                assert!(!game.player.is_stunned());
                if game.player.state == PlayerState::Hanging {
                    grabbed = true;
                    assert!(hanging_sprite(&game));
                    break;
                }
            }
            assert!(grabbed, "FIXTURE_GRAB: {}", diagnostic(&game));
        }
    }
}

#[test]
fn ordinary_attacks_birth_source_sequential_parts_without_duplicates() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = room(rules, character, false, false);
            settle(&mut game);
            let tunnelman = character == characters::Character::TunnelMan;
            let pre = if tunnelman { "oMattockPre" } else { "oWhipPre" };
            let front = if tunnelman { "oMattockHit" } else { "oWhip" };
            let mut pre_ids = std::collections::BTreeSet::new();
            let mut front_ids = std::collections::BTreeSet::new();
            let mut sounds = 0;
            for tick in 0..24 {
                game.step(&InputFrame {
                    attack: tick == 0,
                    attack_pressed: tick == 0,
                    ..Default::default()
                });
                for entity in &game.world.entities {
                    if entity.object == pre {
                        pre_ids.insert(entity.id);
                    }
                    if entity.object == front {
                        front_ids.insert(entity.id);
                        assert_eq!(entity.value("damage"), if tunnelman { 2.0 } else { 1.0 });
                    }
                }
                // The source queries the live oWhip family each Step. A retired
                // first MattockHit permits a later hit, but never two live hits.
                assert!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == front)
                        .count()
                        <= 1,
                    "no simultaneous forward parts {character:?}/{rules:?} tick {tick}"
                );
                sounds += game
                    .events
                    .iter()
                    .filter(
                        |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndWhip"),
                    )
                    .count();
            }
            assert_eq!(pre_ids.len(), 1, "normal pre {character:?}/{rules:?}");
            // Source Player Step tests image_index > 4 and live oWhip count
            // every frame. The 3-frame MattockHit advances by 0.5, including
            // its birth frame, and retires after six advances. The 11-frame
            // Tunnel attack starts at 0.6 then advances by 1: first birth at
            // 4.6, retirement while Player reaches 10.6, then a second birth
            // before Player AnimationEnd. Ordinary Whip survives that window.
            let expected_front_parts = if tunnelman { 2 } else { 1 };
            assert_eq!(
                front_ids.len(),
                expected_front_parts,
                "normal front {character:?}/{rules:?}"
            );
            assert_eq!(
                sounds, expected_front_parts,
                "one sound per sequential forward birth {character:?}/{rules:?}"
            );
        }
    }
}

#[test]
fn controlled_initial_bullet_retains_attack_art_then_next_step_births_pre_before_late_cancel() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = room(rules, character, false, false);
            // Initial actual Bullet is a controlled collision actor, not a
            // claimed natural Shopkeeper producer. No writes after this setup.
            let bullet = game.spawn(
                "oBullet",
                Vec2::new(game.player.pos.x - 8.0, game.player.pos.y),
            );
            game.world.entity_mut(bullet).unwrap().vel.x = 8.0;
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                game.player.stun,
                20,
                "FIXTURE_BULLET: {}",
                diagnostic(&game)
            );
            assert!(
                attack_sprite(&game),
                "automatic Collision must retain the preceding attack sprite"
            );
            assert!(game.world.entity(bullet).is_none());
            let next_id = game.world.next_id;
            game.step(&InputFrame::default());
            assert!(!attack_sprite(&game));
            assert_eq!(game.player.attack_timer, 0);
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|entity| entity.alive && catalog::is_a(&entity.object, "oWhip"))
            );
            assert!(
                game.world.entities.iter().any(|entity| entity.id >= next_id
                    && entity.alive
                    && entity.object
                        == if character == characters::Character::TunnelMan {
                            "oMattockPre"
                        } else {
                            "oWhipPre"
                        }),
                "source unguarded old-sprite pre birth precedes late Whip-only cancellation"
            );
            game.step(&InputFrame::default());
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|entity| entity.alive && catalog::is_a(&entity.object, "oWhipPre"))
            );
        }
    }
}

// The following are controlled initial source-state fixtures, not claims that
// a natural producer reached these combinations. Every mutation precedes the
// first authoritative Game.step; no post-Step latch/sprite/HP rewrites occur.
fn initial_grounded_arrow(health: i32, wall_hurt: f64, bloodless: bool, debris: usize) -> Game {
    let mut game = room(
        RulesPreset::Classic,
        characters::Character::Explorer,
        false,
        false,
    );
    game.player.pos = Vec2::new(184.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.health = health;
    game.player.data.insert("bounced".into(), 0.0);
    game.player.data.insert("wall_hurt".into(), wall_hurt);
    game.player.data.insert("wallHurt".into(), wall_hurt);
    game.player
        .data
        .insert("bloodless".into(), if bloodless { 1.0 } else { 0.0 });
    for index in 0..debris {
        game.spawn("oBone", Vec2::new(400.0 + index as f64, 100.0));
    }
    let arrow = game.spawn("oArrow", game.player.pos);
    game.world.entity_mut(arrow).unwrap().vel.x = 8.0;
    game
}

#[test]
fn first_late_floor_contact_runs_direct_wall_then_independent_script_bottom_injury() {
    for (bloodless, debris, blood) in [(false, 0, 6), (true, 0, 3), (false, 15, 3), (false, 16, 3)]
    {
        let mut game = room(
            RulesPreset::Classic,
            characters::Character::Explorer,
            false,
            false,
        );
        game.player.pos = Vec2::new(184.0, 150.0);
        game.player.previous_pos = game.player.pos;
        game.player.vel = Vec2::new(1.0, 1.4);
        game.player.state = PlayerState::Stunned;
        game.player.stun = 20;
        game.player.health = 10;
        game.player.grounded = false;
        game.player.set_sprite("sDieLR");
        game.player.data.insert("bounced".into(), 0.0);
        game.player.data.insert("wall_hurt".into(), 2.0);
        game.player.data.insert("wallHurt".into(), 2.0);
        game.player
            .data
            .insert("bloodless".into(), if bloodless { 1.0 } else { 0.0 });
        for index in 0..debris {
            game.spawn("oBone", Vec2::new(400.0 + index as f64, 100.0));
        }
        // Initial bottom158 + source early test offset1 remains above floor160.
        // Early +0.6 makes vy2 and movement reaches the new solid contact.
        assert_eq!(game.player.bounds().y + game.player.bounds().h, 158.0);
        assert!(!game.world.solid_at(game.player.pos.x, 159.0));
        assert!(game.world.solid_at(game.player.pos.x, 160.0));
        assert!(game.player.is_stunned());
        assert_eq!(game.player.data["bounced"], 0.0);
        assert!(
            game.world
                .entities
                .iter()
                .all(|entity| entity.object != "oBlood")
        );
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.stun, 20,
            "initial controlled tumble retains its counter"
        );
        assert_eq!(game.player.health, 8, "direct wall1 + independent bottom1");
        assert_eq!(game.player.data["wall_hurt"], 0.0);
        assert_eq!(game.player.data["bounced"], 1.0);
        assert_eq!(
            game.player.sprite, "sDieLR",
            "art reads the prior false bounced latch"
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oBlood")
                .count(),
            blood,
            "direct3 bypasses bloodless/cap16; subsequent script3 uses both gates"
        );
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Damage { amount: 1 }))
                .count(),
            2
        );
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHurt"))
                .count(),
            1,
            "one wall sound; independent bottom injury is silent"
        );
    }
    // This oracle checks same-frame branch births, damage and sound, not the
    // complete frame RNG stream: newborn Blood/Detritus can run later events.
}

#[test]
fn zero_health_stunned_player_uses_prior_dead_flag_for_stationary_art() {
    for already_dead in [false, true] {
        let mut game = room(
            RulesPreset::Classic,
            characters::Character::Explorer,
            false,
            false,
        );
        game.player.pos = Vec2::new(184.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.health = 0;
        game.player.stun = 20;
        game.player.state = if already_dead {
            PlayerState::Dead
        } else {
            PlayerState::Stunned
        };
        game.state = if already_dead {
            RunState::Dead
        } else {
            RunState::Playing
        };
        game.player.vel = Vec2::default();
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.sprite,
            if already_dead { "sDieL" } else { "sStunL" }
        );
        assert_eq!(
            game.state,
            RunState::Dead,
            "DIED is after late sprite selection"
        );
    }
}

#[test]
fn fatal_own_step_hurt_consumes_all_live_parachutes_into_one_used_before_ankh_return() {
    let mut game = initial_grounded_arrow(2, 0.0, true, 0);
    // Keep the initial impact far from the authored return entrance, so the
    // used canopy's x also observes the pre-revival Player origin.
    game.player.pos = Vec2::new(320.0, 128.0);
    game.player.vel.y = 1.0;
    game.player.state = PlayerState::Falling;
    game.player.grounded = false;
    game.player.previous_pos = game.player.pos;
    let arrow = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.object == "oArrow")
        .unwrap();
    arrow.pos = game.player.pos;
    arrow.previous_pos = arrow.pos;
    game.player.equipment.insert("Ankh".into());
    let first = game.spawn("oParachute", Vec2::new(312.0, 112.0));
    let second = game.spawn("oParachute", Vec2::new(312.0, 112.0));
    assert!(game.player.vel.y > 0.0 && !game.player.grounded);
    assert_eq!(game.player.state, PlayerState::Falling);
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, 4, "actual fatal Arrow triggers Ankh");
    assert!(!game.player.equipment.contains("Ankh"));
    assert!(game.world.entity(first).is_none() && game.world.entity(second).is_none());
    let used: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.object == "oParaUsed")
        .collect();
    assert_eq!(
        used.len(),
        1,
        "one instance_exists birth then retirement of every canopy"
    );
    assert_eq!(
        used[0].pos.x, 312.0,
        "late tail precedes Ankh position replacement"
    );
    assert_ne!(game.player.pos.x, 320.0);
}
