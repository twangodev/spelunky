//! Configured source own-Step contexts; not an ordinary-input campaign claim.
//! Source emitters assign sprite_index, while Entity.facing is port bookkeeping.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 20, Area::Mines, 1);
    game.world.name = "rMoon".into();
    game.flags.insert("hub".into());
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.player.set_sprite("sAttackLeft");
    game.player.animation = 3.0;
    game.player.attack_timer = 10;
    game.world.entrance = game.player.pos;
    game
}

#[test]
fn sprite_owned_front_and_rear_hitboxes_ignore_default_facing_and_ground_jar_uses_real_side() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, sprite, offset) in [
            ("oWhip", "sWhipLeft", -16.0),
            ("oWhip", "sWhipRight", 16.0),
            ("oWhipPre", "sWhipPreL", 16.0),
            ("oWhipPre", "sWhipPreR", -16.0),
        ] {
            let mut game = room(rules);
            let whip = game.spawn(object, Vec2::new(240.0, 100.0));
            // Mirrors the original Player producer's explicit sprite assignment.
            // Do not also assign a facing variable the original object never reads.
            game.world.entity_mut(whip).unwrap().set_sprite(sprite);
            assert_eq!(game.world.entity(whip).unwrap().facing, 1);
            let jar = if object == "oWhip" {
                let jar = game.spawn("oJar", Vec2::new(104.0 + offset, 154.0));
                let mut source_pose = game.world.entity(whip).unwrap().clone();
                source_pose.pos = Vec2::new(104.0 + offset, 152.0);
                assert!(
                    source_pose.event_intersects(game.world.entity(jar).unwrap()),
                    "SOURCE_FRAME_ZERO_REAL_JAR_MASK prerequisite {sprite}"
                );
                Some(jar)
            } else {
                None
            };
            game.step(&InputFrame::default());
            let target = game
                .world
                .entity(whip)
                .expect("whipping remains active before AnimationEnd");
            assert_eq!(
                target.pos,
                Vec2::new(game.player.pos.x + offset, game.player.pos.y),
                "SOURCE_SPRITE_OFFSET {rules:?} {object} {sprite}"
            );
            assert_eq!(target.sprite, sprite);
            assert!(game.player.attack_timer > 0);
            assert_eq!(game.player.health, 4);
            if let Some(jar) = jar {
                assert!(
                    game.world.entity(jar).is_none(),
                    "source-side ground Jar receives actual Whip collision"
                );
            }
        }
    }
}

#[test]
fn unrelated_sprite_keeps_position_because_neither_source_alignment_branch_matches() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oWhip", "oWhipPre"] {
            let mut game = room(rules);
            let whip = game.spawn(object, Vec2::new(240.0, 100.0));
            game.world
                .entity_mut(whip)
                .unwrap()
                .set_sprite("sBasketball");
            game.step(&InputFrame::default());
            let target = game.world.entity(whip).unwrap();
            assert_eq!(target.pos, Vec2::new(240.0, 100.0));
            assert_eq!(target.sprite, "sBasketball");
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn inactive_player_is_not_an_instance_exists_owner_for_an_active_whip_event() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oWhip", "oWhipPre"] {
            let mut game = room(rules);
            game.player.engine_active = false;
            let whip = game.spawn(object, Vec2::new(240.0, 100.0));
            assert!(game.world.entity(whip).unwrap().engine_active);
            game.step(&InputFrame::default());
            assert!(!game.player.engine_active);
            assert!(
                game.world.entity(whip).is_none(),
                "source instance_exists owner gate retires hitbox"
            );
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn ordinary_attack_producer_keeps_both_directions_source_consistent() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let mut game = room(rules);
            // All overrides are initial setup; subsequent frames are real inputs.
            game.player.set_sprite("sStandLeft");
            game.player.animation = 0.0;
            game.player.attack_timer = 0;
            game.player.facing = facing;
            let mut forward = false;
            let mut rear = false;
            for tick in 0..18 {
                game.step(&InputFrame {
                    attack: tick == 0,
                    attack_pressed: tick == 0,
                    attack_released: tick == 1,
                    ..Default::default()
                });
                for entity in game.world.entities.iter().filter(|entity| {
                    entity.alive && matches!(entity.object.as_str(), "oWhip" | "oWhipPre")
                }) {
                    let offset = match entity.sprite.as_str() {
                        "sWhipLeft" | "sWhipPreR" => -16.0,
                        "sWhipRight" | "sWhipPreL" => 16.0,
                        other => panic!("stock producer unexpected art {other}"),
                    };
                    assert_eq!(
                        entity.pos,
                        Vec2::new(game.player.pos.x + offset, game.player.pos.y)
                    );
                    assert_eq!(
                        entity.facing, facing,
                        "existing producer consistently writes bookkeeping"
                    );
                    forward |= entity.object == "oWhip";
                    rear |= entity.object == "oWhipPre";
                }
                assert_eq!(game.player.health, 4);
            }
            assert!(
                forward && rear,
                "ordinary attack must produce both source events"
            );
        }
    }
}

#[test]
fn inactive_controls_do_not_remove_an_engine_active_whip_owner() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oWhip", "oWhipPre"] {
            let mut game = room(rules);
            game.player.data.insert("active".into(), 0.0);
            let whip = game.spawn(object, Vec2::new(240.0, 100.0));
            assert!(game.player.engine_active);
            game.step(&InputFrame::default());
            assert!(game.player.engine_active);
            assert!(
                game.world.entity(whip).is_some(),
                "input inactive is not instance_exists=false"
            );
            assert_eq!(game.player.health, 4);
        }
    }
}
