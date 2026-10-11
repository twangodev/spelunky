use super::*;

#[test]
fn destroy_water_request_matches_source_event_inheritance_for_both_storage_forms() {
    // Source oSolid Destroy ends with checkWater=true. Spear traps are the
    // only non-inheriting solid Destroy events with their own unconditional
    // checkWater write. Other listed overrides deliberately do not write it.
    for (object, requests_water) in [
        ("oBrick", true),
        ("oLush", true),
        ("oDark", true),
        ("oTemple", true),
        ("oSpearTrapTop", true),
        ("oSpearTrapBottom", true),
        ("oDoor", false),
        ("oIce", false),
        ("oIceBlock", false),
        ("oBlock", false),
        ("oPushBlock", false),
        ("oArrowTrapLeft", false),
    ] {
        for tiles in [false, true] {
            for clean in ["none", "cleanDeath", "cleanSolids"] {
                for already_dirty in [false, true] {
                    let mut game = Game::shell(7, RulesPreset::Classic);
                    let pos = Vec2::new(80.0, 80.0);
                    let id = if tiles {
                        game.world.set_tile(5, 5, object);
                        game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0))
                    } else {
                        game.world.spawn(object, pos)
                    };
                    if clean == "cleanDeath" {
                        game.world
                            .entity_mut(id)
                            .unwrap()
                            .data
                            .insert("cleanDeath".into(), 1.0);
                    } else if clean == "cleanSolids" {
                        game.flags.insert("cleanSolids".into());
                    }
                    game.world.fluid_dirty = already_dirty;
                    let removed = if tiles {
                        destroy_terrain_self(&mut game, 5, 5)
                    } else {
                        destroy_solid_self(&mut game, id)
                    };
                    assert!(removed, "{object} tiles={tiles} clean={clean}");
                    assert_eq!(
                        game.world.fluid_dirty,
                        already_dirty || requests_water,
                        "{object} tiles={tiles} clean={clean}"
                    );
                }
            }
        }
    }
}

#[test]
fn natural_mattock_pickup_and_attack_breach_dynamic_wall_and_drain_pool() {
    for fluid in ["oWaterSwim", "oLava"] {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(20, 16, Area::Jungle, 5);
        game.flags.insert("custom".into());
        for x in 0..20 {
            game.world.set_tile(x, 6, "oBrick");
        }
        for x in 5..9 {
            game.world.spawn(fluid, Vec2::new(x as f64 * 16.0, 80.0));
        }
        game.world.set_tile(9, 5, "oBrick");
        let wall = game.world.spawn("oBrick", Vec2::new(64.0, 80.0));
        game.player.pos = Vec2::new(56.0, 88.0);
        game.player.previous_pos = game.player.pos;
        game.player.facing = 1;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        let mattock = game.spawn("oMattock", game.player.pos);
        game.world.fluid_dirty = false;
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(
            game.player.held,
            Some(mattock),
            "ordinary Down+Attack pickup"
        );
        game.step(&InputFrame::default());
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..20 {
            if game.world.entity(wall).is_none() {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert!(
            game.world.entity(wall).is_none(),
            "ordinary swing must remove dynamic wall"
        );
        assert!(
            game.world.fluid_dirty,
            "dynamic inherited Destroy must request water"
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == fluid)
                .count(),
            4,
            "Game318 ran before this frame's Animation End destruction"
        );
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == fluid),
            "{fluid}"
        );
    }
}
