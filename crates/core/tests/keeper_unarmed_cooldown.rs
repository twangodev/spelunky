//! Source Keeper Step394–397: positive firing countdown belongs to hasGun.
//! Prepared finite ordinary-input route, UNRUN. No mid-route actor writes.
use spelunky_core::*;
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
fn sound(g: &Game, name: &str) -> bool {
    g.events
        .iter()
        .any(|event| matches!(event, GameEvent::Sound{name:actual,..} if actual==name))
}
fn unharmed(g: &Game) {
    assert_eq!(g.player.health, 4.0, "FIXTURE_HERO_HP4");
    assert_eq!(g.player.stun, 0, "FIXTURE_HERO_STUN0");
    assert!(
        !g.player.is_stunned(),
        "FIXTURE_HERO_NO_THROW_OR_STUN_LATCH"
    );
    assert!(
        !g.events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. })),
        "FIXTURE_NO_DAMAGE_EVENT"
    );
}
fn step(g: &mut Game, input: &InputFrame) {
    g.step(input);
    unharmed(g);
}
fn room(rules: RulesPreset) -> (Game, u64) {
    let mut g = Game::shell(37, rules);
    g.world = World::new(40, 24, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..40 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    // Initial ordinary Running pose is authored, as in the accepted rear Whip
    // source fixture. Keeper health, state, gun, timers and fields are defaults.
    g.player.pos = Vec2::new(182.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.player.facing = 1;
    g.player.state = PlayerState::Running;
    g.player.grounded = true;
    g.player.vel.x = 4.0;
    g.world.entrance = g.player.pos;
    assert!(!g.player.has("SpikeShoes"));
    let id = g.spawn("oShopkeeper", Vec2::new(160.0, 144.0));
    let e = g.world.entity(id).unwrap();
    assert_eq!(e.state, 0);
    assert_eq!(e.health, 20);
    assert_eq!(e.value("has_gun"), 1.0);
    (g, id)
}
fn real_whip_then_first_shot_jump(g: &mut Game, id: u64) {
    let first = g.world.next_id;
    let mut hit = false;
    for frame in 0..16 {
        step(
            g,
            &InputFrame {
                right: true,
                run: true,
                attack: frame == 0,
                attack_pressed: frame == 0,
                ..Default::default()
            },
        );
        let e = g.world.entity(id).unwrap();
        if e.value("whipped") == 0.0 {
            assert_eq!(e.state, 0, "FIXTURE_ATTACK_EARNED_ONLY_BY_WHIP");
            continue;
        }
        assert_eq!(e.state, 2);
        assert_eq!(e.health, 20);
        assert_eq!(e.alarms[0], 10);
        assert!((e.vel.y + 2.0).abs() < 1e-9);
        assert!(sound(g, "sndHit"));
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.id >= first && matches!(e.object.as_str(), "oWhip" | "oWhipPre"))
        );
        hit = true;
        break;
    }
    assert!(hit, "FIXTURE_REAL_REAR_WHIP_REQUIRED");
    let first_shot = g.world.next_id;
    let mut source_rng = g.rng.clone();
    for _ in 0..6 {
        let _speed = source_rng.range(3);
        let _spread = source_rng.unit() - source_rng.unit();
    }
    step(
        g,
        &InputFrame {
            right: true,
            run: true,
            jump: true,
            jump_pressed: true,
            ..Default::default()
        },
    );
    let e = g.world.entity(id).unwrap();
    assert_eq!(e.state, 2);
    assert_eq!(e.health, 20);
    assert_eq!(e.value("has_gun"), 1.0);
    assert_eq!(e.value("firing"), 30.0);
    assert!(sound(g, "sndShotgun"), "FIXTURE_REAL_FIRST_SHOT");
    assert_eq!(e.facing, 1, "FIXTURE_RIGHT_FIRST_SHOT");
    assert_eq!(
        g.world
            .entities
            .iter()
            .filter(|e| e.id >= first_shot && e.object == "oBullet")
            .count(),
        6,
        "FIXTURE_EXACT_SIX_FIRST_PELLETS"
    );
    assert_eq!(
        g.world
            .entities
            .iter()
            .filter(|e| e.id >= first_shot && e.object == "oShotgunBlastRight")
            .count(),
        1,
        "FIXTURE_EXACT_RIGHT_FIRST_BLAST"
    );
    assert_eq!(
        g.world
            .entities
            .iter()
            .filter(|e| e.id >= first_shot && e.object == "oShotgunBlastLeft")
            .count(),
        0
    );
    assert_eq!(
        g.rng.state_bits(),
        source_rng.state_bits(),
        "FIXTURE_FIRST_SHOT_EXACT_18_RNG_CALLS"
    );
}
// Read-only proxy matches the public Player sprite/transform for contact proof.
fn hero_proxy(g: &Game) -> Entity {
    let mut p = Entity::new(0, "oPlayer1", g.player.pos);
    p.sprite = g.player.sprite.clone();
    p.animation = g.player.animation;
    p.data = g.player.data.clone();
    p
}
#[test]
fn real_shot_stomp_gun_drop_and_unarmed_recovery_retain_positive_cooldown() {
    for rules in PRESETS {
        let (mut g, id) = room(rules);
        real_whip_then_first_shot_jump(&mut g, id);
        let mut stomp = false;
        for _ in 0..12 {
            let pre = g.world.entity(id).unwrap().clone();
            let hero_state_before = g.player.state;
            let hero_xscale_before = g.player.data.get("image_xscale").copied().unwrap_or(1.0);
            let swimming_before = g.player.data.get("swimming").copied().unwrap_or(0.0);
            let alarm = pre.alarms[0];
            step(
                &mut g,
                &InputFrame {
                    left: true,
                    ..Default::default()
                },
            );
            let e = g.world.entity(id).unwrap();
            assert!(
                e.alarms[0] <= alarm,
                "FIXTURE_NO_SECOND_WHIP_COOLDOWN_REFRESH"
            );
            if e.state != 98 {
                assert_eq!(e.state, 2);
                assert_eq!(e.health, 20);
                continue;
            }
            assert_eq!(pre.state, 2, "FIXTURE_STOMP_FROM_ATTACK");
            assert_eq!(pre.health, 20, "FIXTURE_PRE_STOMP_HEALTH20");
            assert_eq!(pre.value("has_gun"), 1.0);
            assert!(pre.value("firing") > 0.0);
            assert!(matches!(
                hero_state_before,
                PlayerState::Jumping | PlayerState::Falling
            ));
            assert_eq!(swimming_before, 0.0, "FIXTURE_PRE_STOMP_NOT_SWIMMING");
            assert_eq!(g.player.data.get("swimming").copied().unwrap_or(0.0), 0.0);
            assert!(
                e.health > 0 && e.health < pre.health,
                "FIXTURE_REAL_NONLETHAL_STOMP_DAMAGE"
            );
            assert_eq!(e.timer, 5);
            assert_eq!(e.vel.y, -6.0, "FIXTURE_LITERAL_KEEPER_STOMP_UPWARD_KICK");
            assert_eq!(e.value("has_gun"), 1.0);
            assert!(e.value("firing") > 0.0);
            assert_eq!(e.value("animation_speed"), 0.5);
            assert!(g.player.vel.y < 0.0, "FIXTURE_REAL_STOMP_PLAYER_BOUNCE");
            assert!(matches!(
                g.player.state,
                PlayerState::Jumping | PlayerState::Falling
            ));
            // Read the Draw boundary before Animation, as the existing public
            // stomp fixture. Fixed LEFT continuation must preserve the scale
            // that was already present before this callback.
            let shown = g.presentation();
            assert_eq!(
                shown
                    .player
                    .data
                    .get("image_xscale")
                    .copied()
                    .unwrap_or(1.0),
                hero_xscale_before,
                "FIXTURE_STOMP_SCALE_NOT_CHANGED_BY_DRAW"
            );
            assert!(
                hero_proxy(shown).event_intersects(shown.world.entity(id).unwrap()),
                "FIXTURE_NATIVE_STOMP_MASK_CONTACT"
            );
            assert!((g.player.pos.x - (e.pos.x + 8.0)).abs() <= 8.0);
            assert!(g.player.pos.y < e.pos.y + 5.0);
            assert!(sound(&g, "sndHit"));
            stomp = true;
            break;
        }
        assert!(stomp, "FIXTURE_FIXED_LEFT_INPUTS_MUST_REACH_STOMP");
        let stomp_hp = g.world.entity(id).unwrap().health;
        let firing = g.world.entity(id).unwrap().value("firing");
        let first_drop = g.world.next_id;
        step(&mut g, &InputFrame::default());
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.state, 98);
        assert_eq!(e.health, stomp_hp);
        assert_eq!(e.value("has_gun"), 0.0, "FIXTURE_REAL_STUN_GUN_DROP");
        assert_eq!(
            e.value("firing"),
            firing,
            "FIXTURE_STUN_PHASE_RETAINS_COOLDOWN"
        );
        let dropped = g
            .world
            .entities
            .iter()
            .find(|e| e.id >= first_drop && e.object == "oShotgun")
            .expect("FIXTURE_REAL_DROPPED_GUN_ID")
            .id;
        let mut recovered = false;
        for _ in 0..96 {
            let pre = g.world.entity(id).unwrap().clone();
            assert_eq!(pre.state, 98, "FIXTURE_STUN_NOT_UNDONE_BY_REHIT");
            assert_eq!(pre.value("has_gun"), 0.0);
            step(&mut g, &InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(e.health, stomp_hp, "FIXTURE_NO_EXTRA_STOMP_DAMAGE");
            assert!(e.alarms[0] <= pre.alarms[0], "FIXTURE_NO_LATE_WHIP_REFRESH");
            assert_eq!(
                e.value("firing"),
                firing,
                "FIXTURE_STUN_COUNTDOWN_NO_FIRING_DECAY"
            );
            assert_eq!(
                e.value("has_gun"),
                0.0,
                "FIXTURE_DROPPED_GUN_NOT_RECLAIMED_EARLY"
            );
            assert!(
                g.world.entity(dropped).is_some(),
                "FIXTURE_GUN_REMAINS_A_SEPARATE_BODY"
            );
            if e.state == 2 {
                assert_eq!(pre.timer, 0, "FIXTURE_REAL_GROUNDED_STUN_TIMER_RECOVERY");
                recovered = true;
                break;
            }
            assert_eq!(e.state, 98);
        }
        assert!(
            recovered,
            "FIXTURE_NEUTRAL_RECOVERY_MUST_REACH_UNARMED_ATTACK"
        );
        let pre = g.world.entity(id).unwrap().clone();
        assert_eq!(pre.state, 2);
        assert!(pre.health > 0);
        assert_eq!(pre.value("has_gun"), 0.0);
        assert!(
            pre.value("firing") > 0.0,
            "FIXTURE_POSITIVE_UNARMED_COOLDOWN_REQUIRED"
        );
        let rng_before = g.rng.state_bits();
        let birth_before = g.world.next_id;
        step(&mut g, &InputFrame::default());
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.state, 2);
        assert_eq!(e.health, stomp_hp);
        assert_eq!(
            e.value("has_gun"),
            0.0,
            "FIXTURE_POST_CALLBACK_STILL_UNARMED"
        );
        assert!(e.value("firing") > 0.0);
        assert!(!sound(&g, "sndShotgun"));
        assert!(g.world.entity(dropped).is_some());
        assert_eq!(
            g.rng.state_bits(),
            rng_before,
            "KEEPER_UNARMED_CALLBACK_NO_RNG"
        );
        assert!(
            !g.world.entities.iter().any(|e| e.id >= birth_before
                && matches!(
                    e.object.as_str(),
                    "oBullet" | "oShotgunBlastLeft" | "oShotgunBlastRight"
                )),
            "KEEPER_UNARMED_CALLBACK_NO_SHOT_BIRTHS"
        );
        assert_eq!(
            e.value("firing"),
            pre.value("firing"),
            "KEEPER_UNARMED_POSITIVE_COOLDOWN_MUST_NOT_DECAY"
        );
    }
}
#[test]
fn armed_positive_cooldown_still_decays_after_real_shot() {
    for rules in PRESETS {
        let (mut g, id) = room(rules);
        real_whip_then_first_shot_jump(&mut g, id);
        let before = g.world.entity(id).unwrap().value("firing");
        let rng_before = g.rng.state_bits();
        let birth_before = g.world.next_id;
        step(
            &mut g,
            &InputFrame {
                right: true,
                run: true,
                ..Default::default()
            },
        );
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.state, 2);
        assert_eq!(e.value("has_gun"), 1.0);
        assert_eq!(
            e.value("firing"),
            before - 1.0,
            "KEEPER_ARMED_COUNTDOWN_CONTROL"
        );
        assert_eq!(
            g.rng.state_bits(),
            rng_before,
            "KEEPER_ARMED_COUNTDOWN_NO_RNG"
        );
        assert!(
            !g.world.entities.iter().any(|e| e.id >= birth_before
                && matches!(
                    e.object.as_str(),
                    "oBullet" | "oShotgunBlastLeft" | "oShotgunBlastRight"
                )),
            "KEEPER_ARMED_COUNTDOWN_NO_SHOT_BIRTHS"
        );
        assert!(!sound(&g, "sndShotgun"));
    }
}
#[test]
fn default_unarmed_zero_cooldown_remains_zero_control() {
    for rules in PRESETS {
        let mut g = Game::shell(37, rules);
        g.world = World::new(40, 24, Area::Mines, 1);
        g.world.name = "rLevel".into();
        for x in 0..40 {
            g.world.set_tile(x, 10, "oHardBlock");
        }
        g.player.pos = Vec2::new(80.0, 152.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.player.state = PlayerState::Standing;
        let id = g.spawn("oShopkeeper2", Vec2::new(160.0, 144.0));
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.state, 2);
        assert_eq!(e.value("has_gun"), 0.0);
        assert_eq!(e.value("firing"), 0.0);
        let rng_before = g.rng.state_bits();
        let birth_before = g.world.next_id;
        step(&mut g, &InputFrame::default());
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.state, 2);
        assert_eq!(e.value("has_gun"), 0.0);
        assert_eq!(e.value("firing"), 0.0);
        assert_eq!(g.rng.state_bits(), rng_before, "KEEPER_UNARMED_ZERO_NO_RNG");
        assert!(
            !g.world.entities.iter().any(|e| e.id >= birth_before
                && matches!(
                    e.object.as_str(),
                    "oBullet" | "oShotgunBlastLeft" | "oShotgunBlastRight"
                )),
            "KEEPER_UNARMED_ZERO_NO_SHOT_BIRTHS"
        );
        assert!(!sound(&g, "sndShotgun"));
    }
}
