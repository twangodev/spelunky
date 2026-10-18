//! Complete Game::step fixtures start with terrain and default source actors.
//! Explosions come from a FireFrogBomb's natural 120-tick Alarm1; lasers come
//! from a default UFO's SEARCH→DESTROY→BLAST. No effect or callback is injected.
use crate::{Area, Game, GameEvent, InputFrame, Rng, RulesPreset, Vec2, World};

fn source_room(rules: RulesPreset, player: Vec2, floor: usize) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(40, 20, Area::Ice, 9);
    game.player.pos = player;
    game.player.previous_pos = player;
    for x in 0..40 {
        game.world.set_tile(x, floor, "oHardBlock");
    }
    game
}

fn attack() -> InputFrame {
    InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}

fn pickup(game: &mut Game, weapon: u64) {
    // The inherited Item Step first clears the authored outside-shop price.
    game.step(&InputFrame::default());
    game.step(&InputFrame {
        down: true,
        ..attack()
    });
    assert_eq!(game.player.held, Some(weapon));
    game.step(&InputFrame::default());
}

// Canonical GM8 raw words use mulhigh(width) for these positive integer
// rand bounds, independently of production range/chance and callback helpers.
// The separate EXE-derived vector suite checks the raw recurrence itself.
fn literal_rand(rng: &mut Rng, low: u32, high: u32) -> f64 {
    f64::from(low) + ((rng.next_u64() * u64::from(high - low + 1)) >> 32) as f64
}

fn literal_offset(rng: &mut Rng, origin: Vec2, high: u32) -> Vec2 {
    Vec2::new(
        origin.x + literal_rand(rng, 0, high) - literal_rand(rng, 0, high),
        origin.y + literal_rand(rng, 0, high) - literal_rand(rng, 0, high),
    )
}

// FireFrogBomb Alarm1 creates Explosion, then scrCreateBlood(3), then
// high-graphics scrCreateFlame(3), before any automatic collision. Both child
// Creates contain three random() calls and one rand(1,6); their Detritus parent
// draws nothing. These fixtures assert an empty shared detritus population,
// bloodless=false and high graphics, so all six births pass script cap16 and
// inherited cap32. Spell out the literal child RNG rather than skip24 blindly.
fn literal_fire_frog_alarm_payload(rng: &mut Rng) {
    fn random(rng: &mut Rng, bound: f64) -> f64 {
        (rng.next_u64() as f64 / 4_294_967_296.0) * bound
    }
    for _object in ["oBlood", "oFlame"] {
        for _ in 0..3 {
            let _x_velocity = random(rng, 4.0) - random(rng, 4.0);
            let _y_velocity = -1.0 - random(rng, 2.0);
            let _gravity = literal_rand(rng, 1, 6) * 0.1;
        }
    }
}

fn assert_fire_frog_alarm_payload(game: &Game) {
    for object in ["oBlood", "oFlame"] {
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == object)
                .count(),
            3,
            "source Alarm1 admits three Blood then three Flame births before contacts"
        );
    }
}

#[test]
fn actual_bomb_alarm_blast_runs_boulder_own_collision_then_parent_destroy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = source_room(rules, Vec2::new(40.0, 152.0), 10);
        // An authored hard-wall pocket keeps the default rolling Boulder near
        // the naturally armed FireFrogBomb throughout its complete fuse.
        for y in 6..10 {
            game.world.set_tile(8, y, "oHardBlock");
            game.world.set_tile(11, y, "oHardBlock");
        }
        let boulder = game.spawn("oBoulder", Vec2::new(160.0, 144.0));
        let lamp = game.spawn("oLampRed", Vec2::new(160.0, 160.0));
        let bomb = game.spawn("oFireFrogBomb", Vec2::new(160.0, 120.0));
        assert_eq!(game.world.entity(boulder).unwrap().value("invincible"), 1.0);
        assert_eq!(game.world.entity(bomb).unwrap().alarms[1], 120);
        assert_eq!(game.world.entity(bomb).unwrap().value("bloodless"), 0.0);
        assert!(!game.flags.contains("low_graphics"));
        let mut reached = false;
        for _ in 0..136 {
            let before = game.world.entity(boulder).unwrap().clone();
            let detritus_before = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oDetritus"))
                .count();
            let mut source = game.rng.clone();
            game.step(&InputFrame::default());
            if game.world.entity(boulder).is_some() {
                continue;
            }
            reached = true;
            assert!(game.world.entity(bomb).is_none());
            assert_eq!(
                before.vel.x, 0.0,
                "the pocket settled naturally before the fuse"
            );
            assert_eq!(before.vel.y, 0.0);
            assert_eq!(
                detritus_before, 0,
                "both source script caps admit the complete Alarm payload"
            );
            assert_fire_frog_alarm_payload(&game);
            literal_fire_frog_alarm_payload(&mut source);
            let mut expected = Vec::new();
            for _ in 0..3 {
                expected.push((
                    "oRubble",
                    "sRubbleTan",
                    literal_offset(&mut source, before.pos, 15),
                ));
                if literal_rand(&mut source, 1, 3) == 1.0 {
                    expected.push((
                        "oRock",
                        "sRock",
                        literal_offset(&mut source, before.pos, 15),
                    ));
                }
            }
            for _ in 0..6 {
                expected.push((
                    "oRubbleSmall",
                    "sRubbleTanSmall",
                    literal_offset(&mut source, before.pos, 15),
                ));
            }
            let actual: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall" | "oRock"))
                .map(|e| (e.object.as_str(), e.sprite.as_str(), e.pos))
                .collect();
            assert_eq!(
                actual, expected,
                "raw Boulder origin, interleaved Rock draws, and 3+6 rubble"
            );
            assert!(
                game.world.entity(lamp).is_none(),
                "inherited oSolid Destroy still runs"
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oLampRedItem")
                    .count(),
                1
            );
            assert_eq!(
                game.rng.clone().next_u64(),
                source.next_u64(),
                "39+4*k draws, no rubble/rock Create RNG or cap"
            );
            assert_eq!(game.player.health, 4.0);
            break;
        }
        assert!(
            reached,
            "natural Bomb Alarm must destroy the default invincible Boulder"
        );
    }
}

fn laser_room(rules: RulesPreset, dynamic: bool, protected: bool) -> (Game, Vec<u64>) {
    let mut game = source_room(rules, Vec2::new(88.0, 152.0), 10);
    let mut targets = Vec::new();
    for x in 0..13 {
        let origin = Vec2::new(x as f64 * 16.0, 160.0);
        if dynamic {
            game.world.set_tile(x, 10, "");
            targets.push(game.spawn("oArrowTrapLeft", origin));
        } else {
            game.world
                .set_tile(x, 10, if protected { "oHardBlock" } else { "oBrick" });
        }
        // Two layer3 tiles may overlap. Source tile_layer_find deletes exactly
        // one at the receiver's raw(x,y-16), even on an invincible receiver.
        for depth in [3.0, 3.0, 2.0] {
            let id = game.spawn(
                "oGenerationDecoration",
                Vec2::new(origin.x, origin.y - 16.0),
            );
            let tile = game.world.entity_mut(id).unwrap();
            tile.set_sprite("bgCaveTop3");
            // Match the authored layer3 tile crop representation used by
            // opening::bricks: backgrounds do not have sprite collision masks.
            for (key, value) in [
                ("depth", depth),
                ("source_x", 0.0),
                ("source_y", 0.0),
                ("source_w", 16.0),
                ("source_h", 16.0),
            ] {
                tile.data.insert(key.into(), value);
            }
        }
    }
    let ufo = game.spawn("oUFO", Vec2::new(80.0, 96.0));
    assert_eq!(game.world.entity(ufo).unwrap().state, 0);
    assert_eq!(game.world.entity(ufo).unwrap().vel, Vec2::default());
    for _ in 0..64 {
        game.step(&InputFrame::default());
        if game.world.entity(ufo).unwrap().state == 2 {
            return (game, targets);
        }
    }
    panic!("default UFO must naturally acquire the nearby standing player");
}

#[test]
fn actual_ufo_laser_runs_solid_own_destroy_and_unconditional_layer_three_continuation() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (dynamic, protected) in [(false, false), (true, false), (false, true)] {
            let (mut game, targets) = laser_room(rules, dynamic, protected);
            // Move away during the source charge animation so the born Laser
            // reaches the authored floor rather than consuming itself on us.
            let right = InputFrame {
                right: true,
                run: true,
                ..Default::default()
            };
            let mut reached = false;
            for _ in 0..96 {
                let mut source = game.rng.clone();
                game.step(&right);
                if !game.events.iter().any(
                    |e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSmallExplode"),
                ) {
                    continue;
                }
                reached = true;
                assert!(!game.world.entities.iter().any(|e| e.object == "oLaser"));
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oLaserExplode")
                        .count(),
                    1
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oGenerationDecoration" && e.value("depth") == 3.0)
                        .count(),
                    25
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oGenerationDecoration" && e.value("depth") == 2.0)
                        .count(),
                    13
                );
                let rubble: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                    .collect();
                if protected {
                    assert!(rubble.is_empty());
                    assert!(
                        (0..13).all(|x| game.world.tile(x, 10).unwrap().object == "oHardBlock")
                    );
                } else {
                    let origin = if dynamic {
                        let missing: Vec<_> = targets
                            .iter()
                            .enumerate()
                            .filter(|(_, id)| game.world.entity(**id).is_none())
                            .collect();
                        assert_eq!(missing.len(), 1);
                        assert_eq!(
                            game.world
                                .entities
                                .iter()
                                .filter(|e| e.object == "oArrow")
                                .count(),
                            1,
                            "unfired own Destroy releases recoverable arrow"
                        );
                        Vec2::new(missing[0].0 as f64 * 16.0, 160.0)
                    } else {
                        let missing: Vec<_> = (0..13)
                            .filter(|&x| game.world.tile(x, 10).unwrap().object.is_empty())
                            .collect();
                        assert_eq!(missing.len(), 1);
                        Vec2::new(missing[0] as f64 * 16.0, 160.0)
                    };
                    assert_eq!(rubble.len(), 3);
                    let center = Vec2::new(origin.x + 8.0, origin.y + 8.0);
                    for (index, fragment) in rubble.iter().enumerate() {
                        assert_eq!(fragment.pos, literal_offset(&mut source, center, 8));
                        let sprite = match (dynamic, index) {
                            (true, 0) => "sRubbleTan",
                            (true, _) => "sRubbleTanSmall",
                            (false, 0) => "sRubble",
                            (false, _) => "sRubbleSmall",
                        };
                        assert_eq!(fragment.sprite, sprite);
                    }
                }
                assert_eq!(
                    game.rng.clone().next_u64(),
                    source.next_u64(),
                    "own Destroy precedes impact effect; fixed Creates add no draws"
                );
                assert_eq!(game.player.health, 4.0);
                break;
            }
            assert!(reached, "natural UFO blast must reach the authored solid");
        }
    }
}

#[test]
fn actual_firearm_pickup_and_attack_barrier_hit_emits_bullet_only_fx() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oPistol", "oWebCannon"] {
            let mut game = source_room(rules, Vec2::new(124.0, 136.0), 9);
            game.world.set_tile(10, 4, "oHardBlock");
            game.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
            let weapon = game.spawn(object, game.player.pos);
            pickup(&mut game, weapon);
            game.step(&attack());
            let projectile = if object == "oPistol" {
                "oBullet"
            } else {
                "oWebBall"
            };
            let id = game
                .world
                .entities
                .iter()
                .find(|e| e.object == projectile)
                .unwrap()
                .id;
            let mut reached = false;
            for _ in 0..16 {
                let before = game.world.entity(id).unwrap().clone();
                game.step(&InputFrame::default());
                if game.world.entity(id).is_some() {
                    continue;
                }
                reached = true;
                let smoke: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oSmokePuff")
                    .collect();
                let hit: Vec<_> = game
                    .events
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Sound { name, pos } if name == "sndHit" => Some(*pos),
                        _ => None,
                    })
                    .collect();
                if object == "oPistol" {
                    let impact =
                        Vec2::new(before.pos.x + before.vel.x, before.pos.y + before.vel.y);
                    assert_eq!(smoke.len(), 1);
                    assert_eq!(smoke[0].pos, impact);
                    assert_eq!(hit, [impact]);
                } else {
                    assert!(smoke.is_empty(), "WebBall source collision stays kill-only");
                    assert!(hit.is_empty());
                }
                assert_eq!(game.player.health, 4.0);
                break;
            }
            assert!(reached);
        }
    }
}

#[test]
fn actual_bomb_alarm_emitter_destroy_removes_barrier_before_same_frame_fired_bullet() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = source_room(rules, Vec2::new(124.0, 136.0), 9);
        game.world.set_tile(10, 4, "oHardBlock");
        let emitter = game.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
        // The bottom-8 Bomb bounds rest on this separate authored shelf.
        // Placing it above the emitter support would settle too high to hit.
        game.world.set_tile(11, 6, "oHardBlock");
        let bomb = game.spawn("oFireFrogBomb", Vec2::new(184.0, 88.0));
        let weapon = game.spawn("oPistol", game.player.pos);
        pickup(&mut game, weapon);
        while game.tick < 115 {
            game.step(&InputFrame::default());
        }
        game.step(&attack()); // First gun RNG gives vx6; born bullet moves now.
        let bullet = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oBullet")
            .unwrap()
            .id;
        assert_eq!(game.world.entity(bullet).unwrap().vel.x, 6.0);
        while game.tick < 119 {
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.world.entity(bomb).unwrap().pos,
            Vec2::new(184.0, 88.0),
            "source bottom-8 bounds must remain on the separate shelf"
        );
        assert_eq!(game.world.entity(bomb).unwrap().alarms[1], 1);
        assert_eq!(
            game.world.entity(bullet).unwrap().pos,
            Vec2::new(156.0, 134.0)
        );
        let barrier = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oBarrier")
            .unwrap();
        assert_eq!(
            barrier.animation.floor(),
            2.0,
            "source Barrier frame at the next collision is phase-sensitive"
        );
        let mut next_bullet = game.world.entity(bullet).unwrap().clone();
        next_bullet.pos.x += next_bullet.vel.x;
        assert!(
            barrier.event_intersects(&next_bullet),
            "without synchronous Destroy the next bullet mask must hit"
        );
        assert_eq!(game.world.entity(bomb).unwrap().value("bloodless"), 0.0);
        assert!(!game.flags.contains("low_graphics"));
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && crate::catalog::is_a(&e.object, "oDetritus")),
            "both source script caps admit the complete Alarm payload"
        );
        let mut source = game.rng.clone();
        literal_fire_frog_alarm_payload(&mut source);
        game.step(&InputFrame::default());
        assert_eq!(game.tick, 120);
        assert_fire_frog_alarm_payload(&game);
        assert!(
            game.world.entity(bomb).is_none(),
            "natural Alarm1 creates the blast"
        );
        assert!(
            game.world.entity(emitter).is_none(),
            "first source Explosion alpha frame intersects the emitter"
        );
        assert!(
            game.world.entity(bullet).is_some(),
            "Explosion391/Emitter762 Destroy removes Barrier before Bullet669/Barrier763"
        );
        assert!(!game.world.entities.iter().any(|e| e.object == "oBarrier"));
        let sparks: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oFlareSpark")
            .collect();
        assert_eq!(sparks.len(), 6);
        for spark in sparks {
            let pos = Vec2::new(
                162.0 + literal_rand(&mut source, 0, 14),
                82.0 + literal_rand(&mut source, 0, 14),
            );
            let vy = literal_rand(&mut source, 1, 3);
            assert_eq!(spark.pos, pos);
            assert_eq!(spark.vel.y, vy);
        }
        assert_eq!(
            game.rng.clone().next_u64(),
            source.next_u64(),
            "six fixed uncapped Spark Creates preserve18draw source order"
        );
        assert_eq!(game.events.iter().filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSmallExplode")).count(), 1);
        assert_eq!(game.player.health, 4.0);
        game.step(&InputFrame::default());
        assert!(
            !game.events.iter().any(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSmallExplode")
            ),
            "finish_destroy_events must not replay synchronous Destroy"
        );
    }
}

fn literal_gold_fragment(
    rng: &mut Rng,
    origin: Vec2,
    object: &'static str,
) -> (&'static str, Vec2, Vec2) {
    let pos = literal_offset(rng, origin, 2);
    let velocity = Vec2::new(
        literal_rand(rng, 0, 3) - literal_rand(rng, 0, 3),
        literal_rand(rng, 3, 6),
    );
    (object, pos, velocity)
}

fn literal_gold_destroy(rng: &mut Rng, origin: Vec2, big: bool) -> Vec<(&'static str, Vec2, Vec2)> {
    let mut fragments = Vec::new();
    if big {
        fragments.push(literal_gold_fragment(rng, origin, "oGoldNugget"));
        let mut index = 0;
        // Independent literal source condition: a fresh inclusive rand on
        // every comparison, including the unsuccessful final comparison.
        while f64::from(index) < literal_rand(rng, 2, 3) {
            fragments.push(literal_gold_fragment(rng, origin, "oGoldChunk"));
            index += 1;
        }
    } else {
        for _ in 0..3 {
            fragments.push(literal_gold_fragment(rng, origin, "oGoldChunk"));
        }
    }
    fragments
}

#[test]
fn actual_ufo_laser_gold_own_destroy_precedes_solid_destroy_with_literal_loot_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Source-configured static Gold objects exercise both own callbacks,
        // both GoldBig loop outcomes, and invincible/failed-point-gate controls.
        for (gold, big, protected, point_gate, seed) in [
            (true, false, false, true, 0),
            (false, true, false, true, 0),
            (false, true, false, true, 3),
            (true, true, false, true, 7),
            (true, true, true, true, 7),
            (true, true, false, false, 7),
        ] {
            let mut game = source_room(rules, Vec2::new(88.0, 152.0), 10);
            game.rng = Rng::new(seed); // Initial source fixture seed, before actors.
            let mut blocks = Vec::new();
            for x in 0..13 {
                game.world
                    .set_tile(x, 10, if protected { "oHardBlock" } else { "oBrick" });
                for (enabled, object) in [(gold, "oGold"), (big, "oGoldBig")] {
                    if enabled {
                        // Gold's source sprite bbox starts at local(1,1).
                        // The -1 placement contains the receiver's raw origin;
                        // +1 still overlaps its mask but fails the point gate.
                        let offset = if point_gate { -1.0 } else { 1.0 };
                        let pos = Vec2::new(x as f64 * 16.0 + offset, 160.0 + offset);
                        let id = game.spawn(object, pos);
                        blocks.push((id, object, pos));
                    }
                }
            }
            let ufo = game.spawn("oUFO", Vec2::new(80.0, 96.0));
            assert_eq!(game.world.entity(ufo).unwrap().state, 0);
            let mut charging = false;
            for _ in 0..64 {
                game.step(&InputFrame::default());
                if game.world.entity(ufo).unwrap().state == 2 {
                    charging = true;
                    break;
                }
            }
            assert!(charging, "default UFO must naturally acquire the player");
            let right = InputFrame {
                right: true,
                run: true,
                ..Default::default()
            };
            let mut reached = false;
            for _ in 0..96 {
                let mut source = game.rng.clone();
                game.step(&right);
                if !game.events.iter().any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSmallExplode")) {
                    continue;
                }
                reached = true;
                let missing_tiles: Vec<_> = (0..13)
                    .filter(|&x| game.world.tile(x, 10).unwrap().object.is_empty())
                    .collect();
                let removed: Vec<_> = blocks
                    .iter()
                    .filter(|(id, _, _)| game.world.entity(*id).is_none())
                    .collect();
                let mut expected = Vec::new();
                if protected || !point_gate {
                    assert!(
                        removed.is_empty(),
                        "protected receiver and failed point gate keep Gold objects"
                    );
                } else {
                    assert_eq!(removed.len(), usize::from(gold) + usize::from(big));
                    // Source always completes Gold's own Destroy first, then
                    // GoldBig, before the Solid receiver's own rubble callback.
                    for object in ["oGold", "oGoldBig"] {
                        if let Some((_, _, origin)) =
                            removed.iter().copied().find(|(_, name, _)| *name == object)
                        {
                            let fragments =
                                literal_gold_destroy(&mut source, *origin, object == "oGoldBig");
                            if !gold && big {
                                assert_eq!(
                                    fragments.len(),
                                    if seed == 0 { 3 } else { 4 },
                                    "both condition-rerolled GoldBig loop outcomes are exercised"
                                );
                            }
                            expected.extend(fragments);
                        }
                    }
                }
                let actual: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| matches!(e.object.as_str(), "oGoldChunk" | "oGoldNugget"))
                    .map(|e| (e.object.as_str(), e.pos, e.vel))
                    .collect();
                assert_eq!(
                    actual, expected,
                    "literal raw Gold origin and 3..6 launch velocity; uncapped fixed Creates add no RNG"
                );
                let rubble: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                    .collect();
                if protected {
                    assert!(missing_tiles.is_empty());
                    assert!(rubble.is_empty());
                } else {
                    assert_eq!(missing_tiles.len(), 1);
                    let center = Vec2::new(missing_tiles[0] as f64 * 16.0 + 8.0, 168.0);
                    assert_eq!(rubble.len(), 3);
                    for fragment in rubble {
                        assert_eq!(
                            fragment.pos,
                            literal_offset(&mut source, center, 8),
                            "Solid Destroy draws must follow all Gold own-Destroy draws"
                        );
                    }
                }
                assert_eq!(
                    game.rng.clone().next_u64(),
                    source.next_u64(),
                    "21draw Gold;24/32draw GoldBig including failed condition; then12draw Solid"
                );
                assert_eq!(game.player.health, 4.0);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oLaserExplode")
                        .count(),
                    1
                );
                break;
            }
            assert!(reached, "natural UFO Laser must contact the authored floor");
        }
    }
}
