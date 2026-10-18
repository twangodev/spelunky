//! Source-derived spear-hit regressions; earlier proposals preserved separately.
//! Only Level terrain, default public births, initial free price, and InputFrame
//! controls are used. No post-start HP/state/frame/timer/velocity writes.
use spelunky_core::{
    Game, GameEvent, InputFrame, Rng, RngAlgorithm, RulesPreset, Vec2, level::Level,
};

const CUSTOM_BORDER: f64 = 16.0;
const TILE_SIZE: f64 = 16.0;
const ENTRANCE_OFFSET: f64 = 8.0;
const ENTRANCE_ROW: usize = 8;
const FLOOR_ROW: usize = 9;
const FLOOR_Y: f64 = CUSTOM_BORDER + FLOOR_ROW as f64 * TILE_SIZE; //160

fn room(rules: RulesPreset, damsel_x: f64, trap_x: f64, hero_tile: usize) -> (Game, u64, u64) {
    let mut level = Level::default();
    level.set_tile(hero_tile, ENTRANCE_ROW, '@').unwrap();
    level.set_tile(35, ENTRANCE_ROW, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, FLOOR_ROW, '1').unwrap();
    }
    let mut game = Game::shell(42, rules); // one fixed seed, no outcome filtering
    game.load_custom(&level, "natural-damsel-run-spear", "");
    let damsel = game.spawn("oDamsel", Vec2::new(damsel_x, 152.0));
    // Source producer override: free Damsel. No state/timer/speed override.
    game.world
        .entity_mut(damsel)
        .unwrap()
        .data
        .insert("cost".into(), 0.0);
    let trap = game.spawn("oSpearTrapTop", Vec2::new(trap_x, 144.0));
    let d = game.world.entity(damsel).unwrap();
    assert_eq!((d.state, d.health, d.timer), (0, 4, 200));
    assert_eq!(d.image_speed(), 0.5);
    assert_eq!(game.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    let expected_hero = Vec2::new(
        CUSTOM_BORDER + hero_tile as f64 * TILE_SIZE + ENTRANCE_OFFSET,
        CUSTOM_BORDER + ENTRANCE_ROW as f64 * TILE_SIZE + ENTRANCE_OFFSET,
    );
    assert_eq!(
        game.player.pos, expected_hero,
        "loader border plus entrance center"
    );
    assert_eq!(game.world.entrance, expected_hero);
    assert_eq!(expected_hero.y, 152.0);
    assert_eq!(d.pos, Vec2::new(damsel_x, 152.0));
    assert_eq!(
        d.bounds().y + d.bounds().h,
        FLOOR_Y,
        "default Damsel bottom is supported"
    );
    assert!(
        game.world
            .tiles_point(Vec2::new(damsel_x, FLOOR_Y), "oSolid")
    );
    assert!(
        game.world.tiles_point(Vec2::new(trap_x, FLOOR_Y), "oSolid"),
        "stock trap support is at y+16, not one row below"
    );
    assert!(
        game.world
            .tiles_point(Vec2::new(expected_hero.x, FLOOR_Y), "oSolid")
    );
    assert!(
        !game.world.tiles_point(d.pos, "oSolid"),
        "Damsel origin clear before first Step"
    );
    (game, damsel, trap)
}

fn safe_step(game: &mut Game, input: &InputFrame) {
    game.step(input);
    assert_eq!(
        game.player.health, 4.0,
        "Hero remains healthy after every ordinary input Step"
    );
    assert_eq!(
        game.player.stun, 0,
        "no player stun countdown after any Step"
    );
    assert!(
        !game.player.is_stunned(),
        "no independent source stunned flag after any Step"
    );
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. })),
        "no player Damage event after any Step"
    );
}

fn no_spear(game: &Game) {
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oSpearsLeft"),
        "strict earned-RUN prefix must precede first stock trap activation"
    );
}

fn draw_actor(game: &Game, id: u64) -> &spelunky_core::Entity {
    game.draw_view
        .as_ref()
        .expect("real Draw snapshot before animation")
        .world
        .entity(id)
        .expect("live actor at Draw")
}

fn literal_unit(rng: &mut Rng) -> f64 {
    rng.next_u64() as f64 / 4294967296.0
}

// This runs through the actual inherited body/child Damsel callback. Source
// sets counter120 at hit, then supported THROWN decrements it to119 in this
// same Step. Normal-Step other==self places its single direct Blood at +8,+8.
fn assert_hit(game: &Game, damsel: u64, trap_x: f64, serial_before: u64, before_rng: &Rng) {
    let d = draw_actor(game, damsel);
    assert_eq!((d.health, d.state, d.timer), (2, 2, 119));
    assert_eq!(d.vel, Vec2::new(4.0, -6.0)); // actual left-side Spears origin < Damsel
    assert!(!d.held);
    assert_eq!(game.player.held, None);
    assert!(game.player.pickup_item_type.is_empty());
    assert_eq!(
        d.image_speed(),
        0.5,
        "RUN .8 must be reset by actual spear hit"
    );
    assert!(game.events.iter().any(
        |event| matches!(event, GameEvent::Sound { name, pos } if name == "sndHit" && *pos == d.pos)
    ));
    let view = game.draw_view.as_ref().unwrap();
    let active: Vec<_> = view
        .world
        .entities
        .iter()
        .filter(|s| {
            s.alive
                && s.object == "oSpearsLeft"
                && s.sprite == "sSpearsLeft"
                && s.pos == Vec2::new(trap_x - 16.0, 144.0)
                && (19.0..28.0).contains(&s.animation)
        })
        .collect();
    assert_eq!(
        active.len(),
        1,
        "stock trap owns actual live damaging frame"
    );
    let spear = active[0];
    // Source prec0 uses the sprite's aggregate bbox, not the selected visual
    // frame19 bbox (8..15). For stock unit/unrotated sSpearsLeft the aggregate
    // is inclusive [7,0,15,14]. Keep this literal oracle off the production selector.
    assert_eq!(spear.sprite, "sSpearsLeft");
    assert_eq!(spear.data.get("image_xscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(spear.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(spear.data.get("image_angle").copied().unwrap_or(0.0), 0.0);
    assert_eq!(spear.data.get("rotation").copied().unwrap_or(0.0), 0.0);
    assert_eq!((spear.pos.x.fract(), spear.pos.y.fract()), (0.0, 0.0));
    let b = spelunky_core::Rect::new(spear.pos.x + 7.0, spear.pos.y, 9.0, 15.0);
    assert!(
        d.pos.x + 3.0 >= b.x
            && d.pos.x - 3.0 < b.x + b.w
            && d.pos.y + 3.0 >= b.y
            && d.pos.y - 3.0 < b.y + b.h
    );
    assert!(
        game.player.pos.x + 3.0 < b.x,
        "Hero is outside spear edge while attached Damsel reaches it"
    );
    assert_eq!(game.player.health, 4.0, "no player hurt/RNG contamination");
    let blood: Vec<_> = view
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oBlood" && e.creation_serial > serial_before)
        .collect();
    assert_eq!(
        blood.len(),
        1,
        "source direct birth, not capped blood helper"
    );
    let blood = blood[0];
    assert_eq!(blood.pos, Vec2::new(d.pos.x + 8.0, d.pos.y + 8.0));
    assert_eq!(blood.previous_pos, blood.pos);
    assert_eq!(
        blood.value("life"),
        60.0,
        "Blood resource370 already passed Damsel556: own Step waits until next frame"
    );
    assert_eq!(blood.value("world_effect_initialized"), 1.0);
    assert_eq!(blood.alarms, [1, 1, 5, -1, -1, -1, -1, -1, -1, -1, -1, -1]);
    let mut expected_rng = before_rng.clone();
    let velocity = Vec2::new(
        literal_unit(&mut expected_rng) * 4.0 - literal_unit(&mut expected_rng) * 4.0,
        -1.0 - literal_unit(&mut expected_rng) * 2.0,
    );
    let gravity = (1 + ((expected_rng.next_u64() * 6) >> 32)) as f64 * 0.1;
    assert_eq!(
        blood.vel, velocity,
        "four literal own-Create draws completed now"
    );
    assert_eq!(blood.value("gravity"), gravity);
    assert_eq!(game.rng.state_bits(), expected_rng.state_bits());
}

#[test]
fn default_public_damsel_earns_run_then_held_run_is_hit_by_stock_spear() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, damsel, _trap) = room(rules, 180.0, 304.0, 9);
        for _ in 0..8 {
            safe_step(&mut game, &InputFrame::default());
            no_spear(&game);
        }
        assert_eq!(game.player.pos, Vec2::new(168.0, 152.0));
        safe_step(
            &mut game,
            &InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(game.player.held, Some(damsel), "real source pickup");
        safe_step(&mut game, &InputFrame::default());
        let held = game.world.entity(damsel).unwrap();
        assert_eq!(
            held.pos,
            Vec2::new(game.player.pos.x + 4.0, game.player.pos.y)
        );
        safe_step(
            &mut game,
            &InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            },
        ); // ordinary gentle Damsel throw/drop
        assert_eq!(game.player.held, None);
        assert_eq!(game.world.entity(damsel).unwrap().state, 2);
        assert!(
            game.events
                .iter()
                .any(|e| matches!(e, GameEvent::Sound{name,..} if name=="sndThrow"))
        );
        let mut supported_recovery = 0;
        let mut run_seen = false;
        for _ in 0..200 {
            let prior = game.world.entity(damsel).unwrap().clone();
            safe_step(&mut game, &InputFrame::default());
            no_spear(&game);
            let d = game.world.entity(damsel).unwrap();
            assert_eq!(d.health, 4);
            if prior.state == 2 && d.timer < prior.timer {
                supported_recovery += 1;
            }
            if d.state == 1 && d.image_speed() == 0.8 {
                run_seen = true;
                break;
            }
        }
        assert!(
            run_seen && supported_recovery >= 118,
            "real throw/support recovery must earn RUN rather than injected state"
        );
        // Catch the genuine running Damsel using ordinary movement and pickup.
        // Geometric steering is fixed, never selects seeds or rewrites outcomes.
        for _ in 0..80 {
            if game.player.held == Some(damsel) {
                break;
            }
            let d = game.world.entity(damsel).unwrap();
            let nearby = (d.pos.x - game.player.pos.x).abs() <= 10.0;
            safe_step(
                &mut game,
                &InputFrame {
                    right: !nearby,
                    run: !nearby,
                    down: nearby,
                    attack: nearby,
                    attack_pressed: nearby,
                    ..Default::default()
                },
            );
            no_spear(&game);
        }
        assert_eq!(
            game.player.held,
            Some(damsel),
            "real re-pickup retains RUN .8"
        );
        let d = game.world.entity(damsel).unwrap();
        assert_eq!(d.state, 1);
        assert_eq!(d.image_speed(), 0.8);
        // Carry right, then use slow ordinary crawling to the legal hand-only
        // contact interval. Every unexpected early hit is a fixture failure.
        let mut stock_birth_seen = false;
        let mut healthy_hit = false;
        for _ in 0..180 {
            let before = game.world.entity(damsel).unwrap().clone();
            assert_eq!(before.state, 1);
            assert_eq!(before.image_speed(), 0.8);
            assert_eq!(before.health, 4);
            assert!(before.held);
            let serial = game.world.creation_serial;
            let rng = game.rng.clone();
            let move_right = game.player.pos.x < 289.0;
            let crawl = game.player.pos.x >= 279.0 && move_right;
            safe_step(
                &mut game,
                &InputFrame {
                    right: move_right,
                    down: crawl,
                    ..Default::default()
                },
            );
            stock_birth_seen |= game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oSpearsLeft");
            let d = game.world.entity(damsel).unwrap();
            if d.health == 2 {
                assert_hit(&game, damsel, 304.0, serial, &rng);
                healthy_hit = true;
                break;
            }
            assert!(
                game.player.pos.x <= 291.0 && game.player.health == 4.0,
                "legal carry target must leave Hero clear and healthy"
            );
            assert_eq!(d.health, 4, "no unrelated hazard damage");
        }
        assert!(
            stock_birth_seen && healthy_hit,
            "unrun route must actually reach held RUN .8 -> real stock Spear .5 reset"
        );
    }
}

#[test]
fn default_idle_half_speed_control_also_receives_real_stock_spear_hit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, damsel, _) = room(rules, 122.0, 128.0, 0);
        let mut hit = false;
        for _ in 0..40 {
            let d = game.world.entity(damsel).unwrap();
            assert_eq!((d.state, d.health), (0, 4));
            assert_eq!(d.image_speed(), 0.5);
            let serial = game.world.creation_serial;
            let rng = game.rng.clone();
            safe_step(&mut game, &InputFrame::default());
            if game.world.entity(damsel).unwrap().health == 2 {
                assert_hit(&game, damsel, 128.0, serial, &rng);
                hit = true;
                break;
            }
        }
        assert!(
            hit,
            "IDLE .5 control is actual stock trap contact, not callback injection"
        );
    }
}
