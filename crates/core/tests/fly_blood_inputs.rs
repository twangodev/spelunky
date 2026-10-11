//! Source-derived Fly blood origins through a default Tomb Lord producer.
//! Actors/terrain are authored before play; subsequent actions are InputFrames.
use spelunky_core::{Entity, Game, InputFrame, RulesPreset, RunState, Vec2, level::Level};

fn room(canonical: bool, rules: RulesPreset, damsel: bool) -> (Game, u64) {
    let mut level = Level {
        music: "TEMPLE".into(),
        ..Default::default()
    };
    level.set_tile(if damsel { 5 } else { 0 }, 8, '@').unwrap();
    level.set_tile(14, 7, 'T').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 9, '1').unwrap();
    }
    if damsel {
        // Retreat under this cover after the throw. It ends before pickup.
        for x in 0..=4 {
            level.set_tile(x, 7, '1').unwrap();
        }
    } else {
        // Hang source checks collision_point(x,y-16), not a broad ceiling.
        level.set_tile(10, 6, '1').unwrap();
    }
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.load_custom(&level, "fly-blood-source", "");
    let lord = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oTombLord")
        .unwrap()
        .id;
    let e = game.world.entity(lord).unwrap();
    assert_eq!(
        (e.pos, e.state, e.facing, e.health),
        (Vec2::new(240.0, 128.0), 0, 1, 20)
    );
    assert!(!game.world.entities.iter().any(|e| e.object == "oFly"));
    (game, lord)
}

fn step(game: &mut Game, lord: u64, input: InputFrame) {
    game.step(&input);
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(
        game.player.health, 4,
        "NATURAL_FLY_ROUTE_HEALTH_PREREQUISITE"
    );
    assert_eq!(game.world.entity(lord).unwrap().health, 20);
}

fn fresh_blood(game: &Game) -> Vec<Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood" && e.age == 0)
        .cloned()
        .collect()
}

#[test]
fn natural_tomb_flies_hit_hanger_before_its_next_own_conversion_step() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, lord) = room(canonical, rules, false);
            let hanger = game.spawn("oGiantSpiderHang", Vec2::new(176.0, 128.0));
            let initial = game.world.entity(hanger).unwrap();
            assert_eq!(
                initial.data.get("image_xscale").copied().unwrap_or(1.0),
                1.0
            );
            assert_eq!(
                initial.data.get("image_yscale").copied().unwrap_or(1.0),
                1.0
            );
            let mut saw_fly = false;
            let mut witnessed = false;
            for _ in 0..180 {
                let before = game
                    .world
                    .entity(hanger)
                    .expect("default ceiling must retain hanger before first hit")
                    .clone();
                assert_eq!(
                    (
                        before.object.as_str(),
                        before.sprite.as_str(),
                        before.health
                    ),
                    ("oGiantSpiderHang", "sGiantSpiderHang", 10)
                );
                saw_fly |= game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oFly");
                step(&mut game, lord, InputFrame::default());
                let after = game
                    .world
                    .entity(hanger)
                    .expect("collision frame precedes next own conversion");
                if after.health < 10 {
                    assert!(saw_fly, "default Tomb must first produce an actual Fly");
                    assert_eq!(after.object, "oGiantSpiderHang");
                    assert_eq!(after.sprite, "sGiantSpiderHang");
                    assert_eq!(after.pos, before.pos);
                    assert_eq!((after.state, after.timer), (98, 20));
                    assert_eq!(after.value("image_xscale"), before.value("image_xscale"));
                    assert_eq!(
                        spelunky_core::catalog::sprite_dimensions(&after.sprite),
                        Some((32, 16))
                    );
                    let hits = ((10 - after.health) / 2) as usize;
                    assert!(hits > 0 && 10 - after.health == 2 * hits as i32);
                    // Account for simultaneous hits rather than assuming one.
                    // Positive budget is checked after HP subtraction; the
                    // sixth and later negative-HP hits spend the four units.
                    assert_eq!(
                        after.value("blood_left"),
                        (4isize - (hits as isize - 5).max(0)).max(0) as f64
                    );
                    let blood = fresh_blood(&game);
                    assert_eq!(blood.len(), hits.min(9));
                    for drop in blood {
                        assert_eq!(
                            drop.pos,
                            Vec2::new(after.pos.x + 16.0, after.pos.y + 8.0),
                            "SOURCE_FLY_HANG_CANVAS_32_BY_16"
                        );
                    }
                    witnessed = true;
                    break;
                }
            }
            assert!(witnessed, "NATURAL_FLY_HANGER_INTERCEPTION_PREREQUISITE");
        }
    }
}

#[test]
fn ordinary_damsel_pickup_throw_and_retreat_preserves_signed_draw_width() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, lord) = room(canonical, rules, true);
            let damsel = game.spawn("oDamsel", Vec2::new(104.0, 152.0));
            // Initial ordinary treasure caller overrides only: source
            // scrTreasureGen:76-78 clears cost/forSale after Damsel Create.
            let e = game.world.entity_mut(damsel).unwrap();
            e.data.insert("cost".into(), 0.0);
            e.data.insert("for_sale".into(), 0.0);
            for _ in 0..8 {
                step(&mut game, lord, InputFrame::default());
            }
            assert!(game.player.grounded);
            step(
                &mut game,
                lord,
                InputFrame {
                    down: true,
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!(game.player.held, Some(damsel));
            step(&mut game, lord, InputFrame::default());
            assert_eq!(game.player.facing, 1);
            assert_eq!(game.world.entity(damsel).unwrap().facing, 1);
            assert_eq!(
                game.world.entity(damsel).unwrap().value("image_xscale"),
                -1.0
            );
            let mut attack = false;
            for _ in 0..120 {
                if game.world.entity(lord).unwrap().state == 3 {
                    attack = true;
                    break;
                }
                step(&mut game, lord, InputFrame::default());
            }
            assert!(attack, "NATURAL_TOMB_ATTACK_PREREQUISITE");
            step(
                &mut game,
                lord,
                InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!(game.player.held, None);
            assert!(!game.world.entity(damsel).unwrap().held);
            assert_eq!(game.world.entity(damsel).unwrap().health, 4);
            assert_eq!(game.world.entity(damsel).unwrap().state, 2);
            let mut saw_fly = false;
            let mut witnessed = false;
            for tick in 0..160 {
                let before = game
                    .world
                    .entity(damsel)
                    .expect("thrown Damsel survives before first hit")
                    .clone();
                assert_eq!(before.health, 4);
                assert_eq!(before.value("image_xscale"), -1.0);
                saw_fly |= game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oFly");
                step(
                    &mut game,
                    lord,
                    InputFrame {
                        left: tick < 20,
                        ..Default::default()
                    },
                );
                let after = game.world.entity(damsel).unwrap();
                if after.health < 4 {
                    assert!(saw_fly, "default Tomb must produce Fly before damage");
                    assert_eq!(
                        after.health, 2,
                        "NATURAL_FLY_SINGLE_DAMSEL_HIT_PREREQUISITE"
                    );
                    assert_eq!((after.state, after.timer), (2, 120));
                    assert!(!after.held);
                    assert_eq!(game.player.held, None);
                    assert_eq!(
                        after.value("blood_left"),
                        4.0,
                        "Damsel blood budget check precedes HP subtraction"
                    );
                    assert_eq!(after.value("image_xscale"), -1.0);
                    assert_eq!(
                        spelunky_core::catalog::sprite_dimensions(&after.sprite),
                        Some((16, 16))
                    );
                    let blood = fresh_blood(&game);
                    assert_eq!(blood.len(), 1);
                    assert_eq!(
                        blood[0].pos,
                        Vec2::new(after.pos.x - 8.0, after.pos.y + 8.0),
                        "SOURCE_FLY_DAMSEL_SIGNED_WIDTH"
                    );
                    witnessed = true;
                    break;
                }
            }
            assert!(
                witnessed,
                "NATURAL_FLY_THROWN_DAMSEL_INTERCEPTION_PREREQUISITE"
            );
        }
    }
}
