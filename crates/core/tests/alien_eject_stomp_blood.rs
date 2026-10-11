//! Fixed ordinary inputs: pick up and throw a default Rock at a default UFO,
//! then approach its actual AlienEject from the side and jump onto it.
//! Initial room/seed only; no actor/inventory/RNG writes after the first Step.
use spelunky_core::*;
// Bits: Left, Right, Jump, Jump edge, Down, Up, Attack, Attack edge.
const TAPE: [u8; 57] = [
    0, 208, 0, 224, 0, 0, 0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 1, 1, 0, 0, 0, 0, 0, 2, 2, 2, 2,
    0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 0, 0, 0, 0, 12, 4, 4, 4, 6, 6, 6, 4, 6, 6, 6, 6, 6,
];
fn input(bits: u8) -> InputFrame {
    InputFrame {
        left: bits & 1 != 0,
        right: bits & 2 != 0,
        jump: bits & 4 != 0,
        jump_pressed: bits & 8 != 0,
        down: bits & 16 != 0,
        up: bits & 32 != 0,
        attack: bits & 64 != 0,
        attack_pressed: bits & 128 != 0,
        ..Default::default()
    }
}
fn room(rules: RulesPreset) -> (Game, u64, u64) {
    let mut game = Game::shell_with_algorithm(0, rules, RngAlgorithm::Gm8Lcg32V1).unwrap();
    game.world = World::new(30, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(56.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let rock = game.spawn("oRock", game.player.pos);
    let ufo = game.spawn("oUFO", Vec2::new(80.0, 104.0));
    let actor = game.world.entity(ufo).unwrap();
    assert_eq!((actor.health, actor.state), (1, 0));
    assert_eq!(actor.vel, Vec2::default());
    (game, rock, ufo)
}
fn blood(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect()
}
fn prefix(rules: RulesPreset) -> (Game, u64) {
    let (mut game, rock, ufo) = room(rules);
    let mut born = None;
    for (tick, bits) in TAPE[..56].iter().copied().enumerate() {
        let mut birth_oracle = game.rng.clone();
        let before_rock_vx = game.world.entity(rock).unwrap().vel.x;
        let expected_birth = if tick == 7 {
            assert_eq!(birth_oracle.range(3), 0, "SOURCE_UFO_ONE_IN_THREE_EJECTION");
            let facing = if birth_oracle.range(2) == 0 { -1 } else { 1 };
            if birth_oracle.range(5) == 0 {
                for _ in 0..4 {
                    birth_oracle.range(4);
                }
            }
            // Automatic Explosion/Item collision follows normal Step.
            // Its ordinary Rock branch kicks once with rand(4,6).
            let kick = f64::from(4 + birth_oracle.range(3));
            Some((facing, kick))
        } else {
            None
        };
        game.step(&input(bits));
        assert_eq!(
            game.player.health, 4,
            "ORDINARY_ROUTE_NO_DAMAGE tick={tick}"
        );
        if tick == 1 {
            assert_eq!(game.player.held, Some(rock), "REAL_ROCK_PICKUP");
        }
        if tick == 3 {
            assert_eq!(game.player.held, None, "REAL_UPWARD_ROCK_THROW");
            let rock = game.world.entity(rock).unwrap();
            assert!(!rock.held);
            assert!(rock.vel.x > 2.0 && rock.vel.y < 0.0);
        }
        if tick == 7 {
            assert!(game.world.entity(ufo).is_none(), "REAL_UFO_ITEM_DEATH");
            let eject = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oAlienEject")
                .expect("REAL_UFO_CHANCE_AND_EJECT_CREATE");
            assert_eq!(eject.pos, Vec2::new(94.0, 106.0));
            assert_eq!(eject.vel, Vec2::new(0.0, -5.5));
            assert_eq!((eject.health, eject.state), (1, 0));
            assert_eq!(eject.value("ice_context_created"), 1.0);
            assert_eq!(
                eject.facing,
                expected_birth.unwrap().0,
                "SOURCE_SYNCHRONOUS_EJECT_DIRECTION_CREATE"
            );
            let launched_rock = game.world.entity(rock).unwrap();
            assert!(launched_rock.pos.x > eject.pos.x);
            assert_eq!(
                launched_rock.vel.x,
                before_rock_vx + expected_birth.unwrap().1,
                "SOURCE_NEWBORN_EXPLOSION_ITEM_KICK_AFTER_EJECT_STEP"
            );
            assert_eq!(
                game.rng.state_bits(),
                birth_oracle.state_bits(),
                "SOURCE_EXPLOSION_CREATE_NO_RNG_CHANCE_EJECT_CREATE_BURN_THEN_ITEM_COLLISION_TAPE"
            );
            born = Some(eject.id);
        }
        assert!(blood(&game).is_empty(), "NO_PREEXISTING_BLOOD tick={tick}");
        assert_eq!(game.data.get("aliens"), None);
    }
    let id = born.unwrap();
    let eject = game.world.entity(id).unwrap();
    assert_eq!(eject.pos, Vec2::new(130.0, 145.0));
    assert_eq!(eject.vel, Vec2::new(1.0, 2.0));
    assert_eq!(eject.state, 2);
    assert_eq!(game.player.pos, Vec2::new(116.0, 132.0));
    assert_eq!(game.player.vel, Vec2::new(3.0, 2.5));
    assert_eq!(game.player.state, PlayerState::Falling);
    assert!(game.world.entities.iter().all(|e| {
        !e.alive
            || !matches!(
                e.object.as_str(),
                "oUFO" | "oExplosion" | "oAlien" | "oBlood"
            )
    }));
    assert_eq!(game.data.get("ufos"), Some(&1.0));
    assert_eq!(game.data.get("kills"), Some(&1.0));
    (game, id)
}
#[test]
fn ordinary_ufo_rock_eject_route_survives_until_real_stomp() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        prefix(rules);
    }
}
#[test]
fn ordinary_ufo_eject_stomp_creates_three_player_offset_blood_and_twelve_rng_draws() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = prefix(rules);
        // Eject FLOAT has no RNG; Burn Step and the remaining Rock have no
        // source random producers here. Clone only reads the pre-collision RNG.
        let mut oracle = game.rng.clone();
        let expected: Vec<_> = (0..3)
            .map(|_| {
                let velocity = Vec2::new(
                    oracle.unit() * 4.0 - oracle.unit() * 4.0,
                    -1.0 - oracle.unit() * 2.0,
                );
                let gravity = f64::from(1 + oracle.range(6)) * 0.1;
                (velocity, gravity)
            })
            .collect();
        game.step(&input(TAPE[56]));
        assert!(game.world.entity(id).is_none(), "REAL_EJECT_STOMP_DESTROY");
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.pos, Vec2::new(119.0, 136.0));
        assert_eq!(game.player.vel, Vec2::new(3.0, -6.7));
        assert_eq!(game.player.data.get("fall_timer"), Some(&0.0));
        assert_eq!(game.data.get("aliens"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&2.0));
        assert_eq!(game.lifetime_stats.enemy_kills[15], 1);
        assert!(
            matches!(game.events.as_slice(), [GameEvent::Sound { name, pos }]
            if name=="sndHit" && *pos==Vec2::new(131.0,147.0))
        );
        let particles = blood(&game);
        assert_eq!(particles.len(), 3, "SOURCE_EJECT_THREE_DIRECT_BLOOD");
        for (particle, (velocity, gravity)) in particles.iter().zip(expected) {
            assert_eq!(
                particle.pos,
                Vec2::new(127.0, 144.0),
                "SOURCE_PLAYER_PLUS_EIGHT"
            );
            assert_eq!(particle.vel, velocity);
            assert_eq!(particle.value("gravity"), gravity);
            assert_eq!(&particle.alarms[..3], &[1, 1, 5]);
            assert_eq!(
                particle.age, 0,
                "COLLISION_BIRTH_HAS_NO_EARLIER_NORMAL_STEP"
            );
            assert_eq!(particle.image_speed(), 0.3);
            assert_eq!(particle.value("bounce"), 0.0);
            assert_eq!(particle.value("invincible"), 1.0);
        }
        assert_eq!(
            game.rng.state_bits(),
            oracle.state_bits(),
            "SOURCE_DIRECT_THREE_CREATE_TWELVE_DRAWS"
        );
        let blood_ids: Vec<_> = particles.iter().map(|e| e.id).collect();
        assert!(blood_ids.windows(2).all(|ids| ids[0] < ids[1]));
        game.step(&InputFrame::default());
        assert_eq!(
            blood(&game).iter().map(|e| e.id).collect::<Vec<_>>(),
            blood_ids,
            "NO_FOURTH_BLOOD_AFTER_RETIRED_EJECT"
        );
        assert_eq!(game.data.get("kills"), Some(&2.0));
        assert_eq!(game.data.get("aliens"), Some(&1.0));
        assert_eq!(game.lifetime_stats.enemy_kills[15], 1);
        assert_eq!(game.player.health, 4);
    }
}
#[test]
fn same_real_ufo_birth_without_jump_avoids_contact_and_emits_no_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, rock, ufo) = room(rules);
        let mut born = None;
        for (tick, bits) in TAPE.iter().copied().enumerate() {
            game.step(&input(bits & !(4 | 8)));
            if tick == 1 {
                assert_eq!(game.player.held, Some(rock));
            }
            if tick == 7 {
                assert!(game.world.entity(ufo).is_none());
                born = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oAlienEject")
                    .map(|e| e.id);
            }
        }
        assert!(
            game.world.entity(born.unwrap()).is_some(),
            "NO_STOMP_EJECT_SURVIVES"
        );
        assert_eq!(game.player.health, 4, "SOURCE_NO_CONTACT_WITHOUT_JUMP");
        assert!(
            matches!(
                game.player.state,
                PlayerState::Standing | PlayerState::Running
            ),
            "SOURCE_STOMP_REQUIRES_JUMPING_OR_FALLING"
        );
        assert!(game.player.grounded);
        assert_eq!(game.data.get("aliens"), None);
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert!(blood(&game).is_empty(), "SOURCE_NO_CONTACT_HAS_NO_BLOOD");
    }
}

// Actual grounded contact control for the source AlienEject else branch.
// Initial default actors only; all later writes are ordinary input frames.
#[test]
fn real_ufo_eject_grounded_contact_hurts_once_without_stomp_blood_or_credit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell_with_algorithm(0, rules, RngAlgorithm::Gm8Lcg32V1).unwrap();
        game.world = World::new(30, 20, Area::Ice, 9);
        game.world.name = "rLevel".into();
        for x in 0..30 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(56.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let rock = game.spawn("oRock", game.player.pos);
        let ufo = game.spawn("oUFO", Vec2::new(80.0, 104.0));
        let mut born = None;
        let mut contact = false;
        for tick in 0..64 {
            let mut input = InputFrame::default();
            if tick == 1 {
                input.down = true;
                input.attack = true;
                input.attack_pressed = true;
            }
            if tick == 3 {
                input.up = true;
                input.attack = true;
                input.attack_pressed = true;
            }
            if tick > 3 {
                if let Some(e) = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oAlienEject")
                {
                    let dx = e.pos.x - game.player.pos.x;
                    input.left = dx < -2.0;
                    input.right = dx > 2.0;
                }
            }
            assert!(!input.jump && !input.jump_pressed);
            let rng_before = game.rng.state_bits();
            game.step(&input);
            if tick == 1 {
                assert_eq!(game.player.held, Some(rock));
            }
            if tick == 3 {
                assert_eq!(game.player.held, None);
            }
            if tick == 7 {
                assert!(
                    game.world.entity(ufo).is_none(),
                    "REAL_UFO_THROWN_ROCK_DEATH"
                );
                let e = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oAlienEject")
                    .expect("REAL_UFO_EJECT_BIRTH");
                assert_eq!(e.pos, Vec2::new(94.0, 106.0));
                assert_eq!(e.state, 0);
                born = Some(e.id);
            }
            assert!(
                game.world
                    .entities
                    .iter()
                    .all(|e| !e.alive || e.object != "oBlood"),
                "SOURCE_GROUNDED_CONTACT_HAS_NO_BLOOD tick={tick}"
            );
            assert_eq!(game.data.get("aliens"), None);
            if game.player.health < 4 {
                let e = game
                    .world
                    .entity(born.unwrap())
                    .expect("SOURCE_HURT_BRANCH_RETAINS_EJECT");
                assert_eq!(game.player.health, 3, "SOURCE_CONTACT_MINUS_ONE");
                assert_eq!(game.player.invincible, 30);
                assert_eq!(game.player.stun, 0);
                assert!(!game.player.is_stunned());
                assert!(
                    matches!(
                        game.player.state,
                        PlayerState::Standing | PlayerState::Running
                    ),
                    "SOURCE_STOMP_STATE_GATE_FALSE"
                );
                assert_eq!(e.state, 2, "REAL_FLOAT_CONTACT");
                assert_eq!(e.health, 1);
                assert_eq!(game.data.get("kills"), Some(&1.0));
                assert_eq!(game.data.get("ufos"), Some(&1.0));
                assert_eq!(game.lifetime_stats.enemy_kills[15], 0);
                assert_eq!(
                    game.rng.state_bits(),
                    rng_before,
                    "SOURCE_EJECT_FLOAT_HURT_HAS_NO_RNG"
                );
                assert!(
                    matches!(game.events.as_slice(),[GameEvent::Damage{amount:1},GameEvent::Sound{name,..}] if name=="sndHurt"),
                    "SOURCE_ACTUAL_CONTACT_DAMAGE_EVENT"
                );
                eprintln!(
                    "GROUNDED_CONTACT {rules:?} tick={tick} player={:?} state={:?} eject={:?} masks={:?}/{:?} hp={} inv={} stun={}",
                    game.player.pos,
                    game.player.state,
                    e.pos,
                    game.player.sprite_bounds(),
                    e.sprite_bounds(),
                    game.player.health,
                    game.player.invincible,
                    game.player.stun
                );
                contact = true;
                break;
            }
        }
        assert!(contact, "BOUNDED_REAL_GROUNDED_CONTACT_NOT_REACHED");
    }
}
