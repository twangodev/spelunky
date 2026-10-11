//! Literal oYetiKing Step.xml42-73 death-prefix oracle. Public ordinary Pistol
//! pickup/fire drives every HP change; no actor/RNG writes occur after birth.
use spelunky_core::{
    Game, InputFrame, Rng, RngAlgorithm, RulesPreset, Vec2, catalog, level::Level,
};

fn arena(seed: u32, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level {
        music: "TEMPLE".into(),
        starting_life: 99, // ordinary authored level metadata, no mid-combat healing
        ..Default::default()
    };
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    level.set_tile(7, 7, '-').unwrap(); // ordinary source custom-level Pistol
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    // Source King attackTimer can still be positive after its thirty-tick
    // post-yell IDLE, allowing real WALK steps toward the player before attack.
    // This initial upper Brick stops its custom left bound at x208, hence
    // origin>=202, while the ordinary Pistol muzzle (y132..134) passes below.
    level.set_tile(11, 6, '1').unwrap();
    let mut game = Game::shell(seed, rules);
    game.load_custom(&level, "king-ordinary-pistol-debris", "");
    assert_eq!(
        game.world.tile(12, 7).unwrap().object,
        "oBrick",
        "custom border maps the initial stopper to [192,208) x [112,128)"
    );
    assert_eq!(
        game.world.tile(14, 9).unwrap().object,
        "oBrick",
        "floor top stays144"
    );
    // Authored initial enemy birth: the custom character palette has no King.
    // Source Create supplies all HP/state/facing/velocity/image/clock fields.
    let king = game.spawn("oYetiKing", Vec2::new(224.0, 112.0));
    assert_eq!(game.world.entity(king).unwrap().health, 30);
    assert_eq!(game.world.entity(king).unwrap().value("bloodless"), 0.0);
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| catalog::is_a(&e.object, "oIce"))
    );
    assert!(
        !game
            .world
            .tiles
            .iter()
            .any(|t| catalog::is_a(&t.object, "oIce"))
    );
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
    let held = game
        .player
        .held
        .expect("ordinary down+attack picks up the authored Pistol");
    assert_eq!(game.world.entity(held).unwrap().object, "oPistol");
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.facing, 1);
    (game, king)
}

fn shoot_until(game: &mut Game, king: u64, fatal: bool) -> usize {
    let mut previous_hp = 30;
    let mut hits = 0;
    for tick in 0..480 {
        let fire = tick % 24 == 0;
        if fire {
            assert!(
                game.player.pos.y - 2.0 >= 128.0,
                "ordinary Pistol muzzle remains below the initial upper Brick"
            );
        }
        game.step(&InputFrame {
            attack: fire,
            attack_pressed: fire,
            ..Default::default()
        });
        let actor = game
            .world
            .entity(king)
            .expect("fatal Bullet Collision precedes the following King Normal Step");
        if actor.health != previous_hp {
            assert_eq!(
                actor.health,
                previous_hp - 4,
                "every HP change is an ordinary Pistol hit"
            );
            previous_hp = actor.health;
            hits += 1;
        }
        if (fatal && actor.health <= 0) || (!fatal && hits == 1) {
            assert!(
                game.player.health > 0,
                "ordinary player controller remains live"
            );
            assert!(
                game.player.held.is_some(),
                "weapon acquired and retained through normal play"
            );
            return hits;
        }
    }
    panic!("ordinary Pistol inputs must reach the requested hit boundary");
}

// Literal source moveTo reciprocal/modulo arithmetic, independent of the Rust
// movement helper. The final ordinary Pistol Bullet sets +3..4,-2 and this open upward path is clear.
fn displacement(value: f64, time: u64) -> f64 {
    let fraction = value.abs().fract();
    let extra = fraction > 0.0 && time % (1.0 / fraction).round_ties_even() as u64 == 0;
    (value.abs().floor() + f64::from(extra)) * value.signum()
}

// Independent arithmetic over one raw draw, not production range()/unit().
// Canonical mapping follows frozen EXE-derived math; Legacy retains historical
// SplitMix mapping. These controls are not original gameplay runtime captures.
fn rand(rng: &mut Rng, width: u64) -> u32 {
    assert!((1..=u64::from(u32::MAX)).contains(&width));
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
    }
}
fn unit(rng: &mut Rng) -> f64 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => raw as f64 / 4_294_967_296.0,
        RngAlgorithm::LegacySplitMix64V1 => (raw >> 11) as f64 / 9_007_199_254_740_992.0,
    }
}

fn launch(rng: &mut Rng) -> (Vec2, f64) {
    let velocity = Vec2::new(unit(rng) * 4.0 - unit(rng) * 4.0, -1.0 - unit(rng) * 2.0);
    let gravity = (1 + rand(rng, 6)) as f64 * 0.1;
    (velocity, gravity)
}

#[test]
fn ordinary_pistol_kills_king_with_three_blood_four_bones_before_four_loot_rolls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [17, 42] {
            let (mut game, king) = arena(seed, rules);
            assert_eq!(
                shoot_until(&mut game, king, true),
                8,
                "30 HP needs eight four-damage shots"
            );
            let actor = game.world.entity(king).unwrap().clone();
            assert_eq!(actor.health, -2);
            assert!(
                [3.0, 3.5, 4.0].contains(&actor.vel.x),
                "source Pistol6..8 gives half-speed recoil"
            );
            assert_eq!(actor.vel.y, -2.0);
            assert_eq!(actor.value("death_recorded"), 0.0);
            assert!(
                !game.world.entities.iter().any(|e| e.object == "oBullet"),
                "no pending shot adds RNG at death"
            );
            let count = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
                .count();
            assert!(
                count <= 13,
                "three death Blood births must fit script cap16 in ordinary spaced combat"
            );
            let time = game.level_tick.wrapping_add(2) % 100_000_001;
            let origin = Vec2::new(
                actor.pos.x + displacement(actor.vel.x, time),
                actor.pos.y + displacement(actor.vel.y, time),
            );
            assert!(
                origin.x >= 200.0 && origin.x + 32.0 < 352.0,
                "upward final recoil stays in open active arena: origin={origin:?}, receiver={:?}",
                actor.pos
            );
            assert!(
                origin.y + 32.0 <= 144.0,
                "final recoil is clear of floor144"
            );
            let serial = game.world.creation_serial;
            let kills = game.data.get("kills").copied().unwrap_or(0.0);
            let mut expected = game.rng.clone();
            // Caller argument draws occur once, even if the Blood script were capped.
            let blood_pos = Vec2::new(
                origin.x + 14.0 + (rand(&mut expected, 5)) as f64,
                origin.y + 14.0 + (rand(&mut expected, 5)) as f64,
            );
            let mut particles = Vec::new();
            for _ in 0..3 {
                let (velocity, gravity) = launch(&mut expected);
                particles.push(("oBlood", blood_pos, velocity, gravity));
            }
            for _ in 0..4 {
                let pos = Vec2::new(
                    origin.x + 14.0 + (rand(&mut expected, 5)) as f64,
                    origin.y + 14.0 + (rand(&mut expected, 7)) as f64,
                );
                let (velocity, gravity) = launch(&mut expected);
                particles.push(("oBone", pos, velocity, gravity));
            }
            let mut loot = Vec::new();
            for _ in 0..4 {
                let object = if rand(&mut expected, 12) == 0 {
                    "oSpikeShoes"
                } else if rand(&mut expected, 2) == 0 {
                    "oSapphireBig"
                } else {
                    "oRopePile"
                };
                let velocity = Vec2::new(
                    (rand(&mut expected, 4)) as f64 - (rand(&mut expected, 4)) as f64,
                    -((1 + rand(&mut expected, 2)) as f64),
                );
                loot.push((object, velocity));
            }
            game.step(&InputFrame::default());
            assert!(game.world.entity(king).is_none());
            let mut births: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| {
                    e.creation_serial > serial
                        && matches!(
                            e.object.as_str(),
                            "oBlood" | "oBone" | "oSpikeShoes" | "oSapphireBig" | "oRopePile"
                        )
                })
                .collect();
            births.sort_by_key(|e| e.creation_serial);
            assert_eq!(
                births.len(),
                11,
                "source three Blood, four Bones, then four loot births"
            );
            for (particle, (object, pos, velocity, gravity)) in births.iter().take(7).zip(particles)
            {
                assert_eq!(particle.object, object, "source birth order");
                assert_eq!(
                    particle.pos, pos,
                    "literal randomized current receiver origin"
                );
                assert_eq!(
                    particle.vel, velocity,
                    "synchronous Create launch before next source draw"
                );
                assert_eq!(particle.value("gravity"), gravity);
                assert_eq!(
                    particle.value("life"),
                    60.0,
                    "Blood370/Bone654 already passed before King877"
                );
                assert_eq!(particle.value("world_effect_initialized"), 1.0);
                assert_eq!(particle.image_speed(), 0.3);
            }
            for (drop, (object, velocity)) in births.iter().skip(7).zip(loot) {
                assert_eq!(
                    drop.object, object,
                    "loot classification uses RNG after all debris Creates"
                );
                assert_eq!(drop.pos, Vec2::new(origin.x + 16.0, origin.y + 16.0));
                assert_eq!(drop.vel, velocity);
                if object != "oSapphireBig" {
                    assert_eq!(drop.value("cost"), 0.0);
                    assert_eq!(drop.value("for_sale"), 0.0);
                }
            }
            let mut actual = game.rng.clone();
            for _ in 0..8 {
                assert_eq!(
                    actual.next_u64(),
                    expected.next_u64(),
                    "complete prefix and loot RNG order"
                );
            }
            assert_eq!(game.data.get("kills"), Some(&(kills + 1.0)));
            assert_eq!(game.data.get("yetikings"), Some(&1.0));
            let last_serial = births.last().unwrap().creation_serial;
            game.step(&InputFrame::default());
            assert!(game.world.entity(king).is_none());
            assert_eq!(
                game.data.get("kills"),
                Some(&(kills + 1.0)),
                "Destroy/credit once"
            );
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.creation_serial > last_serial
                        && matches!(
                            e.object.as_str(),
                            "oBone" | "oSpikeShoes" | "oSapphireBig" | "oRopePile"
                        )),
                "retired King cannot repeat debris or loot on next Step"
            );
        }
    }
}

#[test]
fn ordinary_nonlethal_pistol_hit_leaves_king_alive_without_death_debris_or_loot() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [17, 42] {
            let (mut game, king) = arena(seed, rules);
            assert_eq!(shoot_until(&mut game, king, false), 1);
            assert_eq!(game.world.entity(king).unwrap().health, 26);
            game.step(&InputFrame::default());
            assert!(game.world.entity(king).is_some());
            assert!(!game.world.entities.iter().any(|e| matches!(
                e.object.as_str(),
                "oBone" | "oSpikeShoes" | "oSapphireBig" | "oRopePile"
            )));
            assert_eq!(game.data.get("yetikings").copied().unwrap_or(0.0), 0.0);
            assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        }
    }
}
