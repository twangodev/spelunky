//! Natural Arrow-hit/pickup trajectories plus explicitly controlled initial
//! source-reachable body-pose controls. No post-start state or terrain mutation.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

fn pickup_room(rules: RulesPreset, water: bool) -> Game {
    let mut game = Game::shell(31, rules);
    game.world = World::new(24, 20, Area::Ice, 9);
    game.player.pos = Vec2::new(154.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.facing = 1;
    for x in 0..24 {
        game.world.set_tile(x, 10, "oBlock");
    }
    if water {
        game.world.set_tile(10, 9, "oWaterSwim");
    }
    game
}

fn body(game: &mut Game, object: &str, sprite: &str, dead: bool) -> u64 {
    let id = game.spawn(object, Vec2::new(150.0, 144.0));
    let actor = game.world.entity_mut(id).unwrap();
    // Controlled initial source state: an item-hit moving stunned body, or
    // a resting corpse. These assignments precede the first public Step.
    actor.state = if dead { 99 } else { 98 };
    actor.timer = 120;
    if dead {
        actor.health = 0;
    }
    actor.set_sprite(sprite);
    actor.vel = if dead {
        Vec2::default()
    } else {
        Vec2::new(1.0, -3.0)
    };
    id
}

fn pickup_at(game: &mut Game, id: u64, expected: Vec2) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..InputFrame::default()
    });
    assert_eq!(
        game.player.held,
        Some(id),
        "actual Down+Attack must pick up this enemy"
    );
    let actor = game.world.entity(id).unwrap();
    assert!(actor.held);
    assert_eq!(
        actor.pos, expected,
        "stationary ducking right-facing held placement"
    );
    assert_eq!(game.player.health, 4);
}

fn pickup(game: &mut Game, id: u64) {
    pickup_at(game, id, Vec2::new(150.0, 142.0));
}

fn splashes(game: &Game) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oSplash")
        .count()
}

#[test]
fn ordinary_pickup_preserves_incoming_sixteen_pixel_probe_then_uses_held_canvas() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, incoming, held) in [
            ("oYeti", "sYetiDieLL", "sYetiHeldL"),
            ("oHawkman", "sHawkDieLL", "sHawkHeldL"),
        ] {
            let mut game = pickup_room(rules, true);
            let id = body(&mut game, object, incoming, false);
            pickup(&mut game, id);
            let actor = game.world.entity(id).unwrap();
            assert_eq!(
                actor.value("swimming"),
                0.0,
                "{rules:?} {object}: first parent probes158,150 from incoming16px"
            );
            assert_eq!(
                actor.sprite, held,
                "child selects24px held image after the parent"
            );
            assert_eq!(splashes(&game), 0);
            game.step(&InputFrame {
                down: true,
                ..InputFrame::default()
            });
            let actor = game.world.entity(id).unwrap();
            assert_eq!(
                actor.value("swimming"),
                1.0,
                "next parent probes162,150 from previous Held24px"
            );
            assert_eq!(actor.sprite, held);
            assert_eq!(splashes(&game), 1);
        }
    }
}

#[test]
fn ordinary_pickup_of_already_twenty_four_pixel_stun_probes_water_immediately() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, incoming, held) in [
            ("oYeti", "sYetiStunL", "sYetiHeldL"),
            ("oHawkman", "sHawkStunL", "sHawkHeldL"),
        ] {
            let mut game = pickup_room(rules, true);
            let id = body(&mut game, object, incoming, false);
            // A stationary source STUNNED body has selected Stun24 already.
            game.world.entity_mut(id).unwrap().vel = Vec2::default();
            pickup(&mut game, id);
            assert_eq!(
                game.world.entity(id).unwrap().value("swimming"),
                1.0,
                "{rules:?} {object}"
            );
            assert_eq!(game.world.entity(id).unwrap().sprite, held);
            assert_eq!(
                splashes(&game),
                1,
                "no unconditional first-held16px workaround"
            );
        }
    }
}

#[test]
fn ordinary_dry_pickup_keeps_both_surviving_held_child_selectors() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, incoming, held) in [
            ("oYeti", "sYetiDieLL", "sYetiHeldL"),
            ("oHawkman", "sHawkDieLL", "sHawkHeldL"),
        ] {
            let mut game = pickup_room(rules, false);
            let id = body(&mut game, object, incoming, false);
            pickup(&mut game, id);
            assert_eq!(
                game.world.entity(id).unwrap().sprite,
                held,
                "{rules:?} {object}"
            );
            assert_eq!(game.world.entity(id).unwrap().value("swimming"), 0.0);
            game.step(&InputFrame {
                down: true,
                ..InputFrame::default()
            });
            assert_eq!(game.world.entity(id).unwrap().sprite, held);
            assert_eq!(game.world.entity(id).unwrap().value("swimming"), 0.0);
            assert_eq!(splashes(&game), 0);
        }
    }
}

#[test]
fn ordinary_dead_yeti_pickup_reaches_the_source_dead_held_epilogue() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = pickup_room(rules, true);
        let id = body(&mut game, "oYeti", "sYetiDeadL", true);
        pickup(&mut game, id);
        let actor = game.world.entity(id).unwrap();
        assert_eq!(actor.health, 0);
        assert_eq!(
            actor.sprite, "sYetiDHeldL",
            "{rules:?}: DEAD cannot return before final held assignment"
        );
        assert_eq!(actor.value("swimming"), 1.0, "incoming DeadL itself is24px");
        assert_eq!(splashes(&game), 1);
        game.step(&InputFrame {
            down: true,
            ..InputFrame::default()
        });
        assert_eq!(game.world.entity(id).unwrap().sprite, "sYetiDHeldL");
        assert_eq!(splashes(&game), 1);
    }
}

#[test]
fn ordinary_sixteen_pixel_body_families_keep_their_late_held_images() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, living, dead) in [
            ("oCaveman", "sCavemanHeldL", "sCavemanDHeldL"),
            ("oShopkeeper", "sShopHeldL", "sShopDHeldL"),
            ("oShopkeeper2", "sShopHeldL", "sShopDHeldL"),
        ] {
            for is_dead in [false, true] {
                let mut game = pickup_room(rules, true);
                let incoming = if object == "oCaveman" {
                    "sCavemanStunL"
                } else {
                    "sShopStunL"
                };
                let id = body(&mut game, object, incoming, is_dead);
                pickup(&mut game, id);
                let actor = game.world.entity(id).unwrap();
                assert_eq!(
                    actor.sprite,
                    if is_dead { dead } else { living },
                    "{rules:?} {object}"
                );
                assert_eq!(actor.value("swimming"), 0.0);
                assert_eq!(splashes(&game), 0);
            }
        }
    }
}

#[test]
fn natural_arrow_hit_then_ordinary_pickup_probes_the_previous_moving_body_image() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oYeti", "oHawkman"] {
            let mut game = pickup_room(rules, true);
            game.player.pos = Vec2::new(152.0, 152.0);
            game.player.previous_pos = game.player.pos;
            let id = game.spawn(object, Vec2::new(148.0, 144.0));
            let arrow = game.spawn("oArrow", Vec2::new(151.0, 146.0));
            // Source item impact requires speed>2, Player hurt requires vx>3.
            // Exactly3 permits a real enemy hit with no player-safe flags.
            game.world.entity_mut(arrow).unwrap().vel = Vec2::new(3.0, 0.0);
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(arrow).is_none(),
                "source arrow destroys itself on the hit"
            );
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.state, 98);
            assert_eq!(
                actor.sprite,
                if object == "oYeti" {
                    "sYetiDieLL"
                } else {
                    "sHawkDieLL"
                }
            );
            assert_eq!(game.player.health, 4);
            assert_eq!(actor.value("swimming"), 0.0);
            pickup_at(&mut game, id, Vec2::new(148.0, 142.0));
            let actor = game.world.entity(id).unwrap();
            assert_eq!(
                actor.value("swimming"),
                0.0,
                "{rules:?} {object}: incoming16px probes156,150"
            );
            assert_eq!(
                actor.sprite,
                if object == "oYeti" {
                    "sYetiHeldL"
                } else {
                    "sHawkHeldL"
                }
            );
            assert_eq!(splashes(&game), 0);
            game.step(&InputFrame {
                down: true,
                ..InputFrame::default()
            });
            assert_eq!(game.player.held, Some(id));
            assert_eq!(
                game.world.entity(id).unwrap().value("swimming"),
                1.0,
                "previous Held24 now probes160,150"
            );
            assert_eq!(splashes(&game), 1);
        }
    }
}

#[test]
fn natural_vertical_arrow_stun_then_pickup_uses_twenty_four_pixel_image_immediately() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oYeti", "oHawkman"] {
            let mut game = pickup_room(rules, true);
            game.player.pos = Vec2::new(152.0, 152.0);
            game.player.previous_pos = game.player.pos;
            let id = game.spawn(object, Vec2::new(148.0, 144.0));
            let arrow = game.spawn("oArrow", Vec2::new(156.0, 143.0));
            game.world.entity_mut(arrow).unwrap().vel = Vec2::new(0.0, 3.0);
            game.step(&InputFrame::default());
            assert!(game.world.entity(arrow).is_none());
            assert_eq!(
                game.world.entity(id).unwrap().sprite,
                if object == "oYeti" {
                    "sYetiStunL"
                } else {
                    "sHawkStunL"
                }
            );
            assert_eq!(game.player.health, 4);
            pickup_at(&mut game, id, Vec2::new(148.0, 142.0));
            assert_eq!(
                game.world.entity(id).unwrap().value("swimming"),
                1.0,
                "{rules:?} {object}: actual incomingStun24 must not defer fluid checks"
            );
            assert_eq!(splashes(&game), 1);
        }
    }
}
