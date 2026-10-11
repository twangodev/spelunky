//! Source container point gates, mask receivers, and registration order via ordinary input.
use spelunky_core::{
    CollisionPrecision, Entity, Game, InputFrame, PlayerState, RulesPreset, Vec2, catalog,
    dispatcher::{self, Instance},
    level::Level,
};

fn floor_game(rules: RulesPreset, crouch: bool) -> Game {
    let mut level = Level::default();
    level.set_tile(5, 6, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 7, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "container-queries", "");
    for _ in 0..8 {
        game.step(&InputFrame {
            down: crouch,
            ..Default::default()
        });
    }
    assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
    assert!(game.player.grounded);
    assert!(!game.player.is_stunned());
    if crouch {
        assert_eq!(game.player.state, PlayerState::Ducking);
    }
    game
}

fn open_input(crouch: bool) -> InputFrame {
    InputFrame {
        up: true,
        down: crouch,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}

fn family_order(game: &Game, family: &str) -> Vec<u64> {
    dispatcher::descendant_instances(game, family)
        .into_iter()
        .filter_map(|instance| match instance {
            Instance::Entity(id) if game.world.entity(id).is_some() => Some(id),
            _ => None,
        })
        .collect()
}

fn assert_pair_for_query_sprite(game: &Game, entity: &Entity, crouch: bool) {
    let sprite = if crouch { "sDuckLeft" } else { "sLookLeft" };
    let player_mask = catalog::instance_mask(
        "oPlayer1",
        sprite,
        game.player.pos,
        0.0,
        Vec2::new(
            game.player.data.get("image_xscale").copied().unwrap_or(1.0),
            game.player.data.get("image_yscale").copied().unwrap_or(1.0),
        ),
        0.0,
    );
    let entity_mask = catalog::instance_mask(
        &entity.object,
        &entity.sprite,
        entity.pos,
        entity.animation,
        Vec2::new(1.0, 1.0),
        0.0,
    );
    assert!(player_mask.intersects_mask(&entity_mask));
}

#[test]
fn actual_step_container_point_gate_can_select_an_earlier_full_mask_receiver() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for crouch in [false, true] {
            for family in ["oChest", "oCrate", "oFlareCrate"] {
                let mut game = floor_game(rules, crouch);
                // Controlled arriving world items; player position, sprite and
                // movement state have come solely from the real room/input path.
                let earlier = game.spawn(family, Vec2::new(96.0, 120.0));
                let later = game.spawn(family, Vec2::new(108.0, 120.0));
                assert_eq!(family_order(&game, family), vec![earlier, later]);
                let first = game.world.entity(earlier).unwrap();
                assert!(!first.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
                assert_pair_for_query_sprite(&game, first, crouch);
                assert!(
                    game.world
                        .entity(later)
                        .unwrap()
                        .mask_contains(game.player.pos, CollisionPrecision::BoundingBox)
                );

                game.step(&open_input(crouch));

                assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
                assert_eq!(game.player.health, 4);
                assert_eq!(game.player.attack_timer, 0, "opening consumes Attack");
                if family == "oChest" {
                    assert_eq!(game.world.entity(earlier).unwrap().sprite, "sChestOpen");
                    assert_eq!(game.world.entity(later).unwrap().sprite, "sChest");
                    assert_eq!(game.data.get("totalChests"), None, "custom level guard");
                } else {
                    assert!(game.world.entity(earlier).is_none());
                    assert!(game.world.entity(later).is_some());
                }
            }
        }
    }
}

#[test]
fn actual_step_full_mask_contact_alone_does_not_open_a_container() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for family in ["oChest", "oCrate", "oFlareCrate"] {
            let mut game = floor_game(rules, true);
            let item = game.spawn(family, Vec2::new(96.0, 120.0));
            let source = game.world.entity(item).unwrap();
            assert!(!source.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
            assert_pair_for_query_sprite(&game, source, true);

            // Down keeps the unconsumed edge on the legitimate pickup path,
            // avoiding a later whip collision obscuring the container gate.
            game.step(&open_input(true));

            let item = game.world.entity(item).unwrap();
            assert_eq!(item.state, 0);
            assert_ne!(item.sprite, "sChestOpen");
            assert_eq!(game.data.get("totalChests"), None);
            assert_eq!(game.data.get("totalCrates"), None);
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn actual_step_open_chest_receiver_leaves_attack_for_the_crate_branch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = floor_game(rules, false);
        let earlier = game.spawn("oChest", Vec2::new(96.0, 120.0));
        let later = game.spawn("oChest", Vec2::new(108.0, 120.0));
        game.step(&open_input(false));
        assert_eq!(game.world.entity(earlier).unwrap().sprite, "sChestOpen");
        assert_eq!(game.world.entity(later).unwrap().sprite, "sChest");
        game.step(&InputFrame::default());
        let crate_id = game.spawn("oCrate", game.player.pos);

        game.step(&open_input(false));

        assert!(game.world.entity(crate_id).is_none());
        assert_eq!(game.world.entity(later).unwrap().sprite, "sChest");
        assert_eq!(game.player.attack_timer, 0);
        assert_eq!(game.player.health, 4);
    }
}

#[test]
fn actual_step_crate_branch_precedes_older_flare_crate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for crouch in [false, true] {
            let mut game = floor_game(rules, crouch);
            let flare = game.spawn("oFlareCrate", game.player.pos);
            let crate_id = game.spawn("oCrate", game.player.pos);
            assert!(flare < crate_id);

            game.step(&open_input(crouch));

            assert!(game.world.entity(crate_id).is_none());
            assert!(game.world.entity(flare).is_some());
            assert_eq!(game.player.attack_timer, 0);
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn actual_step_container_receiver_follows_natural_reactivation_order() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for family in ["oChest", "oCrate", "oFlareCrate"] {
            let mut level = Level::default();
            level.set_tile(3, 18, '@').unwrap();
            for x in 0..40 {
                level.set_tile(x, 19, '1').unwrap();
            }
            let mut game = Game::shell(42, rules);
            game.load_custom(&level, "container-reactivation", "");
            // The importer adds its hard-border tile, then Entrance adds8px:
            // authored(3,18) -> Player(72,312), floor row19 -> worldY320.
            assert_eq!(game.player.pos, Vec2::new(72.0, 312.0));
            let older = game.spawn(family, Vec2::new(78.0, 312.0));
            let newer = game.spawn(family, Vec2::new(82.0, 312.0));
            assert_eq!(family_order(&game, family), vec![older, newer]);
            // Initialize both inherited Item depths together while visible.
            // Fresh offscreen objects retain catalog depth900; entering the
            // narrower Item view at different times changes depth to101 and
            // Draw sorting can otherwise erase the activation-order witness.
            for _ in 0..8 {
                game.step(&InputFrame::default());
            }
            assert!(game.player.grounded);
            assert_eq!(game.player.pos, Vec2::new(72.0, 312.0));
            assert_eq!(game.world.entity(older).unwrap().value("depth"), 101.0);
            assert_eq!(game.world.entity(newer).unwrap().value("depth"), 101.0);
            assert_eq!(family_order(&game, family), vec![older, newer]);
            let mut saw_inactive = false;
            for _ in 0..260 {
                if game.player.pos.x >= 493.0 {
                    break;
                }
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                saw_inactive |= !game.world.entity(older).unwrap().engine_active
                    && !game.world.entity(newer).unwrap().engine_active;
            }
            assert!(
                game.player.pos.x >= 493.0,
                "ordinary outbound walk must finish"
            );
            for _ in 0..8 {
                game.step(&InputFrame::default());
                saw_inactive |= !game.world.entity(older).unwrap().engine_active
                    && !game.world.entity(newer).unwrap().engine_active;
            }
            assert!(saw_inactive);
            assert!(!game.world.entity(older).unwrap().engine_active);
            assert!(!game.world.entity(newer).unwrap().engine_active);
            assert_eq!(game.world.entity(older).unwrap().value("depth"), 101.0);
            assert_eq!(game.world.entity(newer).unwrap().value("depth"), 101.0);
            let mut saw_newer_only = false;
            let mut opened = false;
            for _ in 0..260 {
                let first = game.world.entity(older).unwrap();
                let second = game.world.entity(newer).unwrap();
                saw_inactive |= !first.engine_active && !second.engine_active;
                saw_newer_only |= !first.engine_active && second.engine_active;
                let ready = game.player.grounded
                    && game.player.pos.y == 312.0
                    && (79.0..=81.0).contains(&game.player.pos.x);
                if ready {
                    assert!(saw_inactive);
                    assert!(
                        saw_newer_only,
                        "return walk must reactivate newer on a distinct frame"
                    );
                    assert_eq!(first.value("depth"), 101.0);
                    assert_eq!(second.value("depth"), 101.0);
                    assert_eq!(family_order(&game, family), vec![newer, older]);
                    assert!(first.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
                    assert!(second.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
                    let storage: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|entity| entity.object == family)
                        .map(|entity| entity.id)
                        .collect();
                    assert_eq!(storage, vec![older, newer]);

                    game.step(&open_input(false));

                    // Keep the point-gate premise explicit after player motion;
                    // the expected result must not depend on leaving one item.
                    let first = game.world.entity(older).unwrap();
                    assert!(first.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
                    if family == "oChest" {
                        assert_eq!(first.sprite, "sChest");
                        assert_eq!(game.world.entity(newer).unwrap().sprite, "sChestOpen");
                    } else {
                        assert!(game.world.entity(newer).is_none());
                    }
                    assert_eq!(game.player.health, 4);
                    assert_eq!(game.player.attack_timer, 0);
                    opened = true;
                    break;
                }
                let before = game.player.pos;
                game.step(&InputFrame {
                    left: before.x > 80.0,
                    right: before.x < 80.0,
                    ..Default::default()
                });
            }
            assert!(
                opened,
                "natural arrival must reach the overlapping {family} pair"
            );
        }
    }
}
