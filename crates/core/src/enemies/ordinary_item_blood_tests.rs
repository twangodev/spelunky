//! Source oItem Step hit Blood, separate from each enemy's later death burst.
//! These authored rooms test the full scheduler; only the last test bypasses
//! Game.step to isolate the script's four Create draws from unrelated AI RNG.
use super::*;

fn room() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 12, "oBrick");
    }
    game.player.pos = Vec2::new(24.0, 184.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.facing = 1;
    game
}

fn blood(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect()
}

fn prepare_target(game: &mut Game, object: &str, pos: Vec2) -> u64 {
    let id = game.spawn(object, pos);
    let e = game.world.entity_mut(id).unwrap();
    // Source-valid quiet live states keep this hit separate from attack/death
    // effects. Every actor still executes its real Step and automatic events.
    e.vel = Vec2::default();
    e.state = if object == "oGiantSpider" { 2 } else { 0 };
    e.timer = 1000;
    e.data.insert("sight_timer".into(), 1000.0);
    e.data.insert("squirt_timer".into(), 1000.0);
    e.data.insert("psychic_recover".into(), 1000.0);
    e.data.insert("hit_cooldown".into(), 0.0);
    id
}

fn aimed_item(game: &mut Game, object: &str) -> u64 {
    let id = game.spawn(object, Vec2::new(83.0, 88.0));
    let item = game.world.entity_mut(id).unwrap();
    item.vel = Vec2::new(4.0, 0.0);
    item.data.insert("gravity".into(), 0.0);
    id
}

fn assert_new_blood(e: &Entity, pos: Vec2) {
    assert_eq!(e.pos, pos);
    assert_eq!(e.sprite, "sBlood");
    assert_eq!(e.value("world_effect_initialized"), 1.0);
    assert_eq!(e.alarms, [1, 1, 5, -1, -1, -1, -1, -1, -1, -1, -1, -1]);
    assert_eq!(e.value("collectible"), 0.0);
    assert_eq!(e.image_speed(), 0.3);
    assert!((-4.0..4.0).contains(&e.vel.x));
    assert!((-3.0..=-1.0).contains(&e.vel.y));
}

#[test]
fn actual_pickup_and_throw_emits_caveman_hit_blood_before_any_death() {
    let mut game = room();
    game.player.pos = Vec2::new(64.0, 184.0);
    game.player.previous_pos = game.player.pos;
    let target = prepare_target(&mut game, "oCaveman", Vec2::new(120.0, 176.0));
    let rock = game.spawn("oRock", Vec2::new(64.0, 188.0));
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(rock),
        "real down+attack must select the rock"
    );
    game.step(&InputFrame::default());
    assert_eq!(game.player.held, Some(rock));
    assert!(blood(&game).is_empty());
    game.step(&InputFrame {
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held, None,
        "real attack must throw the held rock"
    );
    assert!(!game.world.entity(rock).unwrap().held);

    for _ in 0..16 {
        let origin = game.world.entity(target).unwrap().pos;
        game.step(&InputFrame::default());
        let victim = game
            .world
            .entity(target)
            .expect("a single hit must leave the Caveman alive");
        if victim.health == 2 {
            assert_eq!(victim.state, 98);
            assert_eq!(victim.timer, victim.value("stun_time") as u32);
            assert_eq!(victim.vel.y, -6.0);
            let droplets = blood(&game);
            assert_eq!(
                droplets.len(),
                1,
                "oItem Step291 emits immediately on the surviving hit"
            );
            assert_new_blood(droplets[0], Vec2::new(origin.x + 8.0, origin.y + 8.0));
            assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
            return;
        }
        assert_eq!(
            victim.health, 3,
            "fixture must produce exactly one first impact"
        );
        assert!(blood(&game).is_empty(), "no hit/death Blood before impact");
    }
    panic!("the naturally thrown rock must reach the nearby idle Caveman");
}

#[test]
fn actual_step_ordinary_enemy_script_uses_target_bloodless_not_item() {
    for item_bloodless in [false, true] {
        for target_bloodless in [false, true] {
            let mut game = room();
            let target = prepare_target(&mut game, "oCaveman", Vec2::new(80.0, 80.0));
            game.world
                .entity_mut(target)
                .unwrap()
                .data
                .insert("bloodless".into(), f64::from(target_bloodless));
            let rock = aimed_item(&mut game, "oRock");
            game.world
                .entity_mut(rock)
                .unwrap()
                .data
                .insert("bloodless".into(), f64::from(item_bloodless));
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(target).unwrap().health, 2);
            assert_eq!(
                blood(&game).len(),
                usize::from(!target_bloodless),
                "item={item_bloodless} target={target_bloodless}"
            );
            if !target_bloodless {
                assert_new_blood(blood(&game)[0], Vec2::new(88.0, 88.0));
            }
        }
    }
}

#[test]
fn actual_step_damsel_script_uses_moving_item_bloodless_not_target() {
    // Source Step387 is outside with obj; the same target's own bloodless has
    // no bearing on this script call. Damsel later runs her normal item Step.
    for item_bloodless in [false, true] {
        for target_bloodless in [false, true] {
            let mut game = room();
            let target = prepare_target(&mut game, "oDamsel", Vec2::new(88.0, 88.0));
            let initial_hp = game.world.entity(target).unwrap().health;
            game.world
                .entity_mut(target)
                .unwrap()
                .data
                .insert("bloodless".into(), f64::from(target_bloodless));
            let rock = aimed_item(&mut game, "oRock");
            game.world
                .entity_mut(rock)
                .unwrap()
                .data
                .insert("bloodless".into(), f64::from(item_bloodless));
            game.step(&InputFrame::default());
            let victim = game.world.entity(target).unwrap();
            assert_eq!(victim.health, initial_hp - 1);
            assert_eq!(victim.state, 2);
            assert_eq!(
                blood(&game).len(),
                usize::from(!item_bloodless),
                "item={item_bloodless} target={target_bloodless}"
            );
            if !item_bloodless {
                assert_new_blood(blood(&game)[0], Vec2::new(88.0, 88.0));
            }
        }
    }
}

#[test]
fn actual_step_arrow_and_fishbone_finish_independent_damsel_query_after_destroy() {
    for object in ["oArrow", "oFishBone"] {
        let mut game = room();
        let enemy = prepare_target(&mut game, "oCaveman", Vec2::new(80.0, 80.0));
        let damsel = prepare_target(&mut game, "oDamsel", Vec2::new(88.0, 88.0));
        let item = aimed_item(&mut game, object);
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(item).is_none(),
            "{object} self-destructs during source item Step"
        );
        assert_eq!(
            game.world.entity(enemy).unwrap().health,
            2,
            "{object}: enemy first"
        );
        assert_eq!(
            game.world.entity(damsel).unwrap().health,
            3,
            "{object}: Damsel query still follows"
        );
        let drops = blood(&game);
        assert_eq!(
            drops.len(),
            2,
            "{object}: both source script calls must execute"
        );
        for drop in drops {
            assert_new_blood(drop, Vec2::new(88.0, 88.0));
        }
    }
}

#[test]
fn actual_step_ordinary_hit_respects_script_detritus_cap_15_and_16() {
    for count in [15, 16] {
        let mut game = room();
        let target = prepare_target(&mut game, "oCaveman", Vec2::new(80.0, 80.0));
        for _ in 0..count {
            game.spawn("oBone", Vec2::new(280.0, 48.0));
        }
        aimed_item(&mut game, "oRock");
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(target).unwrap().health,
            2,
            "cap does not suppress damage"
        );
        let droplets = blood(&game);
        assert_eq!(
            droplets.len(),
            usize::from(count < 16),
            "all oDetritus descendants count, including Bone: {count}"
        );
        if count < 16 {
            assert_new_blood(droplets[0], Vec2::new(88.0, 88.0));
        }
    }
}

#[test]
fn actual_step_ordinary_boss_hit_blood_has_literal_offsets_and_gates() {
    for (object, offset) in [
        ("oGiantSpider", Vec2::new(16.0, 24.0)),
        ("oTombLord", Vec2::new(16.0, 16.0)),
        ("oAlienBoss", Vec2::new(8.0, 8.0)),
    ] {
        for blocked in [false, true] {
            let mut game = room();
            let origin = Vec2::new(80.0, 80.0);
            let target = prepare_target(&mut game, object, origin);
            let e = game.world.entity_mut(target).unwrap();
            // Controlled surviving GiantSpider: its ordinary source Create
            // hp1 would enter its later death burst in this same Game.step,
            // mixing death Blood with the immediate impact being isolated.
            if object == "oGiantSpider" {
                e.health = 3;
            }
            let hp = e.health;
            // TombLord normally has bloodless=true. A controlled override
            // makes the offset observable; suppression is covered separately.
            e.data.insert("bloodless".into(), 0.0);
            if blocked {
                if object == "oAlienBoss" {
                    e.set_sprite("sAlienBossHurt");
                } else {
                    e.data.insert("hit_cooldown".into(), 10.0);
                }
            }
            let rock = aimed_item(&mut game, "oRock");
            if object == "oGiantSpider" {
                // sGiantSpider's live bbox starts17px below its origin.
                game.world.entity_mut(rock).unwrap().pos.y = origin.y + 24.0;
            }
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(target).unwrap().health,
                hp - i32::from(!blocked),
                "{object} blocked={blocked}"
            );
            assert_eq!(
                blood(&game).len(),
                usize::from(!blocked),
                "{object} blocked={blocked}"
            );
            if !blocked {
                assert_new_blood(
                    blood(&game)[0],
                    Vec2::new(origin.x + offset.x, origin.y + offset.y),
                );
            }
        }
    }
}

#[test]
fn actual_step_ordinary_shopkeeper_and_generic_blood_use_distinct_origins() {
    for (object, offset) in [
        ("oShopkeeper", Vec2::new(0.0, 0.0)),
        ("oBat", Vec2::new(8.0, 8.0)),
    ] {
        let mut game = room();
        let origin = Vec2::new(80.0, 80.0);
        let target = prepare_target(&mut game, object, origin);
        // A controlled surviving generic target avoids conflating its next
        // normal Step death burst with the inherited item hit branch.
        if object == "oBat" {
            game.world.entity_mut(target).unwrap().health = 2;
        }
        let hp = game.world.entity(target).unwrap().health;
        aimed_item(&mut game, "oRock");
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(target).unwrap().health,
            hp - 1,
            "{object}"
        );
        let drops = blood(&game);
        assert_eq!(drops.len(), 1, "{object}");
        assert_new_blood(
            drops[0],
            Vec2::new(origin.x + offset.x, origin.y + offset.y),
        );
    }
}

#[test]
fn actual_step_ordinary_stunned_invincible_and_ufo_controls_add_no_hit_blood() {
    for invincible in [false, true] {
        let mut game = room();
        let target = prepare_target(&mut game, "oCaveman", Vec2::new(80.0, 80.0));
        let e = game.world.entity_mut(target).unwrap();
        if invincible {
            e.data.insert("invincible".into(), 1.0);
        } else {
            e.state = 98;
            e.data.insert("bounced".into(), 1.0);
        }
        aimed_item(&mut game, "oRock");
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(target).unwrap().health, 3);
        assert!(blood(&game).is_empty());
    }
    let mut game = room();
    let target = prepare_target(&mut game, "oUFO", Vec2::new(80.0, 80.0));
    aimed_item(&mut game, "oRock");
    game.step(&InputFrame::default());
    assert!(game.world.entity(target).is_none());
    assert!(game.world.entities.iter().any(|e| e.object == "oExplosion"));
    assert!(
        blood(&game).is_empty(),
        "ordinary UFO branch explodes, never calls scrCreateBlood"
    );
}

#[test]
fn actual_step_dice_and_fragile_source_no_blood_branches_stay_separate() {
    // These literal no-Blood branches verify that ordinary script insertion
    // does not bleed into replacement Steps. Missing direct-Blood branches
    // require separate family fixes; this deliberately does not bless them.
    for object in ["oDice", "oJar", "oSkull"] {
        let mut game = room();
        let target = prepare_target(&mut game, "oCaveman", Vec2::new(80.0, 80.0));
        let e = game.world.entity_mut(target).unwrap();
        e.state = 98;
        e.data.insert("bounced".into(), 1.0);
        let item = aimed_item(&mut game, object);
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(target).unwrap().health,
            3,
            "{object}: already-stunned body"
        );
        assert!(
            blood(&game).is_empty(),
            "{object}: own Step blocks body Blood at98"
        );
        if object != "oDice" {
            assert!(
                game.world.entity(item).is_none(),
                "fragile still breaks on blocked body branch"
            );
        }
    }
    let mut game = room();
    game.world.set_tile(5, 6, "oBrick");
    let target = prepare_target(&mut game, "oMagmaMan", Vec2::new(80.0, 80.0));
    game.world.entity_mut(target).unwrap().timer = 30;
    let hp = game.world.entity(target).unwrap().health;
    let die = aimed_item(&mut game, "oDice");
    // A supported MagmaMan remains a body; upward Dice clears the support
    // before its impact query, preserving the source >3 speed condition.
    game.world.entity_mut(die).unwrap().vel.y = -4.0;
    game.step(&InputFrame::default());
    // oMagmaMan Step18 consumes one HP each frame; immunity means no
    // additional Dice damage, rather than freezing this source heat decay.
    assert_eq!(game.world.entity(target).unwrap().health, hp - 1);
    assert!(blood(&game).is_empty());
}

#[test]
fn ordinary_script_hit_create_consumes_exact_four_draws_or_zero_below_parent_cap() {
    // Isolate RNG from the real hit handler, after all fixture Create calls.
    // Do not test32: inherited Destroy/child-Create continuation is unresolved.
    for (count, bloodless) in [(0, false), (15, false), (16, false), (0, true)] {
        let mut game = room();
        let target = prepare_target(&mut game, "oCaveman", Vec2::new(80.0, 80.0));
        game.world
            .entity_mut(target)
            .unwrap()
            .data
            .insert("bloodless".into(), f64::from(bloodless));
        for _ in 0..count {
            game.spawn("oBone", Vec2::new(280.0, 48.0));
        }
        let rock = aimed_item(&mut game, "oRock");
        let mut expected = game.rng.clone();
        let emitted = !bloodless && count < 16;
        let expected_velocity = if emitted {
            let vx = expected.unit() * 4.0 - expected.unit() * 4.0;
            let vy = -1.0 - expected.unit() * 2.0;
            let grav = f64::from(1 + expected.range(6)) * 0.1;
            Some((Vec2::new(vx, vy), grav))
        } else {
            None
        };
        item_step_impacts(&mut game, rock);
        assert_eq!(game.world.entity(target).unwrap().health, 2);
        assert_eq!(blood(&game).len(), usize::from(emitted));
        if let Some((velocity, gravity)) = expected_velocity {
            let drops = blood(&game);
            assert_eq!(drops[0].vel, velocity);
            assert_eq!(drops[0].value("gravity"), gravity);
            assert_new_blood(drops[0], Vec2::new(88.0, 88.0));
        }
        assert_eq!(
            game.rng.next_u64(),
            expected.next_u64(),
            "count={count} bloodless={bloodless}"
        );
    }
}
