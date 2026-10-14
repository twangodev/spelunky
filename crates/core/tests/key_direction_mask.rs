//! Source oKey Step and locked-chest Collision regressions.
//! All room/actor setup precedes the first Game::step; later writes are inputs.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World, catalog};

fn room(rules: RulesPreset) -> (Game, u64) {
    let mut game = Game::shell(23, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    // Support the player without settling the neighboring chest onto a floor.
    game.world.set_tile(6, 10, "oHardBlock");
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    let key = game.spawn("oKey", game.player.pos);
    (game, key)
}

fn pickup(left: bool) -> InputFrame {
    InputFrame {
        left,
        right: !left,
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}

#[test]
fn ordinary_key_pickup_turn_and_throw_preserve_source_directional_sprites() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, key) = room(rules);
        assert_eq!(game.world.entity(key).unwrap().sprite, "sKeyRight");
        game.step(&pickup(true));
        assert_eq!(game.player.facing, -1, "ordinary Left input turns player");
        assert_eq!(game.player.held, Some(key), "ordinary Down+Attack pickup");
        assert_eq!(game.world.entity(key).unwrap().sprite, "sKeyLeft");
        game.step(&InputFrame {
            right: true,
            down: true,
            attack_released: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(key));
        assert_eq!(game.world.entity(key).unwrap().sprite, "sKeyRight");
        game.step(&InputFrame {
            left: true,
            ..Default::default()
        });
        assert_eq!(game.world.entity(key).unwrap().sprite, "sKeyLeft");
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, None, "ordinary Attack throws held key");
        let key = game.world.entity(key).expect("thrown key survives");
        assert!(!key.held);
        assert!(key.vel.x < 0.0);
        assert_eq!(key.sprite, "sKeyLeft", "own Step changes only held keys");
    }
}

#[test]
fn ordinary_pickup_uses_directional_precise_key_mask_to_unlock_chest() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Left's origin is (8,6), Right's is (8,8). These literal source mask
        // contacts distinguish the two sprites and a separated control.
        for (left, dx, dy, left_overlap, right_overlap) in [
            (true, -12.0, 3.0, true, false),
            (true, -12.0, -9.0, false, true),
            (false, -12.0, -9.0, false, true),
            (true, -14.0, 3.0, false, false),
        ] {
            let (mut game, key) = room(rules);
            let held_pos = Vec2::new(if left { 100.0 } else { 108.0 }, 156.0);
            let chest_pos = Vec2::new(held_pos.x + dx, held_pos.y + dy);
            let chest = game.spawn("oLockedChest", chest_pos);
            let unit = Vec2::new(1.0, 1.0);
            let chest_mask =
                catalog::instance_mask("oLockedChest", "sLockedChest", chest_pos, 0.0, unit, 0.0);
            for (sprite, expected) in [("sKeyLeft", left_overlap), ("sKeyRight", right_overlap)] {
                let mask = catalog::instance_mask("oKey", sprite, held_pos, 0.0, unit, 0.0);
                assert_eq!(
                    mask.intersects_mask(&chest_mask),
                    expected,
                    "FIXTURE: literal {sprite} mask at chest offset ({dx},{dy})"
                );
            }
            let opens = if left { left_overlap } else { right_overlap };
            game.step(&pickup(left));
            assert_eq!(game.player.facing, if left { -1 } else { 1 });
            assert_eq!(game.world.entity(chest).is_none(), opens);
            assert_eq!(game.world.entity(key).is_none(), opens);
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oUdjatEye")
                    .count(),
                usize::from(opens),
                "source collision grants exactly one Eye only on precise overlap"
            );
            if opens {
                assert_eq!(game.player.held, None);
                assert!(game.player.pickup_item_type.is_empty());
            } else {
                assert_eq!(game.player.held, Some(key), "ordinary pickup must succeed");
                let key = game.world.entity(key).unwrap();
                assert!(key.held);
                assert_eq!(
                    key.pos, held_pos,
                    "inherited attachment precedes mask collision"
                );
                assert_eq!(key.sprite, if left { "sKeyLeft" } else { "sKeyRight" });
                let chest = game.world.entity(chest).unwrap();
                assert_eq!(
                    chest.pos, chest_pos,
                    "fixture chest retains its initial origin"
                );
                assert_eq!(chest.sprite, "sLockedChest");
            }
        }
    }
}
