//! Source-only test proposal, not compiled or executed by its author.
//!
//! oMattockHit AnimationEnd16-74: successful dig deletes one layer3 tile at
//! each raw receiver point (x,y-16)/(x,y+16), destroys that solid, then writes
//! all active Treasure state1 and retires active Spikes without a raw-point
//! Solid at (x,y+16). A protected first target or protected fallback misses.
//! Initial actors/terrain/background crops are authored scene geometry.
//! All acquisition, attack, motion, and activation use ordinary Game.step.
//! No velocity/health/held/status/animation/private weapon fields are injected.
//!
//! Important coverage limit: oTreasure Create sets state=ACTIVE(1). Its floor
//! Step literally assigns STATUS=STATIC, not state. No source state0 producer
//! was found; naturally settled treasure below is an idempotent state1 control,
//! not proof of state0->1 reactivation. No synthetic state0 fixture is used.
//! Mattock durability/visibility retirement and >=32 detritus continuation are
//! outside these tests. Neither successful durability result is forced.

use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

#[derive(Clone, Copy)]
enum Target {
    Tile,
    Dynamic,
    Protected,
    Missing,
}

struct Scene {
    game: Game,
    target: Target,
    target_id: Option<u64>,
    mattock: u64,
    gold: u64,
    upper: [u64; 2],
    lower: [u64; 2],
    layer_controls: [u64; 4],
    unsupported: u64,
    raw_point_miss: u64,
    supported: u64,
    inactive: u64,
}

fn background_tile(game: &mut Game, origin: Vec2, depth: f64) -> u64 {
    let id = game.spawn("oGenerationDecoration", origin);
    let tile = game.world.entity_mut(id).unwrap();
    tile.set_sprite("bgCaveTop3");
    // Authored tile_add crop geometry, matching the public representation in
    // generation::terrain/opening. A four-pixel crop distinguishes raw x144
    // from the erroneous solid-center x152; no actor behavior is assigned.
    for (key, value) in [
        ("decoration", 1.0),
        ("depth", depth),
        ("source_x", 0.0),
        ("source_y", 0.0),
        ("source_w", 4.0),
        ("source_h", 4.0),
    ] {
        tile.data.insert(key.into(), value);
    }
    id
}

fn scene(preset: RulesPreset, facing: i8, target: Target) -> Scene {
    let mut game = Game::shell(23, preset);
    game.world = World::new(80, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(if facing > 0 { 136.0 } else { 168.0 }, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    // Protected floor is also the source failed fallback at hit.y+9=161.
    for x in 0..80 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    let target_id = match target {
        Target::Tile => {
            game.world.set_tile(9, 9, "oBlock");
            None
        }
        Target::Dynamic => Some(game.spawn("oBlock", Vec2::new(144.0, 144.0))),
        Target::Protected => {
            game.world.set_tile(9, 9, "oHardBlock");
            None
        }
        Target::Missing => None,
    };
    let mattock = game.spawn("oMattock", game.player.pos);
    let gold = game.spawn("oGoldNugget", Vec2::new(192.0, 96.0));
    let upper = [
        background_tile(&mut game, Vec2::new(143.0, 127.0), 3.0),
        background_tile(&mut game, Vec2::new(143.0, 127.0), 3.0),
    ];
    let lower = [
        background_tile(&mut game, Vec2::new(143.0, 159.0), 3.0),
        background_tile(&mut game, Vec2::new(143.0, 159.0), 3.0),
    ];
    let layer_controls = [
        background_tile(&mut game, Vec2::new(143.0, 127.0), 2.0),
        background_tile(&mut game, Vec2::new(143.0, 159.0), 4.0),
        background_tile(&mut game, Vec2::new(160.0, 127.0), 3.0),
        background_tile(&mut game, Vec2::new(160.0, 159.0), 3.0),
    ];
    let unsupported = game.spawn("oSpikes", Vec2::new(272.0, 80.0));
    let supported = game.spawn("oSpikes", Vec2::new(224.0, 80.0));
    game.spawn("oBrickSmooth", Vec2::new(224.0, 96.0));
    let raw_point_miss = game.spawn("oSpikes", Vec2::new(288.0, 80.0));
    // This overlaps most of the spike footprint, but does not contain the
    // source raw support point288,96. It must not preserve these Spikes.
    game.spawn("oBrickSmooth", Vec2::new(289.0, 96.0));
    let inactive = game.spawn("oSpikes", Vec2::new(800.0, 80.0));
    Scene {
        game,
        target,
        target_id,
        mattock,
        gold,
        upper,
        lower,
        layer_controls,
        unsupported,
        raw_point_miss,
        supported,
        inactive,
    }
}

fn settle_and_pick_up(scene: &mut Scene) {
    let mut stable = 0;
    let mut previous = scene.game.world.entity(scene.gold).unwrap().pos;
    for _ in 0..64 {
        scene.game.step(&InputFrame::default());
        let gold = scene
            .game
            .world
            .entity(scene.gold)
            .expect("remote gold must not be collected");
        if gold.pos == previous && gold.vel == Vec2::default() {
            stable += 1;
        } else {
            stable = 0;
        }
        previous = gold.pos;
        if stable >= 2 {
            break;
        }
    }
    assert!(
        stable >= 2,
        "authored gold must settle by ordinary gravity before callback"
    );
    assert_eq!(
        scene.game.world.entity(scene.gold).unwrap().state,
        1,
        "source settling preserves state=ACTIVE"
    );
    for id in [scene.unsupported, scene.raw_point_miss, scene.supported] {
        assert!(scene.game.world.entity(id).unwrap().engine_active);
    }
    assert!(
        !scene
            .game
            .world
            .entity(scene.inactive)
            .unwrap()
            .engine_active
    );
    assert_layers(scene, 2);
    scene.game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        scene.game.player.held,
        Some(scene.mattock),
        "real down+attack pickup"
    );
    scene.game.step(&InputFrame::default());
    assert_eq!(scene.game.player.held, Some(scene.mattock));
    assert_eq!(scene.game.player.state, PlayerState::Standing);
    assert!(scene.game.player.grounded);
    assert!(!scene.game.player.is_stunned());
}

fn assert_layers(scene: &Scene, remaining_per_point: usize) {
    for pair in [scene.upper, scene.lower] {
        assert_eq!(
            pair.iter()
                .filter(|id| scene.game.world.entity(**id).is_some())
                .count(),
            remaining_per_point
        );
    }
    assert!(
        scene
            .layer_controls
            .iter()
            .all(|id| scene.game.world.entity(*id).is_some()),
        "wrong depth and outside raw-point crops survive"
    );
}

#[derive(Default)]
struct DigSounds {
    crunch: usize,
    mattock_break: usize,
}

impl DigSounds {
    fn observe(&mut self, game: &Game) {
        for event in &game.events {
            if let GameEvent::Sound { name, .. } = event {
                match name.as_str() {
                    "sndCrunch" => self.crunch += 1,
                    "sndMattockBreak" => self.mattock_break += 1,
                    _ => {}
                }
            }
        }
    }
}

fn ordinary_attack_to_animation_end(scene: &mut Scene) -> DigSounds {
    let mut sounds = DigSounds::default();
    scene.game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    sounds.observe(&scene.game);
    for _ in 0..40 {
        let previous = scene
            .game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oMattockHit")
            .cloned();
        if let Some(hit) = &previous {
            assert_eq!(
                hit.pos,
                Vec2::new(152.0, 152.0),
                "natural swing stays attached at the source target point"
            );
        }
        scene.game.step(&InputFrame::default());
        sounds.observe(&scene.game);
        if let Some(hit) = previous {
            if scene.game.world.entity(hit.id).is_none() {
                let count = spelunky_core::catalog::sprite_frame_count(&hit.sprite) as f64;
                assert!(
                    hit.animation + hit.image_speed() >= count,
                    "fixture must reach the real AnimationEnd, not Player cleanup"
                );
                return sounds;
            }
        }
    }
    panic!("ordinary grounded Mattock attack must produce a complete MattockHit animation");
}

fn assert_common_tail_controls(scene: &Scene) {
    let gold = scene.game.world.entity(scene.gold).unwrap();
    assert_eq!(
        gold.state, 1,
        "natural settled Treasure is an idempotent source state1 control"
    );
    assert_eq!(gold.vel, Vec2::default());
    assert!(
        scene.game.world.entity(scene.supported).is_some(),
        "literal raw support point keeps supported Spikes"
    );
    let inactive = scene.game.world.entity(scene.inactive).unwrap();
    assert!(
        !inactive.engine_active,
        "with oSpikes excludes the deactivated far instance"
    );
}

#[test]
fn natural_mattock_success_removes_one_raw_layer_three_tile_per_point_then_sweeps_active_spikes() {
    for preset in PRESETS {
        for facing in [-1_i8, 1] {
            for target in [Target::Tile, Target::Dynamic] {
                let mut scene = scene(preset, facing, target);
                settle_and_pick_up(&mut scene);
                let sounds = ordinary_attack_to_animation_end(&mut scene);
                match scene.target {
                    Target::Tile => assert!(scene.game.world.tile(9, 9).unwrap().object.is_empty()),
                    Target::Dynamic => {
                        assert!(scene.game.world.entity(scene.target_id.unwrap()).is_none())
                    }
                    _ => unreachable!(),
                }
                assert_layers(&scene, 1);
                assert!(
                    scene.game.world.entity(scene.unsupported).is_none(),
                    "distant active unsupported Spikes are included"
                );
                assert!(
                    scene.game.world.entity(scene.raw_point_miss).is_none(),
                    "bbox support overlap cannot replace the literal raw-point query"
                );
                assert_common_tail_controls(&scene);
                assert_eq!(
                    sounds.crunch + sounds.mattock_break,
                    1,
                    "successful callback evaluates exactly one durability result"
                );
            }
        }
    }
}

fn assert_failed_callback(mut scene: Scene) {
    settle_and_pick_up(&mut scene);
    let sounds = ordinary_attack_to_animation_end(&mut scene);
    assert_layers(&scene, 2);
    assert!(
        scene.game.world.entity(scene.unsupported).is_some(),
        "failed dig cannot run the global spike tail"
    );
    assert!(scene.game.world.entity(scene.raw_point_miss).is_some());
    assert_common_tail_controls(&scene);
    assert_eq!((sounds.crunch, sounds.mattock_break), (0, 0));
    if matches!(scene.target, Target::Protected) {
        assert_eq!(scene.game.world.tile(9, 9).unwrap().object, "oHardBlock");
    }
    assert_eq!(scene.game.world.tile(9, 10).unwrap().object, "oHardBlock");
}

#[test]
fn natural_mattock_protected_first_target_does_not_run_global_tail() {
    for preset in PRESETS {
        for facing in [-1_i8, 1] {
            assert_failed_callback(scene(preset, facing, Target::Protected));
        }
    }
}

#[test]
fn natural_mattock_empty_first_point_and_protected_floor_fallback_do_not_run_global_tail() {
    for preset in PRESETS {
        for facing in [-1_i8, 1] {
            assert_failed_callback(scene(preset, facing, Target::Missing));
        }
    }
}
