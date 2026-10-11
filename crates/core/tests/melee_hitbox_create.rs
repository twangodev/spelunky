//! Stock synchronous Create contracts and ordinary attack producer controls.
//! Raw-spawn/reentry cases are explicit constructor tests, not input witnesses.
use spelunky_core::{Area, Entity, Game, InputFrame, RulesPreset, Vec2, World, characters, items};

fn fields(object: &str) -> (&str, f64, Option<f64>, i32, f64) {
    match object {
        "oWhip" => ("Whip", 1.0, Some(0.0), -1, 1.0),
        "oWhipPre" => ("Whip", 1.0, None, 3, 1.0),
        "oSlash" => ("Machete", 2.0, Some(1.0), -1, 1.0),
        "oMachetePre" => ("Machete", 2.0, None, 3, 1.0),
        "oMattockHit" => ("Mattock", 2.0, Some(1.0), -1, 0.5),
        "oMattockPre" => ("Mattock", 2.0, None, 3, 1.0),
        _ => panic!("not a source hitbox"),
    }
}
const HITBOXES: [&str; 6] = [
    "oWhip",
    "oWhipPre",
    "oSlash",
    "oMachetePre",
    "oMattockHit",
    "oMattockPre",
];

#[test]
fn stock_hitbox_creates_store_declared_fields_without_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for full_create in [false, true] {
            let mut game = Game::shell(79, rules);
            for object in HITBOXES {
                let before = game.rng.state_bits();
                let id = if full_create {
                    game.spawn(object, Vec2::new(64.0, 64.0))
                } else {
                    game.world.spawn(object, Vec2::new(64.0, 64.0))
                };
                assert_eq!(game.rng.state_bits(), before, "{object}: fixed Create");
                let entity = game.world.entity(id).unwrap();
                let (kind, damage, puncture, alarm, speed) = fields(object);
                assert_eq!(
                    entity.text.get("type").map(String::as_str),
                    Some(kind),
                    "{object}"
                );
                assert_eq!(entity.data.get("damage"), Some(&damage), "{object}");
                assert_eq!(entity.data.get("puncture").copied(), puncture, "{object}");
                assert_eq!(entity.data.get("blinkToggle"), Some(&0.0), "{object}");
                assert_eq!(entity.alarms[0], alarm, "{object}");
                assert_eq!(entity.image_speed(), speed, "{object}");
            }
        }
    }
}

#[test]
fn post_create_zero_negative_and_type_overrides_survive_initializer_reentry() {
    // These intentional post-Create caller writes exercise the once-only guard.
    // They make no claim about enemy callbacks' separate numeric coercion.
    for object in HITBOXES {
        for damage in [0.0, -3.0] {
            let mut game = Game::shell(79, RulesPreset::Classic);
            let id = game.spawn(object, Vec2::new(64.0, 64.0));
            let entity = game.world.entity_mut(id).unwrap();
            entity.data.insert("damage".into(), damage);
            entity.data.insert("puncture".into(), -1.0);
            entity.text.insert("type".into(), "caller override".into());
            entity.data.insert("blinkToggle".into(), 1.0);
            entity.alarms[0] = 9;
            entity.set_image_speed(0.25);
            items::weapons::init_entity(entity);
            items::init_entity(entity);
            assert_eq!(entity.data["damage"], damage, "{object}");
            assert_eq!(entity.data["puncture"], -1.0, "{object}");
            assert_eq!(entity.text["type"], "caller override", "{object}");
            assert_eq!(entity.data["blinkToggle"], 1.0, "{object}");
            assert_eq!(entity.alarms[0], 9, "{object}");
            assert_eq!(entity.image_speed(), 0.25, "{object}");
        }
    }
}

fn attack_room(
    rules: RulesPreset,
    facing: i8,
    weapon: Option<&str>,
    tunnel: bool,
) -> (Game, Option<u64>) {
    let mut game = Game::shell(79, rules);
    if tunnel {
        characters::apply(&mut game, characters::Character::TunnelMan);
    }
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(88.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.facing = facing;
    let held = weapon.map(|object| game.spawn(object, game.player.pos));
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "SETUP: settled floor");
    if let Some(id) = held {
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(id), "SETUP: ordinary weapon pickup");
        game.step(&InputFrame::default());
    }
    (game, held)
}

#[test]
fn ordinary_attack_producers_preserve_damage_puncture_and_resources() {
    // Existing ordinary producers already assign damage/front puncture.
    // Thus this is an unchanged-behavior control, not a promised old failure.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for (weapon, tunnel, pre, front) in [
                (None, false, "oWhipPre", "oWhip"),
                (Some("oMachete"), false, "oMachetePre", "oSlash"),
                (Some("oMattock"), false, "oMattockPre", "oMattockHit"),
                (None, true, "oMattockPre", "oMattockHit"),
            ] {
                let (mut game, held) = attack_room(rules, facing, weapon, tunnel);
                let resources = (game.player.health, game.player.bombs, game.player.ropes);
                let before_rng = game.rng.state_bits();
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert!(game.player.attack_timer > 0, "SETUP: attack onset");
                let mut saw_pre = false;
                let mut saw_front = false;
                for _ in 0..20 {
                    game.step(&InputFrame::default());
                    for entity in game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && (e.object == pre || e.object == front))
                    {
                        let (_, damage, puncture, _, _) = fields(&entity.object);
                        assert_eq!(entity.value("damage"), damage, "ordinary {}", entity.object);
                        assert_eq!(
                            entity.value("puncture"),
                            puncture.unwrap_or(0.0),
                            "ordinary {}",
                            entity.object
                        );
                        assert_eq!(entity.facing, facing);
                        saw_pre |= entity.object == pre;
                        saw_front |= entity.object == front;
                    }
                }
                assert!(
                    saw_pre && saw_front,
                    "SETUP: both ordinary attack parts ({weapon:?}/{tunnel}/{facing}/{rules:?})"
                );
                assert_eq!(
                    (game.player.health, game.player.bombs, game.player.ropes),
                    resources
                );
                assert_eq!(game.player.held, held);
                assert_eq!(
                    game.rng.state_bits(),
                    before_rng,
                    "hard floor: no dig/loot draws"
                );
                assert_eq!(game.world.tile(5, 10).unwrap().object, "oHardBlock");
            }
        }
    }
}

#[test]
fn unrelated_weapon_and_plain_entity_defaults_are_unchanged() {
    for object in ["oRock", "oMachete", "oMattock", "oPistol"] {
        let mut entity = Entity::new(1, object, Vec2::new(64.0, 64.0));
        items::init_entity(&mut entity);
        assert_eq!(entity.value("damage"), 1.0, "{object}: Item parent");
        assert!(!entity.data.contains_key("puncture"), "{object}");
        assert!(!entity.data.contains_key("blinkToggle"), "{object}");
        assert!(
            !entity.text.contains_key("type"),
            "{object}: existing source_type fallback"
        );
    }
}
