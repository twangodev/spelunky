//! Ordinary source Creates and public Game.step only. Terrain, player entrance,
//! and enemy births are fixed before the first Step; no midway actor, velocity,
//! sprite, RNG, or terrain edits. First-step pool fixtures are authored initial
//! contacts; the final two tests reach the same branch through default AI motion.
//! Source: oEnemy Step78-88; oTombLord Step42-56; oShopkeeper Step505-547.
use spelunky_core::{Area, Entity, Game, GameEvent, InputFrame, Rng, RulesPreset, Vec2, World};

#[derive(Clone, Copy, Debug)]
enum Pool {
    Dry,
    Feet,
    HeadAndFeet,
}

fn room(seed: u64, rules: RulesPreset, level: u32, upper_entrance: bool) -> Game {
    let mut game = Game::shell(seed, rules);
    game.world = World::new(30, 20, Area::Temple, level);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = if upper_entrance {
        Vec2::new(200.0, 88.0)
    } else {
        Vec2::new(240.0, 152.0)
    };
    game.player.previous_pos = game.player.pos;
    if upper_entrance {
        for x in 12..15 {
            game.world.set_tile(x, 6, "oBlock");
        }
    }
    assert!(
        !game.world.fluid_dirty,
        "authored initial pools are not a pending drain pass"
    );
    game
}

fn lava(game: &mut Game, pos: Vec2) {
    let id = game.spawn("oLava", pos);
    let lava = game.world.entity(id).unwrap();
    assert_eq!(lava.value("lava_initialized"), 1.0);
    assert_eq!(
        lava.value("spurt"),
        0.0,
        "literal source Create, no spurt overrides"
    );
}

fn alive<'a>(game: &'a Game, object: &str) -> Vec<&'a Entity> {
    let mut values: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .collect();
    values.sort_by_key(|e| e.creation_serial);
    values
}

// Independent source order adapter: raw PRNG outputs, not production range(),
// unit(), movement, or Create helpers. This does not claim original GM8 PRNG parity.
fn rand(rng: &mut Rng, width: u64) -> u32 {
    (rng.next_u64() % width) as u32
}
fn random(rng: &mut Rng) -> f64 {
    (rng.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
}

fn tomb_oracle(rng: &mut Rng, origin: Vec2) -> Vec<(Vec2, Vec2, f64)> {
    rand(rng, 5);
    rand(rng, 5); // evaluated Blood arguments, despite bloodless=true
    (0..4)
        .map(|_| {
            let pos = Vec2::new(
                origin.x + 14.0 + f64::from(rand(rng, 5)),
                origin.y + 12.0 + f64::from(rand(rng, 7)),
            );
            let velocity = Vec2::new(
                random(rng) * 4.0 - random(rng) * 4.0,
                -1.0 - random(rng) * 2.0,
            );
            let gravity = f64::from(1 + rand(rng, 6)) * 0.1;
            (pos, velocity, gravity)
        })
        .collect()
}

fn check_tomb_death(
    game: &Game,
    id: u64,
    level: u32,
    expected: &[(Vec2, Vec2, f64)],
    rng: &mut Rng,
    retired: bool,
) {
    assert_eq!(game.world.entity(id).is_none(), retired);
    if let Some(actor) = game.world.entity(id) {
        assert_eq!(actor.health, 0);
        assert_eq!(actor.value("counts_as_kill"), 0.0);
    }
    let bones = alive(game, "oBone");
    assert_eq!(
        bones.len(),
        4,
        "child death prefix must run after parent retirement"
    );
    for (bone, (pos, velocity, gravity)) in bones.iter().zip(expected) {
        assert_eq!(
            bone.pos, *pos,
            "Bone654 waits until after Tomb775's current resource phase"
        );
        assert_eq!(
            bone.vel, *velocity,
            "each complete Bone Create precedes the next origin draw"
        );
        assert_eq!(bone.value("gravity"), *gravity);
        assert_eq!(bone.value("life"), 60.0);
    }
    let sceptres = alive(game, "oSceptre");
    assert_eq!(
        sceptres.len(),
        usize::from(level == 13),
        "source level-thirteen guard"
    );
    if let Some(sceptre) = sceptres.first() {
        assert!(sceptre.creation_serial > bones.last().unwrap().creation_serial);
        assert_eq!(sceptre.value("cost"), 0.0);
    }
    assert_eq!(
        alive(game, "oBlood").len(),
        0,
        "bloodless suppresses drops, not argument RNG"
    );
    assert_eq!(
        alive(game, "oFly").len(),
        0,
        "ordinary IDLE/WALK entry is not ATTACK"
    );
    assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
    assert_eq!(
        game.rng.clone().next_u64(),
        rng.next_u64(),
        "literal parent/child/Create draw order"
    );
}

fn keeper_oracle(rng: &mut Rng) -> Vec<f64> {
    (0..1 + rand(rng, 4))
        .map(|_| f64::from(1 + rand(rng, 3)) - f64::from(1 + rand(rng, 3)))
        .collect()
}

struct KeeperFrame {
    retired: bool,
    next_id: u64,
    serial: u64,
}

fn check_keeper_death(
    game: &Game,
    id: u64,
    object: &str,
    origin: Vec2,
    nuggets: &[f64],
    rng: &mut Rng,
    frame: KeeperFrame,
) {
    assert_eq!(game.world.entity(id).is_none(), frame.retired);
    if let Some(actor) = game.world.entity(id) {
        assert_eq!(actor.health, 0);
        assert_eq!(actor.value("counts_as_kill"), 0.0);
    }
    let drops = alive(game, "oGoldNugget");
    assert_eq!(
        drops.len(),
        nuggets.len(),
        "source Keeper DEAD loot despite parent countsAsKill=false"
    );
    for (drop, vx) in drops.iter().zip(nuggets) {
        assert_eq!(drop.pos, Vec2::new(origin.x + 8.0, origin.y + 8.0));
        assert_eq!(
            drop.vel,
            Vec2::new(*vx, -1.0),
            "Nugget461 cannot Step after Keeper666 in this frame"
        );
        assert!(
            !drop.solid,
            "movement queries use oSolid ancestry, distinct from the native ObjectDef solid flag"
        );
    }
    let gun_births = u64::from(object == "oShopkeeper");
    assert_eq!(
        game.world.next_id,
        frame.next_id + nuggets.len() as u64 + gun_births,
        "literal Nugget births then exactly one optional Shotgun birth"
    );
    assert_eq!(
        game.world.creation_serial,
        frame.serial + nuggets.len() as u64 + gun_births
    );
    for (index, nugget) in drops.iter().enumerate() {
        assert_eq!(nugget.id, frame.next_id + index as u64);
        assert_eq!(nugget.creation_serial, frame.serial + index as u64 + 1);
    }
    // Shotgun670 Steps after Keeper666. Its birth center(88,152) moves
    // at most six pixels right; the floor limits y to156, and its top-Lava
    // probe remains in the source foot pool. The gun must be born, consume
    // both launch draws, and retire before public end-of-Step observation.
    // GoldNugget's native solid flag does not make it an oSolid descendant.
    assert_eq!(
        alive(game, "oShotgun").len(),
        0,
        "the source gun birth is witnessed by counters even after its later Lava Step"
    );
    assert_eq!(game.data.get("kills"), Some(&1.0));
    assert_eq!(game.data.get("shopkeepers"), Some(&1.0));
    assert!(game.flags.contains("murderer"));
    assert_eq!(
        game.lifetime_stats.enemy_kills[spelunky_core::statistics::EnemyKill::Shopkeeper as usize],
        1
    );
    assert_eq!(
        game.events
            .iter()
            .filter(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndCavemanDie")
            )
            .count(),
        1
    );
    assert_eq!(
        game.rng.clone().next_u64(),
        rng.next_u64(),
        "source Nugget then gun draw order"
    );
}

#[test]
fn public_step_tomb_parent_lava_continues_drops_with_foot_only_and_dry_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [7, 42] {
            for level in [12, 13] {
                for pool in [Pool::Dry, Pool::Feet, Pool::HeadAndFeet] {
                    let mut game = room(seed, rules, level, false);
                    if matches!(pool, Pool::HeadAndFeet) {
                        lava(&mut game, Vec2::new(96.0, 112.0));
                    }
                    if !matches!(pool, Pool::Dry) {
                        lava(&mut game, Vec2::new(96.0, 144.0));
                    }
                    let origin = Vec2::new(80.0, 128.0);
                    let id = game.spawn("oTombLord", origin);
                    assert_eq!(game.world.entity(id).unwrap().health, 20);
                    let mut expected_rng = game.rng.clone();
                    let expected = if matches!(pool, Pool::Dry) {
                        vec![]
                    } else {
                        tomb_oracle(&mut expected_rng, origin)
                    };
                    game.step(&InputFrame::default());
                    if matches!(pool, Pool::Dry) {
                        assert_eq!(game.world.entity(id).unwrap().health, 20);
                        assert_eq!(alive(&game, "oBone").len(), 0);
                        assert_eq!(alive(&game, "oSceptre").len(), 0);
                        assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                    } else {
                        check_tomb_death(&game, id, level, &expected, &mut expected_rng, true);
                    }
                }
            }
        }
    }
}

#[test]
fn public_step_keeper_parent_lava_continues_loot_credit_with_foot_only_and_dry_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [7, 42] {
            for object in ["oShopkeeper", "oShopkeeper2"] {
                for pool in [Pool::Dry, Pool::Feet, Pool::HeadAndFeet] {
                    let mut game = room(seed, rules, 13, false);
                    if matches!(pool, Pool::HeadAndFeet) {
                        lava(&mut game, Vec2::new(80.0, 128.0));
                    }
                    if !matches!(pool, Pool::Dry) {
                        lava(&mut game, Vec2::new(80.0, 144.0));
                    }
                    let origin = Vec2::new(80.0, 144.0);
                    let id = game.spawn(object, origin);
                    let births = (game.world.next_id, game.world.creation_serial);
                    let mut expected_rng = game.rng.clone();
                    let expected = if matches!(pool, Pool::Dry) {
                        vec![]
                    } else {
                        let nuggets = keeper_oracle(&mut expected_rng);
                        if object == "oShopkeeper" {
                            rand(&mut expected_rng, 3);
                            rand(&mut expected_rng, 3);
                        }
                        nuggets
                    };
                    game.step(&InputFrame::default());
                    if matches!(pool, Pool::Dry) {
                        assert_eq!(game.world.entity(id).unwrap().health, 20);
                        assert_eq!(alive(&game, "oGoldNugget").len(), 0);
                        assert_eq!(alive(&game, "oShotgun").len(), 0);
                        assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
                        assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
                    } else {
                        check_keeper_death(
                            &game,
                            id,
                            object,
                            origin,
                            &expected,
                            &mut expected_rng,
                            KeeperFrame {
                                retired: matches!(pool, Pool::HeadAndFeet),
                                next_id: births.0,
                                serial: births.1,
                            },
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn ordinary_tomb_walks_into_head_and_foot_lava_without_losing_sceptre() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [7, 42] {
            let mut game = room(seed, rules, 13, true);
            for y in 7..10 {
                lava(&mut game, Vec2::new(112.0, f64::from(y * 16)));
            }
            let id = game.spawn("oTombLord", Vec2::new(80.0, 128.0));
            let mut witnessed = false;
            for _ in 0..64 {
                let actor = game
                    .world
                    .entity(id)
                    .expect("ordinary approach remains alive before first Lava contact");
                let origin = actor.pos;
                assert_eq!(actor.health, 20);
                assert_eq!(origin.y, 128.0);
                let touches = (112.0..128.0).contains(&(origin.x + 16.0));
                let mut expected_rng = game.rng.clone();
                let expected = if touches {
                    tomb_oracle(&mut expected_rng, origin)
                } else {
                    vec![]
                };
                game.step(&InputFrame::default());
                if touches {
                    assert!(
                        origin.x > 80.0,
                        "default AI must actually walk into the pool"
                    );
                    check_tomb_death(&game, id, 13, &expected, &mut expected_rng, true);
                    let sceptre = alive(&game, "oSceptre")[0].id;
                    game.step(&InputFrame::default());
                    assert!(
                        game.world.entity(sceptre).is_some(),
                        "retired Lord cannot remove or recreate its unique drop next Step"
                    );
                    assert_eq!(alive(&game, "oSceptre").len(), 1);
                    witnessed = true;
                    break;
                }
                assert_eq!(alive(&game, "oBone").len(), 0);
            }
            assert!(
                witnessed,
                "bounded ordinary walk must reach the authored Lava column"
            );
        }
    }
}

#[test]
fn ordinary_unarmed_keeper_walks_into_head_and_foot_lava_without_losing_credit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [7, 42] {
            let mut game = room(seed, rules, 13, true);
            for y in 8..10 {
                lava(&mut game, Vec2::new(64.0, f64::from(y * 16)));
            }
            let id = game.spawn("oShopkeeper2", Vec2::new(80.0, 144.0));
            let mut witnessed = false;
            for _ in 0..64 {
                let actor = game
                    .world
                    .entity(id)
                    .expect("default ATTACK approach remains alive before Lava contact");
                let origin = actor.pos;
                assert_eq!(actor.health, 20);
                assert_eq!(
                    actor.value("has_gun"),
                    0.0,
                    "source Shopkeeper2 Create, not a dropped-gun injection"
                );
                assert_eq!(origin.y, 144.0);
                let touches = (64.0..80.0).contains(&(origin.x + 8.0));
                let mut expected_rng = game.rng.clone();
                let expected = if touches {
                    keeper_oracle(&mut expected_rng)
                } else {
                    vec![]
                };
                let births = (game.world.next_id, game.world.creation_serial);
                game.step(&InputFrame::default());
                if touches {
                    assert!(
                        origin.x < 80.0,
                        "source ATTACK motion must actually enter the authored column"
                    );
                    check_keeper_death(
                        &game,
                        id,
                        "oShopkeeper2",
                        origin,
                        &expected,
                        &mut expected_rng,
                        KeeperFrame {
                            retired: true,
                            next_id: births.0,
                            serial: births.1,
                        },
                    );
                    game.step(&InputFrame::default());
                    assert_eq!(
                        game.data.get("kills"),
                        Some(&1.0),
                        "Destroy/finalizer cannot credit twice"
                    );
                    assert_eq!(game.data.get("shopkeepers"), Some(&1.0));
                    witnessed = true;
                    break;
                }
                assert_eq!(alive(&game, "oGoldNugget").len(), 0);
            }
            assert!(witnessed, "bounded source ATTACK walk must reach Lava");
        }
    }
}
