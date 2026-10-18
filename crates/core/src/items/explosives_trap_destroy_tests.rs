//! Literal own-Destroy controls; no source seed-layout equivalence is claimed.
use super::*;

fn callback_room() -> Game {
    let mut game = Game::shell(57, RulesPreset::Classic);
    game.world = World::new(24, 18, Area::Temple, 13);
    game.player.pos = Vec2::new(40.0, 200.0);
    game.player.previous_pos = game.player.pos;
    game
}

fn source_rubble_positions(rng: &mut Rng, origin: Vec2, door: bool) -> Vec<Vec2> {
    // Literal Door Destroy: two small pieces; narrower horizontal range.
    // Literal guarded trap Destroy: one large then two small pieces.
    // Rubble and RubbleSmall inherit RNG-free RubblePiece Create.
    let count = if door { 2 } else { 3 };
    let x_limit = if door { 5 } else { 9 };
    (0..count)
        .map(|_| {
            Vec2::new(
                origin.x + 8.0 + f64::from(rng.range(x_limit)) - f64::from(rng.range(x_limit)),
                origin.y + 8.0 + f64::from(rng.range(9)) - f64::from(rng.range(9)),
            )
        })
        .collect()
}

fn debris(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
        .collect()
}

#[test]
fn own_door_destroy_is_unconditional_narrow_and_once_only() {
    for clean_death in [false, true] {
        for clean_solids in [false, true] {
            let mut game = callback_room();
            let origin = Vec2::new(96.0, 80.0);
            let id = game.spawn("oDoor", origin);
            assert_eq!(game.world.entity(id).unwrap().value("invincible"), 0.0);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("cleanDeath".into(), if clean_death { 1.0 } else { 0.0 });
            if clean_solids {
                game.flags.insert("cleanSolids".into());
                game.data.insert("cleanSolids".into(), 1.0);
            }
            game.world.fluid_dirty = false;
            let mut source = game.rng.clone();
            let expected = source_rubble_positions(&mut source, origin, true);
            assert!(destroy_solid_self(&mut game, id));
            let actual = debris(&game);
            assert_eq!(
                actual.len(),
                2,
                "cleanDeath={clean_death}, cleanSolids={clean_solids}"
            );
            assert_eq!(actual.iter().map(|e| e.pos).collect::<Vec<_>>(), expected);
            assert!(
                actual
                    .iter()
                    .all(|e| e.object == "oRubbleSmall" && e.sprite == "sRubbleTanSmall")
            );
            assert!(
                !game.world.fluid_dirty,
                "Door own event does not inherit Solid"
            );
            assert!(!destroy_solid_self(&mut game, id));
            assert_eq!(debris(&game).len(), 2);
            assert_eq!(game.rng.next_u64(), source.next_u64());
        }
    }
}

#[test]
fn natural_bomb_input_destroys_authored_door_with_two_small_tan_pieces() {
    // A controlled authored room, then only public inputs: a real grounded
    // bomb drop, retreat, fuse and Explosion Collision. The Door is an initial
    // source-derived actor; this does not claim a generated-room input route.
    let mut game = Game::shell(3, RulesPreset::Classic);
    let mut level = crate::level::Level::default();
    level.set_tile(6, 5, '@').unwrap();
    game.load_custom(&level, "door-bomb-contract.lvl", "");
    // Initial source HardBlocks isolate the Door callback from floor rubble.
    for x in 0..game.world.width {
        game.world.set_tile(x, 7, "oHardBlock");
    }
    let door = game.spawn("oDoor", Vec2::new(136.0, 80.0));
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert_eq!(game.player.pos, Vec2::new(120.0, 104.0));
    assert!(game.world.entity(door).is_some());
    let original_health = game.player.health;
    game.step(&InputFrame {
        down: true,
        bomb_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.bombs, 3.0);
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oBomb" && e.value("armed") != 0.0)
    );
    let mut removed = false;
    for tick in 0..180 {
        game.step(&InputFrame {
            left: tick < 28,
            ..Default::default()
        });
        if game.world.entity(door).is_none() {
            let actual = debris(&game);
            assert_eq!(actual.len(), 2, "ordinary Door Explosion Collision");
            assert!(
                actual
                    .iter()
                    .all(|e| e.object == "oRubbleSmall" && e.sprite == "sRubbleTanSmall")
            );
            assert_eq!(game.player.health, original_health);
            removed = true;
            break;
        }
    }
    assert!(removed, "real bomb fuse must destroy the initial Door");
}

#[test]
fn own_non_spear_trap_destroy_uses_literal_three_piece_clean_guards() {
    for object in [
        "oSmashTrap",
        "oSmashTrapLit",
        "oCeilingTrap",
        "oArrowRepeaterL",
        "oArrowRepeaterR",
    ] {
        for clean_death in [false, true] {
            for clean_solids in [false, true] {
                let mut game = callback_room();
                let origin = Vec2::new(96.0, 80.0);
                let id = game.spawn(object, origin);
                assert_eq!(game.world.entity(id).unwrap().value("invincible"), 0.0);
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("cleanDeath".into(), if clean_death { 1.0 } else { 0.0 });
                if clean_solids {
                    game.flags.insert("cleanSolids".into());
                    game.data.insert("cleanSolids".into(), 1.0);
                }
                game.world.fluid_dirty = false;
                let mut source = game.rng.clone();
                let expected = if clean_death || clean_solids {
                    Vec::new()
                } else {
                    source_rubble_positions(&mut source, origin, false)
                };
                assert!(destroy_solid_self(&mut game, id));
                let actual = debris(&game);
                assert_eq!(
                    actual.iter().map(|e| e.pos).collect::<Vec<_>>(),
                    expected,
                    "{object}/{clean_death}/{clean_solids}"
                );
                if !expected.is_empty() {
                    assert_eq!(actual[0].object, "oRubble");
                    assert_eq!(actual[0].sprite, "sRubbleTan");
                    assert!(
                        actual[1..]
                            .iter()
                            .all(|e| e.object == "oRubbleSmall" && e.sprite == "sRubbleTanSmall")
                    );
                }
                assert!(
                    !game.world.fluid_dirty,
                    "{object} own event must not inherit Solid"
                );
                assert!(!destroy_solid_self(&mut game, id));
                assert_eq!(game.rng.next_u64(), source.next_u64());
            }
        }
    }
}

fn literal_jar_destroy_without_loot(rng: &mut Rng) -> bool {
    // oJar Destroy creates three rubble pieces before checking its short-circuiting
    // loot table. This focused order oracle chooses seeds with no loot branch.
    for _ in 0..3 {
        rng.range(3);
        rng.range(3);
        rng.range(4);
    }
    [3, 6, 12, 12, 12, 6, 12]
        .into_iter()
        .all(|denominator| rng.range(denominator) != 0)
}

#[test]
fn explosion_runs_jar_destroy_before_later_item_resource_rng() {
    // Source collision pairs order the Jar resource (635) before SpikeShoes
    // (876), regardless of instance creation order. Create shoes first to
    // ensure source resource order, not storage order, selects the RNG stream.
    let seed = (0..128)
        .find(|seed| {
            let mut probe = Game::shell(*seed, RulesPreset::Classic);
            literal_jar_destroy_without_loot(&mut probe.rng)
        })
        .expect("a seed without Jar loot exists in this range");
    let mut game = Game::shell(seed, RulesPreset::Classic);
    game.player.pos = Vec2::new(240.0, 200.0);
    let shoes = game.spawn("oSpikeShoes", Vec2::new(88.0, 88.0));
    let jar = game.spawn("oJar", Vec2::new(88.0, 88.0));
    game.world.entity_mut(shoes).unwrap().vel = Vec2::new(0.0, 0.0);

    let mut source_order = game.rng.clone();
    assert!(literal_jar_destroy_without_loot(&mut source_order));
    let expected_shoes_speed = 4.0 + f64::from(source_order.range(3));

    assert!(
        mask_intersects(
            Vec2::new(88.0, 88.0),
            0,
            game.world.entity(jar).unwrap().sprite_bounds()
        ),
        "jar bounds: {:?}",
        game.world.entity(jar).unwrap().sprite_bounds()
    );
    let explosion_id = game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
    init_entity(game.world.entity_mut(explosion_id).unwrap());
    let explosion = game.world.entity(explosion_id).unwrap().clone();
    apply_explosion(&mut game, &explosion);

    assert!(game.world.entity(jar).is_none());
    assert_eq!(
        game.world.entity(shoes).unwrap().vel.x,
        expected_shoes_speed
    );
}

fn literal_skull_destroy(rng: &mut Rng) {
    let mut pieces = 0;
    loop {
        if pieces >= 1 + rng.range(2) {
            break;
        }
        // Each Bone Create consumes four source draws before Skull's caller
        // overwrites the launch velocities with three more draws.
        rng.unit();
        rng.unit();
        rng.unit();
        rng.range(6);
        rng.range(3);
        rng.range(3);
        rng.range(4);
        pieces += 1;
    }
}

#[test]
fn explosion_runs_skull_destroy_before_later_item_resource_rng() {
    // Source collision pairs order the Skull resource (663) before
    // SpikeShoes (876), even though the fixture creates shoes first.
    let mut game = Game::shell(7, RulesPreset::Classic);
    game.player.pos = Vec2::new(240.0, 200.0);
    let shoes = game.spawn("oSpikeShoes", Vec2::new(88.0, 88.0));
    let skull = game.spawn("oSkull", Vec2::new(88.0, 88.0));
    game.world.entity_mut(shoes).unwrap().vel = Vec2::new(0.0, 0.0);

    let mut source_order = game.rng.clone();
    literal_skull_destroy(&mut source_order);
    let expected_shoes_speed = 4.0 + f64::from(source_order.range(3));

    assert!(
        mask_intersects(
            Vec2::new(88.0, 88.0),
            0,
            game.world.entity(skull).unwrap().sprite_bounds()
        ),
        "skull bounds: {:?}",
        game.world.entity(skull).unwrap().sprite_bounds()
    );
    let explosion_id = game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
    init_entity(game.world.entity_mut(explosion_id).unwrap());
    let explosion = game.world.entity(explosion_id).unwrap().clone();
    apply_explosion(&mut game, &explosion);

    assert!(game.world.entity(skull).is_none());
    assert_eq!(
        game.world.entity(shoes).unwrap().vel.x,
        expected_shoes_speed
    );
}
