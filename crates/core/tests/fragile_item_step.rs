//! Source-only proposal: not compiled or executed by its author.
//!
//! Canonical source: oJar Step134-209 / oSkull Step132-207, their Destroy
//! events, oItem Step383-414, oDamsel Step, and Player ordinary pickup/throw.
//! Initial actors and geometry are authored fixtures. No velocity, health,
//! invincibility, cooldown, held pointer, or item-private field is injected.
//! Resource order is Player319, Blood370, Rock450, Damsel556, Jar635,
//! AlienBoss658, Skull663. Thus new impact Blood has not moved this frame.
//! Geometric predictions below require the separately authorized CI run;
//! failure is evidence to inspect, not permission to weaken a source oracle.
//! V2 corrects source-clock arithmetic (engine_time=level_tick+1) and removes
//! V1's unsupported negative postwall-speed fixture. That coverage remains
//! open; no stationary rescued-Damsel-in-solid assumption is retained.

use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

const FRAGILE: [&str; 2] = ["oJar", "oSkull"];
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

fn room(preset: RulesPreset, player_x: f64, facing: i8) -> Game {
    let mut game = Game::shell(23, preset);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(player_x, 152.0);
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBrick");
    }
    game
}

#[derive(Default)]
struct Sounds {
    hit: usize,
    broken: usize,
}

impl Sounds {
    fn observe(&mut self, game: &Game) {
        for event in &game.events {
            if let GameEvent::Sound { name, .. } = event {
                match name.as_str() {
                    "sndHit" => self.hit += 1,
                    "sndBreak" => self.broken += 1,
                    _ => {}
                }
            }
        }
    }
}

fn pickup_and_stand(game: &mut Game, object: &str) -> u64 {
    let id = game.spawn(object, game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(id), "ordinary {object} pickup");
    game.step(&InputFrame::default());
    assert_eq!(game.player.held, Some(id));
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.world.entity(id).unwrap().vel, Vec2::default());
    id
}

fn throw_until_break(game: &mut Game, id: u64, down: bool) -> Sounds {
    let mut sounds = Sounds::default();
    game.step(&InputFrame {
        attack_pressed: true,
        down,
        ..Default::default()
    });
    sounds.observe(game);
    assert!(
        game.player.held.is_none(),
        "ordinary attack must release item"
    );
    for _ in 0..15 {
        if game.world.entity(id).is_none() {
            return sounds;
        }
        game.step(&InputFrame::default());
        sounds.observe(game);
    }
    panic!("the source-predicted fragile impact did not break within 16 frames");
}

fn fall_until_break(game: &mut Game, id: u64) -> Sounds {
    let mut sounds = Sounds::default();
    for _ in 0..20 {
        game.step(&InputFrame::default());
        sounds.observe(game);
        if game.world.entity(id).is_none() {
            return sounds;
        }
    }
    panic!("the source-predicted falling impact did not break within 20 frames");
}

fn blood(game: &Game) -> Vec<(u64, Vec2)> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .map(|e| (e.id, e.pos))
        .collect()
}

fn assert_blood_precedes_break(game: &Game, blood_id: u64, object: &str) {
    let smoke = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oSmokePuff")
        .expect("fragile Destroy must create smoke");
    assert!(
        blood_id < smoke.id,
        "impact Create precedes Destroy callback"
    );
    let debris = if object == "oJar" {
        "oRubbleSmall"
    } else {
        "oBone"
    };
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| { e.alive && e.object == debris && e.id > smoke.id }),
        "source break debris follows smoke"
    );
}

#[test]
fn ordinary_fragile_pickup_throw_damsel_creates_direct_blood_without_hit_sound() {
    for preset in PRESETS {
        for object in FRAGILE {
            for facing in [-1_i8, 1] {
                let px = if facing > 0 { 88.0 } else { 392.0 };
                let mut game = room(preset, px, facing);
                let damsel = game.spawn("oDamsel", Vec2::new(px + f64::from(facing) * 32.0, 152.0));
                let fragile = pickup_and_stand(&mut game, object);
                let sounds = throw_until_break(&mut game, fragile, false);
                let target = game.world.entity(damsel).unwrap();
                assert_eq!(
                    (target.health, target.state, target.timer),
                    (3, 2, 120),
                    "{preset:?}/{object}/{facing}"
                );
                assert_eq!(
                    target.value("hit_cooldown"),
                    0.0,
                    "fragile branch does not assign ordinary-item hit=10"
                );
                assert_eq!(target.vel.y, -6.0);
                let blood = blood(&game);
                assert_eq!(blood.len(), 1);
                // Release x=player+4*facing, then three source moveTo steps
                // Clock times4,5,6: (+8,-3), (+8,-2), (+8,-2).
                // First bbox overlap is therefore y145.
                assert_eq!(blood[0].1, Vec2::new(px + f64::from(facing) * 28.0, 145.0));
                assert_blood_precedes_break(&game, blood[0].0, object);
                assert_eq!((sounds.hit, sounds.broken), (0, 1));
            }
        }
    }
}

#[test]
fn source_runtime_keeper_type_controls_jar_and_dice_hit_branches() {
    for keeper_object in ["oShopkeeper", "oShopkeeper2"] {
        for (item_object, speed, expected_state, expected_health) in
            [("oJar", 5.0, 98, 20), ("oDice", 4.0, 2, 19)]
        {
            let mut game = room(RulesPreset::Classic, 40.0, 1);
            let keeper = game.spawn(keeper_object, Vec2::new(160.0, 144.0));
            assert_eq!(
                game.world
                    .entity(keeper)
                    .unwrap()
                    .text
                    .get("type")
                    .map(String::as_str),
                Some("Shopkeeper"),
                "{keeper_object} inherits the source type used by item impacts"
            );
            let item = game.spawn(item_object, Vec2::new(160.0, 144.0));
            game.world.entity_mut(item).unwrap().vel.x = speed;

            game.step(&InputFrame::default());

            let keeper = game.world.entity(keeper).unwrap();
            assert_eq!(
                keeper.health, expected_health,
                "{keeper_object}/{item_object}"
            );
            assert_eq!(
                keeper.state, expected_state,
                "{keeper_object}/{item_object}"
            );
            assert!(
                !keeper.data.contains_key("origX"),
                "Shopkeeper-specific impact must not take the ordinary enemy branch"
            );
            assert_eq!(
                game.world.entity(item).is_none(),
                item_object == "oJar",
                "Classic destroys an impacted jar, while dice continues rolling"
            );
        }
    }
}

#[test]
fn natural_wall_break_continues_into_enemy_before_destroy_and_direct_blood_ignores_script_cap() {
    for preset in PRESETS {
        for object in FRAGILE {
            for facing in [-1_i8, 1] {
                for initial_bones in [0, 16] {
                    let mut game = room(preset, if facing > 0 { 88.0 } else { 392.0 }, facing);
                    let (wall_x, boss_x) = if facing > 0 { (9, 136.0) } else { (20, 312.0) };
                    game.world.set_tile(wall_x, 9, "oBrick");
                    // Boss's 32-pixel body stands on this wall; its source
                    // psychicRecover=100 prevents attacks in this short scene.
                    let boss = game.spawn("oAlienBoss", Vec2::new(boss_x, 112.0));
                    let bones: Vec<_> = (0..initial_bones)
                        .map(|_| game.spawn("oBone", Vec2::new(240.0, 64.0)))
                        .collect();
                    let fragile = pickup_and_stand(&mut game, object);
                    let sounds = throw_until_break(&mut game, fragile, false);
                    assert_eq!(
                        game.world.entity(boss).unwrap().health,
                        9,
                        "{preset:?}/{object}/{facing}/{initial_bones}"
                    );
                    // Rightward wall-contact center139,143 => x136..142 query;
                    // the previous x132 query ends135 before boss bbox137.
                    // Mirrored center341 query338..344 first reaches bbox343.
                    // Reflected |vx|=4 still passes source's >2 impact gate.
                    let blood = blood(&game);
                    assert_eq!(blood.len(), 1);
                    assert_eq!(blood[0].1, Vec2::new(boss_x + 8.0, 120.0));
                    assert_blood_precedes_break(&game, blood[0].0, object);
                    assert_eq!((sounds.hit, sounds.broken), (1, 1));
                    assert!(
                        bones.iter().all(|id| game.world.entity(*id).is_some()),
                        "the direct-vs-script cap control must still contain its 16 natural Bones"
                    );
                    // No >=32 inherited-Destroy/child-Create continuation claim.
                }
            }
        }
    }
}

#[test]
fn natural_rock_cooldown_does_not_suppress_later_same_frame_fragile_damsel_damage() {
    for preset in PRESETS {
        for object in FRAGILE {
            let mut game = room(preset, 40.0, 1);
            let damsel = game.spawn("oDamsel", Vec2::new(152.0, 152.0));
            let rock = game.spawn("oRock", Vec2::new(152.0, 100.0));
            let fragile = game.spawn(object, Vec2::new(152.0, 94.0));
            let sounds = fall_until_break(&mut game, fragile);
            assert!(
                game.world.entity(rock).is_some(),
                "ordinary Rock survives its impact"
            );
            let target = game.world.entity(damsel).unwrap();
            assert_eq!(
                (target.health, target.state, target.timer),
                (2, 2, 120),
                "{preset:?}/{object}"
            );
            // Rock450 creates hit10, Damsel556 decrements it to9 and moves
            // upward6, then Jar635/Skull663 must still take a second HP.
            assert_eq!(target.value("hit_cooldown"), 9.0);
            assert_eq!(target.vel.y, -6.0);
            let blood = blood(&game);
            assert_eq!(
                blood.len(),
                2,
                "ordinary Rock blood plus independent direct fragile blood"
            );
            assert_eq!(blood[0].1, Vec2::new(152.0, 152.0));
            assert_eq!(blood[1].1, Vec2::new(152.0, 140.0));
            assert!(blood[0].0 < blood[1].0);
            assert_blood_precedes_break(&game, blood[1].0, object);
            assert_eq!(
                (sounds.hit, sounds.broken),
                (1, 1),
                "only ordinary Rock plays sndHit"
            );
        }
    }
}

#[test]
fn naturally_rescued_invincible_damsel_still_takes_fragile_damage_without_blood() {
    for preset in PRESETS {
        for object in FRAGILE {
            let mut game = room(preset, 40.0, 1);
            game.spawn("oExit", Vec2::new(144.0, 144.0));
            let damsel = game.spawn("oDamsel", Vec2::new(152.0, 152.0));
            let fragile = game.spawn(object, Vec2::new(152.0, 94.0));
            game.step(&InputFrame::default());
            let target = game.world.entity(damsel).unwrap();
            assert_eq!(
                (target.state, target.health, target.value("invincible")),
                (4, 4, 1.0),
                "real Exit must produce the invincibility before impact"
            );
            assert!(blood(&game).is_empty());
            let sounds = fall_until_break(&mut game, fragile);
            let target = game.world.entity(damsel).unwrap();
            assert_eq!(
                (target.health, target.state, target.timer),
                (3, 2, 120),
                "{preset:?}/{object}"
            );
            assert_eq!(target.value("invincible"), 1.0);
            assert_eq!(target.vel.y, -6.0);
            assert!(
                blood(&game).is_empty(),
                "invincibility gates direct Blood, not fragile Damsel damage"
            );
            assert_eq!((sounds.hit, sounds.broken), (0, 1));
        }
    }
}

#[test]
fn natural_fragile_wall_break_without_target_keeps_destroy_callback_and_no_hit() {
    for preset in PRESETS {
        for object in FRAGILE {
            for facing in [-1_i8, 1] {
                let mut game = room(preset, if facing > 0 { 88.0 } else { 392.0 }, facing);
                game.world
                    .set_tile(if facing > 0 { 9 } else { 20 }, 9, "oBrick");
                let fragile = pickup_and_stand(&mut game, object);
                let sounds = throw_until_break(&mut game, fragile, false);
                assert!(blood(&game).is_empty());
                assert_eq!(
                    (sounds.hit, sounds.broken),
                    (0, 1),
                    "{preset:?}/{object}/{facing}"
                );
                let smoke: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oSmokePuff")
                    .collect();
                assert_eq!(smoke.len(), 1);
                // Same source throw/body geometry as the paired Boss scene.
                // Smoke490 has already had its Step when Jar635/Skull663
                // creates it; its own y-=.1 starts on the following frame.
                assert_eq!(
                    smoke[0].pos,
                    Vec2::new(if facing > 0 { 139.0 } else { 341.0 }, 143.0)
                );
                let debris = if object == "oJar" {
                    "oRubbleSmall"
                } else {
                    "oBone"
                };
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == debris && e.id > smoke[0].id)
                );
            }
        }
    }
}
