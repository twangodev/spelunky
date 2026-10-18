//! Authored default support/stock Mattock, followed only by ordinary inputs.
//! Original MattockHit AnimationEnd uses a point gate, then an independent
//! translated precise instance_place in current merged Solid-family order.
use spelunky_core::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    ActorFirst,
    TileFirst,
    Primary,
    NoGate,
    Rotated,
}

struct Scene {
    game: Game,
    layout: Layout,
    historical: bool,
    actor: u64,
    mattock: u64,
    column: usize,
    row: usize,
    hero: Vec2,
}

fn ready(rules: RulesPreset, facing: i8, layout: Layout, historical: bool) -> Scene {
    let mut game = if historical {
        Game::legacy_shell(23, rules)
    } else {
        Game::shell(23, rules)
    };
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    let column = if facing > 0 { 10 } else { 8 };
    let row = if layout == Layout::Primary { 9 } else { 10 };
    let hero = Vec2::new(if facing > 0 { 144.0 } else { 159.0 }, 152.0);
    game.player.pos = hero;
    game.player.previous_pos = hero;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = hero;
    // The ordinary Block actor is the Player's support. Leave both possible
    // query cells empty until their explicitly ordered initial births below.
    for x in 0..30 {
        if x != 9 && x != column {
            game.world.set_tile(x, 10, "oHardBlock");
        }
    }
    if layout == Layout::TileFirst {
        game.world.set_tile(column, row, "oHardBlock");
    }
    // Protected-first is an authored mixed-terrain control. AlienShipFloor
    // has the same default16px rectangle and depth100 as HardBlock, without
    // Brick's random Create. Equal depth preserves this birth-order control
    // through stable Draw sorting and subsequent family-array rebuilds.
    let actor = game.spawn(
        if layout == Layout::TileFirst {
            "oAlienShipFloor"
        } else {
            "oBlock"
        },
        Vec2::new(144.0, 160.0),
    );
    if layout == Layout::Rotated {
        // Separate controlled/import-style unsupported-domain control. This
        // initial full-turn property is never assigned during the route.
        game.world
            .entity_mut(actor)
            .unwrap()
            .data
            .insert("image_angle".into(), 360.0);
    }
    if layout != Layout::TileFirst && layout != Layout::NoGate {
        game.world.set_tile(
            column,
            row,
            if layout == Layout::Primary {
                "oBlock"
            } else {
                "oHardBlock"
            },
        );
    }
    let mattock = game.spawn("oMattock", hero);
    if layout == Layout::TileFirst {
        // Prove the intended initial constructor/order before the first input.
        let body = game.world.entity(actor).unwrap();
        let tile = game.world.tile(column as i32, row as i32).unwrap();
        assert_eq!(body.object, "oAlienShipFloor");
        assert_eq!(body.sprite, "sAlienFloor");
        assert_eq!(body.value("invincible"), 0.0);
        assert!(!body.held && body.alive && body.engine_active);
        assert_eq!(catalog::object(&body.object).unwrap().depth, 100);
        assert_eq!(catalog::object(&tile.object).unwrap().depth, 100);
        assert_eq!(catalog::sprite_frame_count(&body.sprite), 1);
        assert!(tile.creation_serial < body.creation_serial);
        let order = &game.dispatcher.registration_order;
        let earlier = order
            .iter()
            .position(|instance| {
                *instance == dispatcher::Instance::Terrain(row * game.world.width + column)
            })
            .unwrap();
        let later = order
            .iter()
            .position(|instance| *instance == dispatcher::Instance::Entity(actor))
            .unwrap();
        assert!(
            earlier < later,
            "FIXTURE protected tile registered before equal-depth support before first input"
        );
    }
    Scene {
        game,
        layout,
        historical,
        actor,
        mattock,
        column,
        row,
        hero,
    }
}

fn source_probe(hit: &Entity, point: Vec2, animation: f64) -> catalog::CollisionMask {
    catalog::instance_mask(
        &hit.object,
        &hit.sprite,
        point,
        animation,
        Vec2::new(
            hit.data.get("image_xscale").copied().unwrap_or(1.0),
            hit.data.get("image_yscale").copied().unwrap_or(1.0),
        ),
        -hit.data
            .get("image_angle")
            .copied()
            .unwrap_or(0.0)
            .to_radians(),
    )
}

fn dig_sounds(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| {
            matches!(event,
                GameEvent::Sound { name, .. } if name == "sndCrunch" || name == "sndMattockBreak"
            )
        })
        .count()
}

fn point_hits(game: &Game, point: Vec2) -> bool {
    dispatcher::descendant_instances(game, "oSolid")
        .into_iter()
        .any(|instance| {
            if !dispatcher::is_active(game, instance) {
                return false;
            }
            match instance {
                dispatcher::Instance::Entity(id) => game
                    .world
                    .entity(id)
                    .is_some_and(|body| body.mask_contains(point, CollisionPrecision::BoundingBox)),
                dispatcher::Instance::Terrain(index) => game
                    .world
                    .tile_collision_mask(
                        (index % game.world.width) as i32,
                        (index / game.world.width) as i32,
                    )
                    .contains(point, CollisionPrecision::BoundingBox),
                dispatcher::Instance::Player => false,
            }
        })
}

fn query_geometry(scene: &Scene, hit: &Entity) {
    let game = &scene.game;
    assert_eq!(
        game.player.pos, scene.hero,
        "FIXTURE no Player displacement"
    );
    assert_eq!(game.player.health, 4.0, "FIXTURE no unrelated hurt");
    assert!(game.player.grounded);
    assert!(!game.player.is_stunned());
    assert!(
        game.player.attack_timer > 0,
        "FIXTURE live whipping before callback"
    );
    assert_eq!(game.player.held, Some(scene.mattock));
    assert!(hit.alive && hit.engine_active);
    assert_eq!(
        hit.pos,
        Vec2::new(scene.hero.x + 16.0 * f64::from(game.player.facing), 152.0)
    );
    assert_eq!(hit.value("damage"), 2.0);
    assert_eq!(hit.value("puncture"), 1.0);
    let count = catalog::sprite_frame_count(&hit.sprite) as f64;
    assert_eq!(count, 3.0, "FIXTURE bundled MattockHit frame count");
    assert_eq!(hit.image_speed(), 0.5);
    let wrapped = hit.animation + hit.image_speed() - count;
    assert_eq!(wrapped, 0.0, "SOURCE automatic wrap precedes AnimationEnd");
    let primary = hit.pos;
    let fallback = Vec2::new(primary.x, primary.y + 9.0);
    let actor = game
        .world
        .entity(scene.actor)
        .expect("FIXTURE support actor alive");
    assert!(actor.engine_active);
    if scene.layout == Layout::TileFirst {
        assert_eq!(actor.object, "oAlienShipFloor");
        assert_eq!(actor.sprite, "sAlienFloor");
    } else {
        assert_eq!(actor.object, "oBlock");
    }
    assert_eq!(actor.pos, Vec2::new(144.0, 160.0));
    assert_eq!(actor.value("invincible"), 0.0);
    assert!(!actor.mask_contains(primary, CollisionPrecision::BoundingBox));
    assert!(
        !actor.mask_contains(fallback, CollisionPrecision::BoundingBox),
        "SOURCE point gate does not identify the support actor"
    );
    let target = dispatcher::Instance::Terrain(scene.row * game.world.width + scene.column);
    let actor_instance = dispatcher::Instance::Entity(scene.actor);
    assert!(game.dispatcher.active_order.contains(&actor_instance));
    let order = &game.dispatcher.registration_order;
    let actor_order = order
        .iter()
        .position(|entry| *entry == actor_instance)
        .unwrap();
    let point = if scene.layout == Layout::Primary {
        primary
    } else {
        fallback
    };
    let probe = source_probe(hit, point, wrapped);
    let actor_mask = catalog::instance_mask(
        &actor.object,
        &actor.sprite,
        actor.pos,
        actor.animation,
        Vec2::new(1.0, 1.0),
        -actor.value("image_angle").to_radians(),
    );
    if scene.layout == Layout::Primary {
        assert!(!probe.intersects_mask(&actor_mask));
    } else {
        assert!(
            probe.intersects_mask(&actor_mask),
            "SOURCE translated mask reaches support"
        );
    }
    assert_eq!(
        point_hits(game, primary),
        scene.layout == Layout::Primary,
        "FIXTURE actual whole-family primary point gate"
    );
    if scene.layout != Layout::Primary {
        assert_eq!(
            point_hits(game, fallback),
            scene.layout != Layout::NoGate,
            "FIXTURE actual whole-family fallback point gate"
        );
    }
    if scene.layout == Layout::NoGate {
        assert!(
            game.world
                .tile(scene.column as i32, scene.row as i32)
                .unwrap()
                .object
                .is_empty()
        );
        return;
    }
    assert!(game.dispatcher.active_order.contains(&target));
    let tile_order = order.iter().position(|entry| *entry == target).unwrap();
    assert_eq!(
        actor_order < tile_order,
        scene.layout != Layout::TileFirst,
        "FIXTURE actual merged actor/terrain source array order"
    );
    let tile_mask = game
        .world
        .tile_collision_mask(scene.column as i32, scene.row as i32);
    assert_eq!(
        tile_mask.contains(primary, CollisionPrecision::BoundingBox),
        scene.layout == Layout::Primary
    );
    assert!(tile_mask.contains(point, CollisionPrecision::BoundingBox));
    assert!(probe.intersects_mask(&tile_mask));
    assert_eq!(
        game.world
            .tile(scene.column as i32, scene.row as i32)
            .unwrap()
            .object,
        if scene.layout == Layout::Primary {
            "oBlock"
        } else {
            "oHardBlock"
        }
    );
}

fn run(mut scene: Scene) {
    scene.game.step(&InputFrame::default());
    assert_eq!(
        scene
            .game
            .world
            .entity(scene.mattock)
            .unwrap()
            .value("cost"),
        0.0,
        "FIXTURE ordinary free stock after inherited Item policy"
    );
    scene.game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        scene.game.player.held,
        Some(scene.mattock),
        "FIXTURE actual pickup"
    );
    assert_eq!(scene.game.player.pickup_item_type, "Mattock");
    scene.game.step(&InputFrame::default());
    assert_eq!(scene.game.player.pos, scene.hero);
    assert_eq!(scene.game.player.state, PlayerState::Standing);
    scene.game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(
        scene.game.player.attack_timer > 0,
        "FIXTURE actual Mattock use"
    );
    assert_eq!(dig_sounds(&scene.game), 0);
    for _ in 0..32 {
        let hit = scene
            .game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oMattockHit")
            .cloned();
        if let Some(hit) = hit {
            let frames = catalog::sprite_frame_count(&hit.sprite) as f64;
            if (hit.animation + hit.image_speed()).floor() >= frames {
                query_geometry(&scene, &hit);
                scene.game.step(&InputFrame::default());
                assert!(
                    scene.game.world.entity(hit.id).is_none(),
                    "FIXTURE real AnimationEnd retires hitbox"
                );
                assert_eq!(scene.game.player.health, 4.0);
                let actor_gone = scene.game.world.entity(scene.actor).is_none();
                let tile_gone = scene
                    .game
                    .world
                    .tile(scene.column as i32, scene.row as i32)
                    .unwrap()
                    .object
                    .is_empty();
                let source_actor = !scene.historical && scene.layout == Layout::ActorFirst;
                assert_eq!(
                    actor_gone, source_actor,
                    "SOURCE separate instance_place selects the earlier overlapping support, not the point-hit tile"
                );
                assert_eq!(
                    tile_gone,
                    scene.layout == Layout::Primary || scene.layout == Layout::NoGate
                );
                assert_eq!(
                    dig_sounds(&scene.game),
                    usize::from(source_actor || scene.layout == Layout::Primary)
                );
                return;
            }
        }
        scene.game.step(&InputFrame::default());
    }
    panic!("FIXTURE natural MattockHit AnimationEnd not reached");
}

#[test]
fn ordinary_mattock_fallback_selects_translated_mask_actor() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            run(ready(rules, facing, Layout::ActorFirst, false));
        }
    }
}
#[test]
fn ordinary_mattock_fallback_keeps_first_protected_tile() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            run(ready(rules, facing, Layout::TileFirst, false));
        }
    }
}
#[test]
fn ordinary_mattock_primary_tile_still_digs() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            run(ready(rules, facing, Layout::Primary, false));
        }
    }
}
#[test]
fn ordinary_mattock_empty_points_do_not_use_mask_overlap_alone() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            run(ready(rules, facing, Layout::NoGate, false));
        }
    }
}
#[test]
fn historical_mattock_retains_legacy_point_receiver() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            run(ready(rules, facing, Layout::ActorFirst, true));
        }
    }
}
#[test]
fn controlled_full_turn_target_retains_unsupported_selector() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            run(ready(rules, facing, Layout::Rotated, false));
        }
    }
}
