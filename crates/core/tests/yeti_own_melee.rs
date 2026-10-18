//! Yeti own Whip/WhipPre source callbacks. Ordinary Machete fixtures preserve
//! default Yeti Create and use real pickup/attack. Initial paused-body, HP,
//! bloodless/cap/bloodLeft controls are explicitly controlled source states.
use spelunky_core::*;
fn room(legacy: bool, seed: u32, rules: RulesPreset, player_x: f64, facing: i8) -> Game {
    let mut game = if legacy {
        Game::legacy_shell(u64::from(seed), rules)
    } else {
        Game::shell(seed, rules)
    };
    game.world = World::new(30, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(player_x, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.facing = facing;
    game
}
fn blood(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect()
}
fn sound_count(game: &Game, sound: &str) -> usize {
    game.events
        .iter()
        .filter(|e| matches!(e, GameEvent::Sound{name,..} if name==sound))
        .count()
}
fn pickup_machete(game: &mut Game, id: u64) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(id), "ORDINARY_MACHETE_PICKUP");
    assert_eq!(game.world.entity(id).unwrap().object, "oMachete");
    game.step(&InputFrame::default());
}
fn attack_until_hit(game: &mut Game, id: u64, hp: i32) -> (Rng, bool) {
    let mut saw_slash = false;
    for tick in 0..18 {
        saw_slash |= game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oSlash");
        let before = game.rng.clone();
        game.step(&InputFrame {
            attack: tick == 0,
            attack_pressed: tick == 0,
            ..Default::default()
        });
        let actor = game
            .world
            .entity(id)
            .expect("YETI_BODY_SURVIVES_FIRST_MELEE_EVENT");
        if actor.health != hp {
            assert_eq!(actor.health, hp - 2, "REAL_MACHETE_DAMAGE_NOT_STOMP");
            assert_eq!(actor.state, 98);
            assert_eq!(actor.timer, 200);
            assert_eq!(actor.vel.y, -3.0);
            assert_eq!(game.player.health, 4.0);
            assert_eq!(sound_count(game, "sndHit"), 1);
            return (before, saw_slash);
        }
    }
    panic!("ORDINARY_MACHETE_MUST_REACH_YETI");
}
fn ordinary_case(legacy: bool, facing: i8, velocity_only: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let seed = match (legacy, facing) {
            (false, -1) => 6,
            (true, 1) => 2,
            _ => 0,
        };
        let mut game = room(
            legacy,
            seed,
            rules,
            if facing > 0 { 80.0 } else { 128.0 },
            facing,
        );
        let blade = game.spawn("oMachete", game.player.pos);
        let id = game.spawn("oYeti", Vec2::new(104.0, 144.0));
        let source = game.world.entity(id).unwrap();
        assert_eq!(
            (source.health, source.state, source.vel),
            (5, 0, Vec2::new(2.5, 0.0))
        );
        assert_eq!(source.value("counter"), 0.0);
        let terrain: Vec<_> = game.world.tiles.iter().map(|t| t.object.clone()).collect();
        pickup_machete(&mut game, blade);
        let (_, saw_slash) = attack_until_hit(&mut game, id, 5);
        let actor = game.world.entity(id).unwrap();
        assert!(
            saw_slash,
            "OBSERVED_ORDINARY_FORWARD_SLASH_BEFORE_COLLISION_CONSUMES_IT"
        );
        if velocity_only {
            assert_eq!(actor.vel.x, -2.0, "FORWARD_PUNCTURE_USES_PLAYER_X_MINUS8");
        } else {
            assert_eq!(blood(&game).len(), 1, "YETI_DAMAGING_OWN_MELEE_BLOOD");
            assert_eq!(
                blood(&game)[0].pos,
                Vec2::new(actor.pos.x + 8.0, actor.pos.y + 8.0)
            );
            assert_eq!(
                blood(&game)[0].value("life"),
                60.0,
                "COLLISION_BIRTH_HAS_NOT_STEPPED"
            );
            assert_eq!(
                actor.value("blood_left"),
                4.0,
                "POSITIVE_HP_DOES_NOT_EXHAUST_BLOOD"
            );
        }
        assert_eq!(
            game.world
                .tiles
                .iter()
                .map(|t| t.object.clone())
                .collect::<Vec<_>>(),
            terrain
        );
    }
}
fn controlled_front_threshold(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(legacy, 0, rules, 80.0, 1);
        let blade = game.spawn("oMachete", game.player.pos);
        let id = game.spawn("oYeti", Vec2::new(88.0, 144.0));
        let actor = game.world.entity_mut(id).unwrap();
        actor.vel = Vec2::default();
        actor.data.insert("counter".into(), 50.0); // explicit source-reachable IDLE pause
        pickup_machete(&mut game, blade);
        attack_until_hit(&mut game, id, 5);
        let actor = game.world.entity(id).unwrap();
        assert_eq!(
            actor.vel.x, 2.0,
            "FRONT_PROJECTILE_EQUAL_CENTER_DIFFERS_FROM_PLAYER_TEST"
        );
        assert_eq!(actor.value("animation_speed"), 0.5);
    }
}
fn rear_control(legacy: bool, hp: i32, cap: usize, bloodless: bool, left: f64) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(legacy, 0, rules, 80.0, 1);
        for _ in 0..cap {
            game.spawn("oBone", Vec2::new(240.0, 64.0));
        }
        let blade = game.spawn("oMachete", game.player.pos);
        let id = game.spawn("oYeti", Vec2::new(56.0, 144.0));
        let actor = game.world.entity_mut(id).unwrap();
        // Controlled initial live HP, pause, script guards and source reserve.
        actor.health = hp;
        actor.vel = Vec2::default();
        actor.data.insert("counter".into(), 50.0);
        actor.data.insert("bloodless".into(), f64::from(bloodless));
        actor.data.insert("blood_left".into(), left);
        actor.data.insert("counts_as_kill".into(), 0.0);
        pickup_machete(&mut game, blade);
        let (mut expected, _) = attack_until_hit(&mut game, id, hp);
        let actor = game.world.entity(id).unwrap();
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oMachetePre")
        );
        assert_eq!(actor.vel.x, -2.0, "REAR_USES_ATTACKER_POSITION");
        assert_eq!(actor.value("counts_as_kill"), 1.0);
        let reserve = if hp - 2 < 0 && left > 0.0 {
            left - 1.0
        } else {
            left
        };
        assert_eq!(
            actor.value("blood_left"),
            reserve,
            "EXHAUSTION_FOLLOWS_HP_EVEN_IF_SCRIPT_SUPPRESSED"
        );
        let births = usize::from(cap < 16 && !bloodless && left > 0.0);
        assert_eq!(blood(&game).len(), births, "REAR_BLOOD_SCRIPT_GUARDS");
        if births == 1 {
            assert_eq!(
                blood(&game)[0].pos,
                Vec2::new(actor.pos.x + 8.0, actor.pos.y + 8.0)
            );
            for _ in 0..4 {
                expected.next_u64();
            }
        }
        assert_eq!(
            game.rng.state_bits(),
            expected.state_bits(),
            "COMPLETE_BLOOD_CREATE_DRAWS_ONLY_WHEN_BORN"
        );
    }
}
fn plain_control(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(legacy, 0, rules, 80.0, 1);
        let id = game.spawn("oYeti", Vec2::new(96.0, 144.0));
        let actor = game.world.entity_mut(id).unwrap();
        actor.vel = Vec2::default();
        actor.data.insert("counter".into(), 50.0);
        let mut hit = false;
        for tick in 0..18 {
            game.step(&InputFrame {
                attack: tick == 0,
                attack_pressed: tick == 0,
                ..Default::default()
            });
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.health, 5);
            assert_eq!(actor.value("blood_left"), 4.0);
            assert!(blood(&game).is_empty());
            if actor.value("hit_cooldown") > 0.0 {
                assert_eq!(actor.vel.y, -2.0);
                assert_eq!(actor.state, 0);
                hit = true;
                break;
            }
        }
        assert!(hit, "PLAIN_OWN_WHIP_BUMP_STILL_REACHED");
    }
}
#[test]
fn canonical_ordinary_right_blood() {
    ordinary_case(false, 1, false);
}
#[test]
fn legacy_ordinary_right_blood() {
    ordinary_case(true, 1, false);
}
#[test]
fn canonical_ordinary_left_blood() {
    ordinary_case(false, -1, false);
}
#[test]
fn legacy_ordinary_left_blood() {
    ordinary_case(true, -1, false);
}
#[test]
fn canonical_ordinary_left_knockback() {
    ordinary_case(false, -1, true);
}
#[test]
fn legacy_ordinary_left_knockback() {
    ordinary_case(true, -1, true);
}
#[test]
fn canonical_controlled_front_threshold() {
    controlled_front_threshold(false);
}
#[test]
fn legacy_controlled_front_threshold() {
    controlled_front_threshold(true);
}
#[test]
fn canonical_rear_healthy_blood() {
    rear_control(false, 5, 0, false, 4.0);
}
#[test]
fn legacy_rear_healthy_blood() {
    rear_control(true, 5, 0, false, 4.0);
}
#[test]
fn canonical_rear_lethal_exhaustion() {
    rear_control(false, 1, 0, false, 4.0);
}
#[test]
fn legacy_rear_lethal_exhaustion() {
    rear_control(true, 1, 0, false, 4.0);
}
#[test]
fn canonical_rear_lethal_full_cap() {
    rear_control(false, 1, 16, false, 4.0);
}
#[test]
fn legacy_rear_lethal_full_cap() {
    rear_control(true, 1, 16, false, 4.0);
}
#[test]
fn canonical_rear_lethal_bloodless() {
    rear_control(false, 1, 0, true, 4.0);
}
#[test]
fn legacy_rear_lethal_bloodless() {
    rear_control(true, 1, 0, true, 4.0);
}
#[test]
fn canonical_rear_exhausted_reserve() {
    rear_control(false, 1, 0, false, 0.0);
}
#[test]
fn legacy_rear_exhausted_reserve() {
    rear_control(true, 1, 0, false, 0.0);
}
#[test]
fn canonical_plain_whip_control() {
    plain_control(false);
}
#[test]
fn legacy_plain_whip_control() {
    plain_control(true);
}
