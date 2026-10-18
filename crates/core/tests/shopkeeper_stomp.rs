//! Authored rooms with native gunless Shopkeeper2 and ordinary pickup/jump.
//! Free boots and the 16-Bone cap context are initial editor metadata; they
//! are not claims about untouched Create prices or generated campaign rooms.
use spelunky_core::{
    Area, Entity, Game, GameEvent, InputFrame, PlayerState, RngAlgorithm, RulesPreset, Vec2, World,
    catalog,
};
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const RNGS: [RngAlgorithm; 2] = [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1];

fn player_proxy(game: &Game) -> Entity {
    // Entity's public native-mask event query retains Player's exact object,
    // current sprite/frame/origin and signed Draw scale/angle properties.
    let mut proxy = Entity::new(0, "oPlayer1", game.player.pos);
    proxy.sprite = game.player.sprite.clone();
    proxy.animation = game.player.animation;
    proxy.data = game.player.data.clone();
    proxy
}

fn arena(rules: RulesPreset, rng: RngAlgorithm, x: f64, boots: bool, bones: usize) -> (Game, u64) {
    let mut game = Game::shell_with_algorithm(23, rules, rng).unwrap();
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    for x in 4..6 {
        game.world.set_tile(x, 6, "oHardBlock");
    }
    game.player.pos = Vec2::new(88.0, 88.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    // Native Create: ATTACK, hp20, no gun, deathTimer200. No actor state edit.
    let keeper = game.spawn("oShopkeeper2", Vec2::new(x, 144.0));
    if boots {
        let id = game.spawn("oSpikeShoes", game.player.pos);
        let item = game.world.entity_mut(id).unwrap();
        item.data.insert("cost".into(), 0.0);
        item.data.insert("forSale".into(), 0.0);
    }
    // These initial native Bone births are an explicit cap-control context.
    for _ in 0..bones {
        game.spawn("oBone", Vec2::new(240.0, 32.0));
    }
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    // Equipment requires this actual pickup action. Empty no-boots controls
    // use the identical elapsed input tape before the ordinary jump.
    game.step(&InputFrame {
        down: true,
        ..Default::default()
    });
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert!(game.player.grounded);
    assert_eq!(game.player.pos, Vec2::new(88.0, 88.0));
    assert_eq!(
        game.player.has("SpikeShoes"),
        boots,
        "ordinary contact must acquire authored free boots"
    );
    let actor = game.world.entity(keeper).unwrap();
    assert_eq!((actor.state, actor.health), (2, 20));
    assert_eq!(actor.value("has_gun"), 0.0);
    assert_eq!(
        actor.value("animation_speed"),
        1.0,
        "actual ATTACK Step establishes rate before stomp"
    );
    (game, keeper)
}

fn source_stomp(rules: RulesPreset, rng: RngAlgorithm, boots: bool, bones: usize) {
    // Bounded authored starting distances locate native precise contact; the
    // success predicate is actual enemy STUNNED plus Player bounce. Semantic
    // assertions run at that first contact; they never select a passing oracle.
    for x in [
        192.0, 208.0, 224.0, 240.0, 256.0, 272.0, 288.0, 304.0, 320.0,
    ] {
        let (mut game, id) = arena(rules, rng, x, boots, bones);
        for frame in 0..44 {
            let before_rng = game.rng.clone();
            let before_fall = game.player.data.get("fall_timer").copied().unwrap_or(0.0);
            let incoming_fall = if game.player.vel.y > 0.0 {
                before_fall + 1.0
            } else {
                0.0
            };
            let before_hp = game.world.entity(id).unwrap().health;
            let before_blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood")
                .map(|e| e.id)
                .collect();
            game.step(&InputFrame {
                right: true,
                jump: frame < 4,
                jump_pressed: frame == 0,
                jump_released: frame == 4,
                ..Default::default()
            });
            let actor = game.world.entity(id).unwrap();
            if actor.state != 98 || game.player.vel.y >= 0.0 {
                continue;
            }
            let shown = game.presentation();
            let shown_actor = shown.world.entity(id).unwrap();
            assert!(
                player_proxy(shown).event_intersects(shown_actor),
                "first accepted native stomp must genuinely contact"
            );
            assert!(matches!(
                game.player.state,
                PlayerState::Jumping | PlayerState::Falling
            ));
            assert_eq!(
                game.player.health, 4.0,
                "no unrelated throw or pellet damage"
            );
            assert_eq!(game.player.stun, 0);
            assert!(
                incoming_fall > 0.0,
                "descending ordinary jump must establish fallTimer"
            );
            let damage = (incoming_fall / 16.0).ceil() as i32 * if boots { 3 } else { 1 };
            assert_eq!(
                actor.health,
                before_hp - damage,
                "literal ceil fallTimer damage"
            );
            assert_eq!(
                game.player.data.get("fall_timer").copied().unwrap_or(-1.0),
                0.0
            );
            assert_eq!(actor.timer, 5);
            assert_eq!(actor.vel.y, -6.0);
            assert_eq!(
                actor.vel.x,
                // Source Step391 and557-559: left ATTACK assigns -3, then
                // unconditional friction adds 0.1 before Character Collision31.
                (-3.0 + 0.1)
                    + if game.player.pos.x < actor.pos.x + 8.0 {
                        1.0
                    } else {
                        -1.0
                    },
                "own ATTACK speed then source side kick"
            );
            assert_eq!(actor.value("killed_by_player"), 0.0);
            let drops: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood" && !before_blood.contains(&e.id))
                .collect();
            let active_bones = shown
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.engine_active && e.object == "oBone")
                .count();
            assert_eq!(
                active_bones, bones,
                "cap premise requires actual active Bone count at callback, not initial birth count"
            );
            let active_detritus = shown
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
                .count();
            assert_eq!(active_detritus, bones + usize::from(boots));
            assert_eq!(
                drops.len(),
                usize::from(boots),
                "own boots callback creates Blood directly even at 16 active detritus"
            );
            let mut expected_rng = before_rng;
            if boots {
                assert_eq!(
                    drops[0].pos,
                    Vec2::new(game.player.pos.x, game.player.pos.y + 8.0)
                );
                let vx = expected_rng.unit() * 4.0 - expected_rng.unit() * 4.0;
                let vy = -1.0 - expected_rng.unit() * 2.0;
                let gravity = f64::from(1 + expected_rng.range(6)) * 0.1;
                assert_eq!(drops[0].vel, Vec2::new(vx, vy));
                assert_eq!(drops[0].value("gravity"), gravity);
            }
            assert_eq!(
                game.rng.clone().next_u64(),
                expected_rng.next_u64(),
                "four Blood Create draws only; no inactive gun or body loot draw"
            );
            assert_eq!(
                actor.value("animation_speed"),
                0.5,
                "own stomp resets ATTACK animation rate"
            );
            assert!(
                game.events
                    .iter()
                    .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndHit"))
            );
            assert!(
                !game
                    .events
                    .iter()
                    .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndHurt"))
            );
            return;
        }
    }
    panic!(
        "ordinary jump failed to reach native Keeper stomp: boots={boots} bones={bones} rules={rules:?} rng={rng:?}"
    );
}

#[test]
fn ordinary_boots_jump_resets_keeper_rate_and_emits_direct_blood() {
    for rules in PRESETS {
        for rng in RNGS {
            source_stomp(rules, rng, true, 0);
        }
    }
}
#[test]
fn controlled_initial_bones_do_not_gate_ordinary_boots_stomp_blood() {
    for rules in PRESETS {
        for rng in RNGS {
            source_stomp(rules, rng, true, 16);
        }
    }
}
#[test]
fn ordinary_unbooted_jump_uses_ceil_damage_without_blood() {
    for rules in PRESETS {
        for rng in RNGS {
            source_stomp(rules, rng, false, 0);
        }
    }
}
#[test]
fn ordinary_jump_into_idle_keeper_is_ignored_by_own_guard() {
    for rules in PRESETS {
        for rng in RNGS {
            let mut game = Game::shell_with_algorithm(23, rules, rng).unwrap();
            game.world = World::new(30, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..30 {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            game.player.pos = Vec2::new(88.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.world.entrance = game.player.pos;
            let id = game.spawn("oShopkeeper", Vec2::new(80.0, 144.0));
            let mut overlap = false;
            for frame in 0..5 {
                game.step(&InputFrame {
                    jump: frame == 2,
                    jump_pressed: frame == 2,
                    jump_released: frame == 3,
                    ..Default::default()
                });
                let shown = game.presentation();
                overlap |= player_proxy(shown).event_intersects(shown.world.entity(id).unwrap());
                let actor = game.world.entity(id).unwrap();
                assert_eq!((actor.state, actor.health), (0, 20));
                assert_eq!(actor.value("animation_speed"), 0.5);
                assert_eq!(game.player.health, 4.0);
                assert_eq!(game.player.stun, 0);
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == "oBlood")
                );
            }
            assert!(
                overlap,
                "ignored idle control must actually overlap native masks"
            );
        }
    }
}
