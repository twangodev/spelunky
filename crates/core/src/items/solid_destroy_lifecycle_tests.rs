//! Controlled nested-Create probes of the native callback-before-retirement
//! contract. Recursive requests test the Rust adapter's bounded execution
//! policy, not an unproven GM8 recursive-Destroy event-count guarantee.
use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type Probe = Rc<dyn Fn(&mut Game, u64)>;
thread_local! { static PROBE: RefCell<Option<Probe>> = RefCell::new(None); }
pub(crate) fn observe_create(game: &mut Game, id: u64) {
    let probe = PROBE.with(|probe| probe.borrow().clone());
    if let Some(probe) = probe {
        probe(game, id);
    }
}
struct Installed;
impl Drop for Installed {
    fn drop(&mut self) {
        PROBE.with(|probe| *probe.borrow_mut() = None);
    }
}
fn install(probe: impl Fn(&mut Game, u64) + 'static) -> Installed {
    PROBE.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Rc::new(probe));
    });
    Installed
}
fn game(rules: RulesPreset) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(24, 20, Area::Jungle, 5);
    game.flags.insert("custom".into());
    game.player.pos = Vec2::new(200.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game
}
fn receiver(game: &mut Game, terrain: bool, x: usize, object: &str) -> u64 {
    if terrain {
        game.world.set_tile(x, 5, object);
        game.world
            .spawn("oTerrainMetadata", Vec2::new(x as f64 * 16.0 + 8.0, 88.0))
    } else {
        game.world.spawn(object, Vec2::new(x as f64 * 16.0, 80.0))
    }
}
fn destroy(game: &mut Game, terrain: bool, x: usize, id: u64) -> bool {
    if terrain {
        destroy_terrain_self(game, x, 5)
    } else {
        destroy_solid_self(game, id)
    }
}
fn visible(game: &Game, terrain: bool, x: usize, id: u64) -> bool {
    game.world
        .entity(id)
        .is_some_and(|e| e.value("destroy_done") == 0.0)
        && (!terrain || game.world.tile(x as i32, 5).unwrap().object == "oBrick")
        && game.world.solid_at(x as f64 * 16.0 + 8.0, 88.0)
}

#[test]
fn callbacks_keep_receiver_and_metadata_live_through_inheritance_and_final_loot() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for terrain in [false, true] {
            let mut game = game(rules);
            let id = receiver(&mut game, terrain, 5, "oBrick");
            let lamp = game.world.spawn("oLamp", Vec2::new(80.0, 96.0));
            assert!(
                game.world
                    .entity(lamp)
                    .unwrap()
                    .sprite_bounds()
                    .contains(Vec2::new(88.0, 98.0))
            );
            if terrain {
                let metadata = game.world.entity_mut(id).unwrap();
                metadata.data.insert("gold_chunks".into(), 1.0);
                metadata.data.insert("embedded_treasure".into(), 1.0);
                metadata.text.insert("treasure".into(), "oRock".into());
            }
            let observed = Rc::new(RefCell::new(Vec::new()));
            let output = observed.clone();
            let _probe = install(move |game, child| {
                let object = game.world.entity(child).unwrap().object.clone();
                assert!(
                    visible(game, terrain, 5, id),
                    "receiver missing during {object} Create"
                );
                if object == "oLampItem" {
                    assert!(
                        game.world.entity(lamp).is_some(),
                        "lamp retires after Item Create"
                    );
                }
                output.borrow_mut().push(object);
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("nested_write".into(), 42.0);
                let json = serde_json::to_value(&*game).unwrap();
                assert!(json["dispatcher"].get("destroying_solids").is_none());
                assert!(json["dispatcher"].get("detached_actor_slots").is_none());
            });
            assert!(destroy(&mut game, terrain, 5, id));
            assert!(game.world.entity(id).is_none());
            assert!(game.world.entity(lamp).is_none());
            let objects = observed.borrow();
            assert_eq!(
                objects.iter().filter(|o| o.as_str() == "oLampItem").count(),
                1
            );
            assert_eq!(
                objects.iter().filter(|o| o.starts_with("oRubble")).count(),
                3
            );
            assert_eq!(
                objects.last().unwrap(),
                if terrain { "oRock" } else { "oRubbleSmall" }
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .find(|e| e.id == id)
                    .unwrap()
                    .value("nested_write"),
                42.0
            );
            assert!(game.dispatcher.destroying_solids.is_empty());
            let roundtrip: Game =
                serde_json::from_value(serde_json::to_value(&game).unwrap()).unwrap();
            assert!(roundtrip.dispatcher.destroying_solids.is_empty());
            assert!(!destroy(&mut game, terrain, 5, id));
        }
    }
}

#[test]
fn nested_sibling_destroy_is_visible_and_recursive_requests_do_not_duplicate_callbacks() {
    for terrain in [false, true] {
        let mut game = game(RulesPreset::Classic);
        let parent = receiver(&mut game, terrain, 5, "oBrick");
        let sibling = receiver(&mut game, terrain, 8, "oBrick");
        let entered = Rc::new(Cell::new(false));
        let nested = Rc::new(Cell::new(0));
        let once = entered.clone();
        let count = nested.clone();
        let _probe = install(move |game, child| {
            assert!(visible(game, terrain, 5, parent));
            assert!(
                !destroy(game, terrain, 5, parent),
                "Rust execution guard, not a native recursion oracle"
            );
            if !once.replace(true) {
                assert!(destroy(game, terrain, 8, sibling));
                assert!(game.world.entity(sibling).is_none());
            } else if game.world.entity(sibling).is_some() {
                assert!(visible(game, terrain, 8, sibling));
                assert!(!destroy(game, terrain, 8, sibling));
                count.set(count.get() + 1);
            }
            assert!(game.world.entity(child).is_some());
        });
        let mut oracle = game.rng.clone();
        // Rubble/RubbleSmall inherit RNG-free RubblePiece Create. Each
        // Brick birth consumes only four literal rand(0,8) position draws.
        for _ in 0..6 {
            for _ in 0..4 {
                oracle.range(9);
            }
        }
        assert!(destroy(&mut game, terrain, 5, parent));
        assert_eq!(nested.get(), 3);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object.starts_with("oRubble"))
                .count(),
            6
        );
        assert_eq!(game.rng.next_u64(), oracle.next_u64());
        assert!(game.dispatcher.destroying_solids.is_empty());
    }
}

#[test]
fn same_cell_replacement_survives_old_terrain_callback_retirement() {
    let mut game = game(RulesPreset::Classic);
    receiver(&mut game, true, 5, "oBrick");
    let first = Rc::new(Cell::new(true));
    let once = first.clone();
    let _probe = install(move |game, _| {
        if once.replace(false) {
            game.world.set_tile(5, 5, "oHardBlock");
        }
    });
    assert!(destroy_terrain_self(&mut game, 5, 5));
    assert_eq!(game.world.tile(5, 5).unwrap().object, "oHardBlock");
}

#[test]
fn default_game_step_tree_destroy_keeps_live_receiver_for_real_child_creates() {
    for terrain in [false, true] {
        let mut game = game(RulesPreset::Classic);
        let id = receiver(&mut game, terrain, 5, "oTree");
        assert!(!game.world.solid_at(80.0, 96.0));
        let creates = Rc::new(Cell::new(0));
        let count = creates.clone();
        let _probe = install(move |game, child| {
            if !game
                .world
                .entity(child)
                .unwrap()
                .object
                .starts_with("oRubble")
            {
                return;
            }
            assert!(
                game.world.entity(id).is_some(),
                "live tree/metadata inside actual Game.step child Create"
            );
            assert!(game.world.solid_at(88.0, 88.0));
            if terrain {
                assert_eq!(game.world.tile(5, 5).unwrap().object, "oTree");
            }
            count.set(count.get() + 1);
        });
        game.step(&InputFrame::default());
        assert_eq!(creates.get(), 3);
        assert!(game.world.entity(id).is_none());
        if terrain {
            assert!(game.world.tile(5, 5).unwrap().object.is_empty());
        }
    }
}

#[test]
fn detached_olmec_callback_publishes_identity_and_preserves_nested_writes() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        let boss = game.world.spawn("oOlmec", Vec2::new(80.0, 80.0));
        let after = game.world.spawn("oRock", Vec2::new(200.0, 80.0));
        let lamp = game.world.spawn("oLamp", Vec2::new(80.0, 96.0));
        crate::dispatcher::register_instances(&mut game);
        let serial = game.world.entity(boss).unwrap().creation_serial;
        let index = game
            .world
            .entities
            .iter()
            .position(|e| e.id == boss)
            .unwrap();
        let mut detached = game.world.entities.remove(index);
        detached.alive = false; // source own Step records death before instance_destroy
        game.dispatcher.detached_actor_slots.push((boss, index));
        let creates = Rc::new(Cell::new(0));
        let count = creates.clone();
        let _probe = install(move |game, child| {
            if game.world.entity(child).unwrap().object != "oLampItem" {
                return;
            }
            let receiver = game
                .world
                .entity(boss)
                .expect("borrowed Olmec published during nested Create");
            assert_eq!(receiver.creation_serial, serial);
            assert_eq!(receiver.value("destroy_done"), 0.0);
            assert_eq!(game.world.entities[0].id, boss);
            assert_eq!(game.world.entities[1].id, after);
            assert!(game.world.entity(lamp).is_some());
            assert!(
                crate::dispatcher::descendant_instances(game, "oOlmec")
                    .contains(&crate::dispatcher::Instance::Entity(boss))
            );
            // Exercise the actual shared child Create owner query, not just
            // storage inspection. This nested child must select the live boss.
            count.set(count.get() + 1);
            let position = Vec2::new(200.0, 80.0);
            let psychic = game.spawn("oPsychicCreate2", position);
            let angle = (96.0_f64 - position.y).atan2(112.0 - position.x);
            assert_eq!(
                game.world.entity(psychic).unwrap().vel,
                Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin())
            );
            game.world
                .entity_mut(boss)
                .unwrap()
                .data
                .insert("nested_write".into(), 42.0);
            assert!(!destroy_solid_self(game, boss));
            if rules == RulesPreset::Assisted {
                game.world.entity_mut(boss).unwrap().engine_active = false;
                game.world.entity_mut(lamp).unwrap().engine_active = false;
                crate::dispatcher::activate_region(game, Rect::new(1000.0, 1000.0, 16.0, 16.0));
            }
        });
        destroy_detached_olmec(&mut game, &mut detached);
        assert_eq!(creates.get(), 1);
        assert_eq!(detached.value("nested_write"), 42.0);
        assert!(!detached.alive);
        assert_eq!(detached.value("destroy_done"), 1.0);
        assert!(!game.world.entities.iter().any(|e| e.id == boss));
        assert!(game.world.entity(lamp).is_none());
        assert!(
            !game
                .dispatcher
                .hidden_entities()
                .iter()
                .any(|entity| entity.id == lamp && entity.alive)
        );
        assert_eq!(detached.engine_active, rules != RulesPreset::Assisted);
        game.dispatcher.detached_actor_slots.pop();
        assert!(game.dispatcher.destroying_solids.is_empty());
        destroy_detached_olmec(&mut game, &mut detached);
        assert_eq!(creates.get(), 1);
    }
}

#[test]
fn callback_deactivation_does_not_leave_hidden_receivers_alive() {
    for terrain in [false, true] {
        let mut game = game(RulesPreset::Classic);
        let id = receiver(&mut game, terrain, 5, "oBrick");
        crate::dispatcher::register_instances(&mut game);
        let first = Rc::new(Cell::new(true));
        let once = first.clone();
        let _probe = install(move |game, _| {
            if once.replace(false) {
                assert!(
                    game.world.entity(id).is_some(),
                    "source receiver is live at nested Create before activation mutation"
                );
                game.world.entity_mut(id).unwrap().engine_active = false;
                if terrain {
                    game.world.tiles[5 * game.world.width + 5].engine_active = false;
                }
                crate::dispatcher::activate_region(game, Rect::new(1000.0, 1000.0, 16.0, 16.0));
            }
        });
        assert!(destroy(&mut game, terrain, 5, id));
        assert!(
            !game
                .dispatcher
                .hidden_entities()
                .iter()
                .any(|e| e.id == id && e.alive)
        );
        assert!(
            !game
                .dispatcher
                .hidden_tiles()
                .iter()
                .any(|(i, _)| *i == 5 * game.world.width + 5)
        );
        crate::dispatcher::activate_object(&mut game, "oSolid");
        assert!(game.world.entity(id).is_none());
        if terrain {
            assert!(game.world.tile(5, 5).unwrap().object.is_empty());
        }
        assert!(game.dispatcher.destroying_solids.is_empty());
    }
}

#[test]
fn default_game_step_olmec_head_lava_callback_publishes_borrowed_receiver() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for lava in [false, true] {
            let mut game = game(rules);
            game.world.area = Area::Temple;
            game.world.level = 13;
            if lava {
                game.world.set_tile(5, 4, "oLava");
            }
            let boss = game.spawn("oOlmec", Vec2::new(80.0, 80.0));
            let lamp = game.spawn("oLamp", Vec2::new(80.0, 96.0));
            assert_eq!(game.world.tiles_point(Vec2::new(80.0, 78.0), "oLava"), lava);
            let creates = Rc::new(Cell::new(0));
            let count = creates.clone();
            let _probe = install(move |game, child| {
                if game.world.entity(child).unwrap().object != "oLampItem" {
                    return;
                }
                assert!(
                    game.world.entity(boss).is_some(),
                    "Olmec visible during actual head-lava Destroy callback"
                );
                assert!(game.world.entity(lamp).is_some());
                game.world
                    .entity_mut(boss)
                    .unwrap()
                    .data
                    .insert("nested_write".into(), 42.0);
                count.set(count.get() + 1);
            });
            game.step(&InputFrame::default());
            assert_eq!(creates.get(), usize::from(lava));
            assert_eq!(game.flags.contains("olmec_dead"), lava);
            assert_eq!(game.world.entity(boss).is_none(), lava);
            assert_eq!(game.world.entity(lamp).is_none(), lava);
            assert!(game.dispatcher.destroying_solids.is_empty());
            assert!(game.dispatcher.detached_actor_slots.is_empty());
        }
    }
}

#[test]
fn stock_altar_reentry_runs_inner_body_after_defile_clear_and_before_retirement() {
    for terrain in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for clean in ["none", "cleanDeath", "cleanSolids"] {
                for punish in [0, 1, 2] {
                    let mut game = game(rules);
                    let id = receiver(&mut game, terrain, 5, "oSacAltarLeft");
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("defile".into(), 1.0);
                    if clean == "cleanDeath" {
                        game.world
                            .entity_mut(id)
                            .unwrap()
                            .data
                            .insert("cleanDeath".into(), 1.0);
                    }
                    if clean == "cleanSolids" {
                        game.flags.insert("cleanSolids".into());
                    }
                    game.data.insert("kali_punish".into(), f64::from(punish));
                    let seen = Rc::new(Cell::new(0));
                    let count = seen.clone();
                    let _probe = install(move |game, child| {
                        let object = &game.world.entity(child).unwrap().object;
                        let live = game
                            .world
                            .entity(id)
                            .expect("origin remains live until inner callback returns");
                        if object.starts_with("oRubble") {
                            let i = count.get();
                            count.set(i + 1);
                            assert_eq!(live.value("defile"), if i < 3 { 1.0 } else { 0.0 });
                            assert_eq!(
                                game.data["kali_punish"],
                                f64::from(punish + usize::from(i >= 3) as i32)
                            );
                        } else if matches!(object.as_str(), "oBall" | "oChain" | "oGhost") {
                            assert_eq!(
                                game.data["kali_punish"],
                                f64::from(punish),
                                "source increments after punishment births"
                            );
                            assert_eq!(live.value("defile"), 1.0);
                            assert_eq!(game.data["favor"], -16.0);
                        }
                    });
                    let births = if clean == "none" { 6 } else { 0 };
                    let mut expected = game.rng.clone();
                    let mut positions = Vec::new();
                    for _ in 0..births {
                        positions.push(Vec2::new(
                            88.0 + f64::from(expected.range(9)) - f64::from(expected.range(9)),
                            88.0 + f64::from(expected.range(9)) - f64::from(expected.range(9)),
                        ));
                    }
                    assert!(destroy(&mut game, terrain, 5, id));
                    assert_eq!(seen.get(), births);
                    let rubble: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.object.starts_with("oRubble"))
                        .collect();
                    assert_eq!(rubble.iter().map(|e| e.pos).collect::<Vec<_>>(), positions);
                    assert_eq!(game.data["favor"], -16.0);
                    assert_eq!(game.data["kali_punish"], f64::from(punish + 1));
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .find(|e| e.id == id)
                            .unwrap()
                            .value("defile"),
                        0.0
                    );
                    assert!(game.world.entity(id).is_none());
                    if terrain {
                        assert!(game.world.tile(5, 5).unwrap().object.is_empty());
                    }
                    assert_eq!(game.rng.next_u64(), expected.next_u64());
                    assert!(game.dispatcher.destroying_solids.is_empty());
                    assert!(!game.flags.contains("altar_defile_cascade"));
                }
            }
        }
    }
}

#[test]
fn altar_fanout_preserves_mixed_active_birth_order_and_skips_inactive_sibling() {
    for origin_terrain in [false, true] {
        for clean in [false, true] {
            let mut game = game(RulesPreset::Classic);
            // Birth order intentionally differs from all-entities-then-tiles.
            let first = receiver(&mut game, true, 8, "oSacAltarRight");
            let origin = receiver(&mut game, origin_terrain, 5, "oSacAltarLeft");
            let last = receiver(&mut game, false, 11, "oSacAltarRight");
            let inactive = receiver(&mut game, false, 14, "oSacAltarRight");
            if clean {
                game.world
                    .entity_mut(origin)
                    .unwrap()
                    .data
                    .insert("cleanDeath".into(), 1.0);
            }
            crate::dispatcher::register_instances(&mut game);
            game.world.entity_mut(inactive).unwrap().engine_active = false;
            crate::dispatcher::activate_region(&mut game, Rect::new(1000.0, 1000.0, 16.0, 16.0));
            let calls = Rc::new(RefCell::new(Vec::new()));
            let observed = calls.clone();
            let _probe = install(move |game, child| {
                if !game
                    .world
                    .entity(child)
                    .unwrap()
                    .object
                    .starts_with("oRubble")
                {
                    return;
                }
                let live: Vec<_> = [first, origin, last]
                    .into_iter()
                    .filter(|id| game.world.entity(*id).is_some())
                    .collect();
                observed.borrow_mut().push(live);
            });
            assert!(destroy(&mut game, origin_terrain, 5, origin));
            assert!(game.world.entity(first).is_none());
            assert!(game.world.entity(last).is_none());
            assert!(
                game.dispatcher
                    .hidden_entities()
                    .iter()
                    .any(|e| e.id == inactive && e.alive)
            );
            // Outer origin, first tile, inner origin, then last entity. Once
            // the inner origin callback returns, last's Create sees it dead.
            let before = vec![first, origin, last];
            let middle = vec![origin, last];
            let after = vec![last];
            let mut expected = Vec::new();
            if !clean {
                expected.extend(vec![before.clone(); 3]);
            }
            expected.extend(vec![before; 3]);
            if !clean {
                expected.extend(vec![middle; 3]);
            }
            expected.extend(vec![after; 3]);
            assert_eq!(*calls.borrow(), expected);
            assert_eq!(game.data["favor"], -16.0);
            assert_eq!(game.data["kali_punish"], 1.0);
            assert!(game.dispatcher.destroying_solids.is_empty());
        }
    }
}
