//! Source FlareCrate/FlareSpark point queries and the matching Flare caller.
//! Controlled incoming scenes: all actor, mask and activation setup precedes
//! Game::step. The authored hub has no oLevel to reactivate inactive controls.
//! Native collision_point flags use >=0.5: source -1/-1 and 0/0 are bbox mode.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(37, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rTitle".into();
    game.flags.insert("hub".into());
    game.flags.insert("hub_title".into());
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.set_tile(6, 10, "oHardBlock");
    game
}

fn sound(game: &Game, expected: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == expected))
        .count()
}

#[test]
fn flare_spark_default_create_uses_raw_cached_bbox_point_against_solids() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (target_y, point, active, empty, hits) in [
            (96.0, Vec2::new(104.0, 104.0), true, false, true),
            (96.25, Vec2::new(104.0, 96.1), true, false, true),
            (96.25, Vec2::new(104.0, 112.1), true, false, false),
            (96.0, Vec2::new(95.9995, 104.0), true, false, false),
            (96.0, Vec2::new(104.0, 104.0), false, false, false),
            (96.0, Vec2::new(104.0, 104.0), true, true, false),
        ] {
            let mut game = room(rules);
            let solid = game.spawn("oHardBlock", Vec2::new(96.0, target_y));
            let target = game.world.entity_mut(solid).unwrap();
            target.engine_active = active;
            if empty {
                target.sprite.clear();
            }
            assert_eq!(
                target.mask_contains(point, CollisionPrecision::BoundingBox) && active,
                hits,
                "FIXTURE: raw point and cached mask, including empty/inactive controls"
            );
            let spark = game.spawn("oFlareSpark", Vec2::new(point.x, point.y + 0.1));
            assert_eq!(game.world.entity(spark).unwrap().vel.y, -0.1);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(spark).is_none(), hits);
            if !hits {
                let spark = game.world.entity(spark).unwrap();
                assert!((spark.pos.y - point.y).abs() < 1e-12);
                assert_eq!(spark.pos.x, point.x);
            }
        }
    }
}

#[test]
fn flare_spark_point_honors_tile_mask_overrides_and_unrounded_left_edge() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (sprite, point, hits) in [
            ("sBrick", Vec2::new(95.9995, 104.0), false),
            ("sRock", Vec2::new(94.0, 94.0), true),
            ("", Vec2::new(104.0, 104.0), false),
        ] {
            let mut game = room(rules);
            game.world.set_tile(6, 6, "oHardBlock");
            let metadata = game.spawn("oTerrainMetadata", Vec2::new(104.0, 104.0));
            game.world
                .entity_mut(metadata)
                .unwrap()
                .text
                .insert("terrain_sprite".into(), sprite.into());
            assert_eq!(
                game.world
                    .tile_collision_mask(6, 6)
                    .contains(point, CollisionPrecision::BoundingBox),
                hits
            );
            let spark = game.spawn("oFlareSpark", Vec2::new(point.x, point.y + 0.1));
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(spark).is_none(), hits);
        }
    }
}

#[test]
fn flare_crate_default_create_uses_water_mask_family_instead_of_storage_cell() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (sprite, point, active, hits) in [
            ("sWater", Vec2::new(104.0, 104.0), true, true),
            ("sRock", Vec2::new(94.0, 94.0), true, true),
            ("sRock", Vec2::new(104.0, 104.0), true, false),
            ("", Vec2::new(104.0, 104.0), true, false),
            ("sWater", Vec2::new(104.0, 104.0), false, false),
        ] {
            let mut game = room(rules);
            game.world.set_tile(6, 6, "oWaterSwim");
            game.world.tiles[6 * game.world.width + 6].engine_active = active;
            let metadata = game.spawn("oTerrainMetadata", Vec2::new(104.0, 104.0));
            let meta = game.world.entity_mut(metadata).unwrap();
            meta.text.insert("terrain_sprite".into(), sprite.into());
            meta.engine_active = active;
            assert_eq!(
                game.world
                    .tile_collision_mask(6, 6)
                    .contains(point, CollisionPrecision::BoundingBox)
                    && active,
                hits
            );
            let receiver = game.spawn("oFlareCrate", point);
            assert_eq!(game.world.entity(receiver).unwrap().vel, Vec2::default());
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(receiver).is_none(), hits);
            assert_eq!(sound(&game, "sndSplash"), usize::from(hits));
            if hits {
                let splash = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == "oSplash")
                    .unwrap();
                let poof = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == "oPoof")
                    .unwrap();
                assert!(splash.creation_serial < poof.creation_serial);
            }
        }
    }
}

#[test]
fn flare_and_flare_crate_negative_flags_use_bbox_water_and_held_receiver() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oFlare", "oFlareCrate"] {
            for (held, active, empty, hits) in [
                (false, true, false, true),
                (true, true, false, true),
                (true, false, false, false),
                (true, true, true, false),
            ] {
                let mut game = room(rules);
                let point = if held {
                    Vec2::new(
                        108.0,
                        if object == "oFlareCrate" {
                            148.0
                        } else {
                            154.0
                        },
                    )
                } else {
                    Vec2::new(96.0, 96.0)
                };
                let water = game.spawn("oWaterSwim", point);
                let water = game.world.entity_mut(water).unwrap();
                water.engine_active = active;
                water.sprite = if empty { "" } else { "sDarkFall" }.into();
                if !empty {
                    assert!(water.mask_contains(point, CollisionPrecision::BoundingBox));
                    assert!(!water.mask_contains(point, CollisionPrecision::Precise));
                }
                // The source receiver's held flag, not hand identity, controls
                // unconditional clearing. The other live hand stays available
                // through its own earlier resource Step in this incoming scene.
                let other = game.spawn("oRock", game.player.pos);
                game.world.entity_mut(other).unwrap().held = true;
                game.player.held = Some(other);
                game.player.pickup_item_type = "Rock".into();
                let receiver = game.spawn(object, point);
                game.world.entity_mut(receiver).unwrap().held = held;
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(receiver).is_none(), hits);
                assert_eq!(sound(&game, "sndSplash"), usize::from(hits));
                assert_eq!(
                    game.player.held,
                    if held && hits { None } else { Some(other) }
                );
                assert_eq!(game.player.pickup_item_type.is_empty(), held && hits);
                assert!(game.world.entity(other).is_some());
            }
        }
    }
}

#[test]
fn default_dynamic_water_create_uses_cached_bbox_to_extinguish_flare_crate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (water_y, point, hits) in [
            (96.0, Vec2::new(104.0, 104.0), true),
            (96.25, Vec2::new(104.0, 96.1), true),
            (96.25, Vec2::new(104.0, 112.1), false),
        ] {
            let mut game = room(rules);
            let water = game.spawn("oWaterSwim", Vec2::new(96.0, water_y));
            assert_eq!(game.world.entity(water).unwrap().sprite, "sWater");
            assert_eq!(
                game.world
                    .entity(water)
                    .unwrap()
                    .mask_contains(point, CollisionPrecision::BoundingBox),
                hits
            );
            let receiver = game.spawn("oFlareCrate", point);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(receiver).is_none(), hits);
            assert_eq!(sound(&game, "sndSplash"), usize::from(hits));
            assert_eq!(
                game.world.entities.iter().any(|e| e.object == "oPoof"),
                hits
            );
        }
    }
}
