//! Source oracle for oTombLord Step.xml:42-56 and oBone inherited/own Create.
use spelunky_core::{Game, InputFrame, Rng, RulesPreset, Vec2, level::Level};

// scrLoadLevel places authored cells at (16 + column*16, 16 + row*16).
// These guards use world coordinates, including that surrounding border.
const CUSTOM_LEVEL_BORDER: f64 = 16.0;
const WORLD_FLOOR_TOP: f64 = CUSTOM_LEVEL_BORDER + 8.0 * 16.0;
const WORLD_COVER_RIGHT: f64 = CUSTOM_LEVEL_BORDER + 12.0 * 16.0;

fn authored_pistol_and_tomb(level_number: u32, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level {
        music: "TEMPLE".into(),
        starting_life: 99, // supported authored Level metadata, not mid-combat healing
        ..Default::default()
    };
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    level.set_tile(7, 7, '-').unwrap(); // source custom-level Pistol
    level.set_tile(13, 6, 'T').unwrap(); // source custom-level Tomb Lord
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    // Authored low cover intercepts the Lord's high flies while a standing
    // Pistol muzzle at y=134 remains below it. No actor is frozen or retuned.
    for x in 0..=11 {
        level.set_tile(x, 6, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "tomblord-ordinary-pistol-death", "");
    // Initial level context only: custom format has no currLevel field. Test
    // both sides of the source level-thirteen guard without altering actors.
    game.world.level = level_number;
    let lord = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oTombLord")
        .unwrap()
        .id;
    let actor = game.world.entity(lord).unwrap();
    assert_eq!(
        actor.pos,
        Vec2::new(
            CUSTOM_LEVEL_BORDER + 13.0 * 16.0,
            CUSTOM_LEVEL_BORDER + 6.0 * 16.0
        ),
        "source custom-loader border applies to the authored Tomb Lord cell"
    );
    assert_eq!(game.world.tile(14, 9).unwrap().object, "oBrick");
    assert_eq!(game.world.tile(12, 7).unwrap().object, "oBrick");
    assert!(game.world.tile(13, 7).unwrap().object.is_empty());
    assert_eq!(actor.health, 20);
    assert_eq!(actor.value("bloodless"), 1.0);
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    for _ in 0..4 {
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
    }
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let pistol = game
        .player
        .held
        .expect("ordinary down+attack picks up authored Pistol");
    assert_eq!(game.world.entity(pistol).unwrap().object, "oPistol");
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.facing, 1);
    (game, lord)
}

// Literal source moveTo reciprocal/modulo arithmetic. The final Bullet gives
// xVel=Bullet.xVel/2 and yVel=-2; these positive-X/integer-Y probes are clear.
fn source_displacement(v: f64, time: u64) -> f64 {
    let magnitude = v.abs();
    let fraction = magnitude.fract();
    let extra = fraction > 0.0 && time % (1.0 / fraction).round_ties_even() as u64 == 0;
    (magnitude.floor() + f64::from(extra)) * v.signum()
}

// This oracle does not call production range(), unit(), spawn(), or Create.
// It transcribes source rand=floor(random(n)) and the current pinned RNG's
// raw32 / 2^32 adapter from the frozen original executable proof. It checks
// source draw ordering; this is not an original gameplay runtime capture.
fn literal_unit(rng: &mut Rng) -> f64 {
    rng.next_u64() as f64 / 4_294_967_296.0
}

#[test]
fn ordinary_pistol_shots_kill_authored_tomb_with_interleaved_bone_creates_and_last_sceptre() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for level_number in [12, 13] {
            let (mut game, lord) = authored_pistol_and_tomb(level_number, rules);
            let mut hits = 0;
            let mut fired = false;
            let mut previous_hp = 20;
            let mut ready_for_death_step = false;
            for tick in 0..360 {
                let shoot = tick % 24 == 0;
                game.step(&InputFrame {
                    attack: shoot,
                    attack_pressed: shoot,
                    ..Default::default()
                });
                fired |= game.world.entities.iter().any(|e| e.object == "oBullet");
                let actor = game.world.entity(lord).expect(
                    "observe fatal Bullet collision before following Normal Step retires Lord",
                );
                if actor.health != previous_hp {
                    assert_eq!(actor.health, previous_hp - 4, "ordinary Bullet damage only");
                    previous_hp = actor.health;
                    hits += 1;
                }
                if actor.health <= 0 {
                    ready_for_death_step = true;
                    break;
                }
            }
            assert!(
                fired && ready_for_death_step,
                "authored combat must reach fatal ordinary shot"
            );
            assert!(
                game.player.health > 0,
                "ordinary input controller remains live"
            );
            assert_eq!(
                hits, 5,
                "twenty source HP requires five four-damage Bullet hits"
            );
            let actor = game.world.entity(lord).unwrap().clone();
            assert_eq!(actor.value("death_recorded"), 0.0);
            assert_eq!(actor.value("bloodless"), 1.0);
            assert_eq!(actor.vel.y, -2.0);
            assert!(actor.vel.x > 0.0);
            // Dispatcher increments level_tick before the actor reads engine_time.
            let time = game.level_tick.wrapping_add(2) % 100_000_001;
            let origin = Vec2::new(
                actor.pos.x + source_displacement(actor.vel.x, time),
                actor.pos.y + source_displacement(actor.vel.y, time),
            );
            assert!(
                origin.x > WORLD_COVER_RIGHT,
                "final movement is clear of the authored fly cover"
            );
            assert!(
                origin.y + 32.0 <= WORLD_FLOOR_TOP,
                "final upward Bullet recoil stays above floor"
            );
            let first_serial = game.world.creation_serial;
            let kills = game.data.get("kills").copied().unwrap_or(0.0);
            let mut expected_rng = game.rng.clone();
            expected_rng.next_u64(); // Blood caller rand(0,4), x (helper then returns)
            expected_rng.next_u64(); // Blood caller rand(0,4), y
            let mut expected = Vec::new();
            for _ in 0..4 {
                let pos = Vec2::new(
                    origin.x + 14.0 + ((expected_rng.next_u64() * 5) >> 32) as f64,
                    origin.y + 12.0 + ((expected_rng.next_u64() * 7) >> 32) as f64,
                );
                // oDrawnSprite/oDetritus defaults then oBone's four RNG draws,
                // completed before the following instance_create origin.
                let vel = Vec2::new(
                    literal_unit(&mut expected_rng) * 4.0 - literal_unit(&mut expected_rng) * 4.0,
                    -1.0 - literal_unit(&mut expected_rng) * 2.0,
                );
                let gravity = (1 + ((expected_rng.next_u64() * 6) >> 32)) as f64 * 0.1;
                expected.push((pos, vel, gravity));
            }
            let expected_fly = if actor.state == 3 && (7.0..=12.0).contains(&actor.animation) {
                let pos = Vec2::new(
                    origin.x + if actor.facing < 0 { 8.0 } else { 24.0 },
                    origin.y + 12.0 + ((expected_rng.next_u64() * 5) >> 32) as f64,
                );
                let vy = 0.5 - 3.0 * literal_unit(&mut expected_rng); // own Fly Create
                let vx =
                    (3 + ((expected_rng.next_u64() * 3) >> 32)) as f64 * f64::from(actor.facing);
                Some((pos, Vec2::new(vx, vy)))
            } else {
                None
            };
            game.step(&InputFrame::default());
            assert!(game.world.entity(lord).is_none());
            assert!(
                !game.world.entities.iter().any(|e| e.object == "oBlood"),
                "bloodless suppresses helper births, not its caller-origin rolls"
            );
            let mut births: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| {
                    e.creation_serial > first_serial
                        && matches!(e.object.as_str(), "oBone" | "oSceptre")
                })
                .collect();
            births.sort_by_key(|e| e.creation_serial);
            assert_eq!(births.len(), if level_number == 13 { 5 } else { 4 });
            for (bone, (pos, vel, gravity)) in births.iter().take(4).zip(expected) {
                assert_eq!(bone.object, "oBone");
                assert_eq!(bone.pos, pos, "source randomized origin, in birth order");
                assert_eq!(
                    bone.vel, vel,
                    "own RNG Create finished before next origin rolls"
                );
                assert_eq!(bone.value("gravity"), gravity);
                assert_eq!(bone.value("world_effect_initialized"), 1.0);
                assert_eq!(
                    bone.value("life"),
                    60.0,
                    "Bone resource already visited before Lord birth"
                );
                assert_eq!(bone.image_speed(), 0.3);
                assert_eq!(bone.value("collision_left"), -4.0);
                assert_eq!(bone.value("collision_bottom"), 4.0);
            }
            if level_number == 13 {
                let sceptre = births[4];
                assert_eq!(
                    sceptre.object, "oSceptre",
                    "drop follows all four Bone Creates"
                );
                assert_eq!(sceptre.pos, Vec2::new(origin.x + 16.0, origin.y + 16.0));
            }
            let born_ids: Vec<_> = births.iter().map(|e| e.id).collect();
            let last_emission_serial = births.last().unwrap().creation_serial;
            let tail_flies: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oFly" && e.creation_serial > first_serial)
                .collect();
            if let Some((pos, vel)) = expected_fly {
                assert_eq!(tail_flies.len(), 1);
                let fly = tail_flies[0];
                assert!(fly.creation_serial > last_emission_serial);
                assert_eq!(
                    fly.previous_pos, pos,
                    "source tail birth after Bones/Sceptre"
                );
                assert_eq!(fly.vel, vel);
                assert_eq!(
                    fly.pos,
                    Vec2::new(pos.x + vel.x, pos.y + vel.y),
                    "newborn Fly resource776 follows Lord775 in the same Normal Step"
                );
            } else {
                assert!(tail_flies.is_empty());
            }
            let mut actual_rng = game.rng.clone();
            for _ in 0..8 {
                assert_eq!(
                    actual_rng.next_u64(),
                    expected_rng.next_u64(),
                    "death prefix plus optional same-Step post-destroy ATTACK Fly"
                );
            }
            assert_eq!(game.data.get("kills"), Some(&(kills + 1.0)));
            assert_eq!(game.data.get("tomblords"), Some(&1.0));
            game.step(&InputFrame::default());
            assert!(game.world.entity(lord).is_none());
            assert_eq!(
                game.data.get("kills"),
                Some(&(kills + 1.0)),
                "death credited once"
            );
            assert!(born_ids.iter().all(|id| game.world.entity(*id).is_some()));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oSceptre")
                    .count(),
                usize::from(level_number == 13),
                "no duplicate drop on following Step"
            );
        }
    }
}

#[test]
fn lethal_ordinary_pistol_during_attack_still_emits_source_post_destroy_fly() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, lord) = authored_pistol_and_tomb(13, rules);
        let mut fired_final = false;
        let mut fatal_attack = None;
        let mut hits = 0;
        let mut previous_hp = 20;
        for tick in 0..720 {
            let target = game
                .world
                .entity(lord)
                .expect("Lord remains until final shot");
            // Wait one attack Step so the shot reaches the source's
            // 7..12 death-tail emission window in this authored room.
            let final_window = target.health == 4
                && target.state == 3
                && (0.75..=1.0).contains(&target.animation)
                && !fired_final;
            let shoot = if target.health > 4 {
                tick % 24 == 0
            } else {
                final_window
            };
            fired_final |= final_window;
            game.step(&InputFrame {
                attack: shoot,
                attack_pressed: shoot,
                ..Default::default()
            });
            let target = game
                .world
                .entity(lord)
                .expect("fatal collision precedes next death Step");
            if target.health != previous_hp {
                assert_eq!(target.health, previous_hp - 4);
                previous_hp = target.health;
                hits += 1;
            }
            if target.health <= 0 {
                fatal_attack = Some(target.clone());
                break;
            }
        }
        let target = fatal_attack.expect("ordinary final input shot reaches fatal ATTACK");
        assert!(fired_final && game.player.health > 0);
        assert_eq!(hits, 5);
        assert_eq!(target.state, 3, "Bullet must preserve real ATTACK status");
        assert!(
            (7.0..=12.0).contains(&target.animation),
            "input-driven projectile arrival must hit source tail-emission image window"
        );
        assert!(!target.held);
        assert_eq!(target.value("bombID"), 0.0); // ordinary authored unlinked receiver
        // Next Normal callback observes the incremented room clock.
        let time = game.level_tick.wrapping_add(2) % 100_000_001;
        let origin = Vec2::new(
            target.pos.x + source_displacement(target.vel.x, time),
            target.pos.y + source_displacement(target.vel.y, time),
        );
        assert!(origin.x > WORLD_COVER_RIGHT && origin.y + 32.0 <= WORLD_FLOOR_TOP);
        let first_serial = game.world.creation_serial;
        let mut expected_rng = game.rng.clone();
        // Corrected death prefix: two discarded Blood origins, then four
        // groups of two Bone origins followed by four own-Create draws.
        for _ in 0..26 {
            expected_rng.next_u64();
        }
        let pos = Vec2::new(
            origin.x + if target.facing < 0 { 8.0 } else { 24.0 },
            origin.y + 12.0 + ((expected_rng.next_u64() * 5) >> 32) as f64,
        );
        let vy = 0.5 - 3.0 * literal_unit(&mut expected_rng);
        let vx = (3 + ((expected_rng.next_u64() * 3) >> 32)) as f64 * f64::from(target.facing);
        let vel = Vec2::new(vx, vy);
        game.step(&InputFrame::default());
        assert!(game.world.entity(lord).is_none());
        let mut births: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| {
                e.creation_serial > first_serial
                    && matches!(e.object.as_str(), "oBone" | "oSceptre" | "oFly")
            })
            .collect();
        births.sort_by_key(|e| e.creation_serial);
        assert_eq!(
            births.iter().map(|e| e.object.as_str()).collect::<Vec<_>>(),
            vec!["oBone", "oBone", "oBone", "oBone", "oSceptre", "oFly"]
        );
        let fly = births[5];
        assert_eq!(fly.previous_pos, pos);
        assert_eq!(
            fly.vel, vel,
            "caller y, synchronous own Create, caller x RNG order"
        );
        assert_eq!(fly.pos, Vec2::new(pos.x + vel.x, pos.y + vel.y));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.data.get("tomblords"), Some(&1.0));
        let mut actual_rng = game.rng.clone();
        for _ in 0..8 {
            assert_eq!(
                actual_rng.next_u64(),
                expected_rng.next_u64(),
                "26 death-prefix draws plus three post-destroy ATTACK Fly draws"
            );
        }
        let fly_id = fly.id;
        game.step(&InputFrame::default());
        assert!(game.world.entity(lord).is_none());
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert!(
            game.world.entity(fly_id).is_some(),
            "newborn continues as ordinary Fly"
        );
    }
}
