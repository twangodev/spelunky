//! Source Item Create and held-branch controls with initial authored links.
//! Dangling-hand cases are controlled initial contexts, not campaign routes.
use spelunky_core::{
    Area, Game, GenerationRevision, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(23, rules);
    game.generation_revision = GenerationRevision::SourceClassicV2;
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game
}

#[test]
fn inherited_create_depth_is_visible_before_first_step_without_rng_draws() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for spectacles in [false, true] {
            let mut game = room(rules);
            if spectacles {
                game.player.equipment.insert("Spectacles".into());
            }
            let before = game.rng.state_bits();
            for object in ["oRock", "oJar", "oSkull"] {
                let id = game.spawn(object, Vec2::new(160.0, 100.0));
                let item = game.world.entity(id).unwrap();
                assert_eq!(
                    item.value("depth"),
                    if spectacles { 51.0 } else { 101.0 },
                    "SOURCE_CREATE_DEPTH {rules:?} {object}"
                );
                assert_eq!(item.pos, Vec2::new(160.0, 100.0));
                assert_eq!(item.vel, Vec2::default());
                assert!(!item.held);
                assert_eq!(
                    game.rng.state_bits(),
                    before,
                    "inherited and own Creates are RNG-free"
                );
            }
        }
    }
}

#[test]
fn empty_hand_releases_parent_and_dice_after_attachment_without_unheld_motion() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oRock", "oDice", "oArrow", "oFlare"] {
            let mut game = room(rules);
            let id = game.spawn(object, Vec2::new(240.0, 100.0));
            let item = game.world.entity_mut(id).unwrap();
            item.held = true;
            item.vel = Vec2::new(4.0, 3.0);
            assert_eq!(game.player.held, None);
            let mut source_rng = game.rng.clone();
            let rolled_value = if object == "oDice" {
                Some(1 + source_rng.range(6))
            } else {
                None
            };
            game.step(&InputFrame::default());
            let item = game.world.entity(id).expect("released receiver survives");
            assert!(!item.held, "SOURCE_EMPTY_HAND {rules:?} {object}");
            if let Some(value) = rolled_value {
                assert_eq!(
                    item.vel,
                    Vec2::new(4.0, 3.0),
                    "Dice own held branch preserves velocity without unheld gravity"
                );
                assert_eq!(item.sprite, "sDiceRoll");
                assert_eq!(item.value("value"), f64::from(value));
                assert_eq!(
                    game.rng.state_bits(),
                    source_rng.state_bits(),
                    "independent Dice rolling tail consumes one draw"
                );
            } else {
                assert_eq!(
                    item.vel,
                    Vec2::default(),
                    "Item held branch zeros velocity and skips unheld gravity"
                );
            }
            assert_eq!(
                item.pos.x,
                game.player.pos.x + f64::from(game.player.facing) * 4.0
            );
            assert_eq!(
                item.pos.y,
                game.player.pos.y + if object == "oDice" { -2.0 } else { 2.0 }
            );
            assert_eq!(
                item.value("depth"),
                1.0,
                "held branch retains this Step's depth"
            );
            assert_eq!(game.player.health, 4.0);
        }
    }
}

#[test]
fn linked_parent_and_replacement_fragile_held_branches_remain_held() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oRock", "oDice", "oJar", "oSkull"] {
            for linked in [false, true] {
                if !linked && matches!(object, "oRock" | "oDice") {
                    continue;
                }
                let mut game = room(rules);
                let id = game.spawn(object, Vec2::new(104.0, 152.0));
                game.world.entity_mut(id).unwrap().held = true;
                if linked {
                    game.player.held = Some(id);
                }
                game.step(&InputFrame::default());
                let item = game.world.entity(id).unwrap();
                assert!(
                    item.held,
                    "SOURCE_HELD_CONTROL {rules:?} {object} linked={linked}"
                );
                assert_ne!(game.player.state, PlayerState::Climbing);
                // Player End Step overrides linked held-item depth after
                // the item's own Step; orphan Jar/Skull keep their depth.
                assert_eq!(item.value("depth"), if linked { 0.0 } else { 1.0 });
                assert_eq!(game.player.held, linked.then_some(id));
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}
