//! City of Gold is live run state until the level-15 exit Animation End.
//! The Gold Door route uses actual pickup, movement, collision, exit, and
//! transition input. Excavation places the player at an existing generated
//! supported wall; it does not claim entrance-to-wall physical traversal.
mod common;

use common::complete_source_exit;
use spelunky_core::{
    Area, CollisionPrecision, Game, InputFrame, PlayerState, Rect, RulesPreset, RunState, Vec2,
    World,
    ending::{self, EndingKind, EndingStage},
};

fn temple_door_room() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(40, 20, Area::Temple, 14);
    game.world.name = "Temple".into();
    for x in 0..40 {
        game.world.set_tile(x, 4, "oBrickSmooth");
    }
    game.player.pos = Vec2::new(40., 56.);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game
}

fn pickup(game: &mut Game, object: &str) -> u64 {
    let id = game.spawn(object, game.player.pos);
    // Player319 runs before Crown789: a newly spawned priced pickup needs
    // its inherited Item Step to clear its no-shop price before the input edge.
    game.step(&InputFrame::default());
    let item = game
        .world
        .entity(id)
        .expect("pickup survives its first Step");
    assert_eq!(item.value("cost"), 0., "{object}: free pickup fixture");
    assert!(item.engine_active, "{object}: pickup must be active");
    assert_ne!(item.value("canPickUp"), 0., "{object}: pickup eligibility");
    assert!(
        !game.world.solid_at_except(item.pos.x, item.pos.y, id),
        "{object}: pickup origin must be outside solids"
    );
    let contact = Rect::new(game.player.pos.x - 8., game.player.pos.y, 17., 9.);
    assert!(
        item.mask_intersects(contact, CollisionPrecision::BoundingBox),
        "{object}: pickup mask must overlap the source contact gate"
    );
    assert_eq!(game.state, RunState::Playing, "{object}: live fixture");
    assert!(game.player.health > 0, "{object}: healthy fixture");
    assert!(!game.player.is_stunned(), "{object}: unstunned fixture");
    assert_eq!(game.player.attack_timer, 0, "{object}: idle attack fixture");
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    game.step(&InputFrame::default());
    id
}

fn enter_city_through_gold_door() -> Game {
    let mut game = temple_door_room();
    pickup(&mut game, "oCrown");
    assert!(game.player.has("Crown"));
    let sceptre = pickup(&mut game, "oSceptre");
    assert_eq!(game.player.held, Some(sceptre));
    let door = game.spawn("oGoldDoor", Vec2::new(104., 48.));
    for _ in 0..90 {
        game.step(&InputFrame {
            right: true,
            up: true,
            ..Default::default()
        });
        if game.state == RunState::Ending {
            break;
        }
    }
    assert!(game.world.entity(door).is_none());
    assert!(game.world.entity(sceptre).is_none());
    assert!(game.flags.contains("cityOfGold"));
    assert_eq!(game.data["next_level"], 15.);
    complete_source_exit(&mut game);
    assert!(game.flags.contains("cityOfGold"));
    game.next_level();
    assert_eq!(game.world.level, 15);
    assert_eq!(game.world.name, "City of Gold");
    assert!(game.flags.contains("cityOfGold"));
    assert!(!game.flags.contains("custom"));
    game
}

fn count_new(game: &Game, object: &str, first_id: u64) -> usize {
    game.world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.id >= first_id && entity.object == object)
        .count()
}

fn generated_excavation_site(game: &Game) -> (usize, usize, Vec2, i8) {
    for y in 2..game.world.height - 2 {
        for x in 2..game.world.width - 2 {
            let Some(tile) = game.world.tile(x as i32, y as i32) else {
                continue;
            };
            if tile.object != "oTemple" || !tile.is_destructible() {
                continue;
            }
            let center = Vec2::new(x as f64 * 16. + 8., y as f64 * 16. + 8.);
            let metadata = game.world.entities.iter().find(|entity| {
                entity.alive && entity.object == "oTerrainMetadata" && entity.pos == center
            });
            if metadata.is_some_and(|entity| entity.value("invincible") != 0.) {
                continue;
            }
            let sprite = metadata
                .and_then(|entity| entity.text.get("terrain_sprite"))
                .map(String::as_str)
                .unwrap_or("");
            // The normal-Temple control excavates a non-gold sprite. City
            // geometry naturally has sGTemple*, whose guaranteed loot still
            // depends on the live City flag in the Destroy callback.
            if sprite.contains("Gold") {
                continue;
            }
            if !game.flags.contains("cityOfGold")
                && !game.flags.contains("city_of_gold")
                && metadata.is_some_and(|entity| {
                    entity.value("gold_chunks") > 0. || entity.value("gold_nuggets") > 0.
                })
            {
                continue;
            }
            for facing in [-1i8, 1] {
                let pos = Vec2::new(center.x - 16. * f64::from(facing), center.y);
                if game
                    .world
                    .collides_except(Rect::new(pos.x - 6., pos.y - 8., 12., 16.), 0)
                    || !game.world.solid_at(pos.x, pos.y + 9.)
                {
                    continue;
                }
                // This positioned fixture needs a dry source bbox, including
                // Player's lava point y+6 and Item's rectangle +/-3 and point
                // y-5 (after the optional 0.05 sink). Keep the generated lava.
                if game.world.rectangle_family_except(
                    Vec2::new(pos.x - 8., pos.y - 8.),
                    Vec2::new(pos.x + 8., pos.y + 8.),
                    "oLava",
                    None,
                    CollisionPrecision::BoundingBox,
                ) {
                    continue;
                }
                if game.world.entities.iter().any(|entity| {
                    entity.alive
                        && spelunky_core::catalog::is_a(&entity.object, "oEnemy")
                        && entity.pos.distance(pos) < 96.
                }) {
                    continue;
                }
                return (x, y, pos, facing);
            }
        }
    }
    panic!("generated Temple fixture needs a quiet supported plain wall");
}

fn excavate_existing_wall(game: &mut Game) -> (usize, usize) {
    let (x, y, pos, facing) = generated_excavation_site(game);
    game.player.pos = pos;
    game.player.previous_pos = pos;
    game.player.vel = Vec2::default();
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.player.facing = facing;
    game.update_camera();
    game.previous_camera = game.camera;
    let mattock = pickup(game, "oMattock");
    assert_eq!(game.player.held, Some(mattock));
    let first_output_id = game.world.next_id;
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    for _ in 0..30 {
        if game.world.tile(x as i32, y as i32).unwrap().object != "oTemple" {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert_ne!(
        game.world.tile(x as i32, y as i32).unwrap().object,
        "oTemple"
    );
    (
        count_new(game, "oGoldChunk", first_output_id),
        count_new(game, "oGoldNugget", first_output_id),
    )
}

#[test]
fn real_gold_door_route_retains_city_runtime_destroy_loot() {
    let mut game = enter_city_through_gold_door();
    assert_eq!(excavate_existing_wall(&mut game), (3, 1));
    assert!(game.flags.contains("cityOfGold"));
}

#[test]
fn ordinary_temple_excavation_does_not_get_city_guaranteed_loot() {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.start_from_hub(false, 15);
    assert_eq!(game.world.name, "Temple");
    assert!(!game.flags.contains("cityOfGold"));
    assert!(!game.flags.contains("city_of_gold"));
    assert_eq!(excavate_existing_wall(&mut game), (0, 0));
}

#[test]
fn city_flags_clear_at_ordinary_exit_animation_end_before_transition_16() {
    let mut game = temple_door_room();
    game.world.level = 15;
    game.flags
        .extend(["cityOfGold".into(), "city_of_gold".into()]);
    game.spawn("oExit", Vec2::new(32., 48.));
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Exit);
    assert_eq!(game.data["next_level"], 16.);
    assert!(game.flags.contains("cityOfGold"));
    assert!(game.flags.contains("city_of_gold"));
    for _ in 0..100 {
        game.step(&InputFrame::default());
        if game.ending.as_ref().unwrap().stage != EndingStage::Exit {
            break;
        }
        assert!(game.flags.contains("cityOfGold"));
        assert!(game.flags.contains("city_of_gold"));
    }
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);
    assert_eq!(game.world.level, 15);
    assert!(!game.flags.contains("cityOfGold"));
    assert!(!game.flags.contains("city_of_gold"));
}

#[test]
fn city_clear_requires_start_exit_and_logical_level_16() {
    for (kind, next) in [
        (EndingKind::Campaign, Some(16.)),
        (EndingKind::Transition, Some(15.)),
        (EndingKind::Transition, None),
    ] {
        let mut game = temple_door_room();
        game.world.level = 15;
        game.flags
            .extend(["cityOfGold".into(), "city_of_gold".into()]);
        if let Some(next) = next {
            game.data.insert("next_level".into(), next);
        }
        ending::begin_exit(&mut game, kind);
        for _ in 0..100 {
            game.step(&InputFrame::default());
            if game.ending.as_ref().unwrap().stage != EndingStage::Exit {
                break;
            }
        }
        assert_ne!(game.ending.as_ref().unwrap().stage, EndingStage::Exit);
        assert!(game.flags.contains("cityOfGold"));
        assert!(game.flags.contains("city_of_gold"));
    }
}
