//! Natural fatal falls distinguish rectangle eligibility from instance_place.
use spelunky_core::*;

// Mirror the public instance-mask constructor with the same stored transform
// properties used by Player/Entity; the internal convenience methods are private.
fn player_mask(player: &Player) -> catalog::CollisionMask {
    catalog::instance_mask(
        "oPlayer1",
        &player.sprite,
        player.pos,
        player.animation,
        Vec2::new(
            player.data.get("image_xscale").copied().unwrap_or(1.0),
            player.data.get("image_yscale").copied().unwrap_or(1.0),
        ),
        -player
            .data
            .get("image_angle")
            .copied()
            .unwrap_or(0.0)
            .to_radians(),
    )
}

fn spike_mask(spikes: &Entity) -> catalog::CollisionMask {
    catalog::instance_mask(
        &spikes.object,
        &spikes.sprite,
        spikes.pos,
        spikes.animation,
        Vec2::new(
            spikes.data.get("image_xscale").copied().unwrap_or(1.0),
            spikes.data.get("image_yscale").copied().unwrap_or(1.0),
        ),
        -spikes
            .data
            .get("image_angle")
            .copied()
            .unwrap_or(0.0)
            .to_radians(),
    )
}

#[test]
fn natural_spike_contact_marks_only_the_first_native_mask_target() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for layout in 0..3 {
            let mut game = Game::shell(7, rules);
            game.world = World::new(30, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..30 {
                game.world.set_tile(x, 7, "oHardBlock");
            }
            game.player.pos = Vec2::new(88.0, if layout == 2 { 40.0 } else { 36.0 });
            game.player.previous_pos = game.player.pos;
            let first = game.spawn(
                "oSpikes",
                Vec2::new(80.0, if layout == 2 { 100.0 } else { 96.0 }),
            );
            let second = if layout == 1 {
                Some(game.spawn("oSpikes", Vec2::new(80.0, 95.0)))
            } else {
                None
            };
            if layout == 2 {
                game.world.set_tile(5, 6, "oSpikes");
            }
            let mut reached = false;
            for _ in 0..30 {
                let mut prefix = game.clone();
                prefix.tick += 1;
                prefix.level_tick += 1;
                player::tick(&mut prefix, &InputFrame::default());
                game.step(&InputFrame::default());
                if !game.events.contains(&GameEvent::Damage { amount: 99 }) {
                    continue;
                }
                reached = true;
                let pos = prefix.player.pos;
                assert!(
                    Rect::new(pos.x - 4.0, pos.y - 4.0, 9.0, 13.0)
                        .intersects(prefix.world.entity(first).unwrap().sprite_bounds()),
                    "first instance passes the source hurt rectangle"
                );
                assert!(
                    !player_mask(&prefix.player)
                        .intersects_mask(&spike_mask(prefix.world.entity(first).unwrap())),
                    "first instance is outside actual Player mask"
                );
                assert_eq!(
                    game.world.entity(first).unwrap().sprite,
                    "sSpikes",
                    "a rectangle hit must not substitute for instance_place"
                );
                if let Some(second) = second {
                    assert!(
                        player_mask(&prefix.player)
                            .intersects_mask(&spike_mask(prefix.world.entity(second).unwrap()))
                    );
                    assert_eq!(game.world.entity(second).unwrap().sprite, "sSpikesBlood");
                }
                if layout == 2 {
                    assert!(
                        player_mask(&prefix.player)
                            .intersects_mask(&prefix.world.tile_collision_mask(5, 6))
                    );
                    assert!(
                        game.world.entities.iter().any(|e| e.alive
                            && e.object == "oTerrainMetadata"
                            && e.text
                                .get("terrain_sprite")
                                .is_some_and(|s| s == "sSpikesBlood")),
                        "selected tile adapter receives its sprite write"
                    );
                }
                assert_eq!(game.state, RunState::Dead);
                assert_eq!(game.player.health, 0);
                break;
            }
            assert!(
                reached,
                "natural fall must reach source spike damage, layout={layout}/{rules:?}"
            );
        }
    }
}
