//! Public simulation witnesses for oEnemy's current-sprite fluid probes.
//! All terrain, births, and the Rock launch are authored before the first Step.
//! Enemy states/sprites are entered by real item impact or prey collision.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(17, rules);
    game.world = World::new(24, 20, Area::Jungle, 5);
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.previous_pos = game.player.pos;
    for x in 0..24 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game
}

fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}

fn stunned_by_rock(game: &mut Game, object: &str) -> u64 {
    let actor = game.spawn(object, Vec2::new(148.0, 64.0));
    let rock = game.spawn("oRock", Vec2::new(156.0, 60.0));
    game.world.entity_mut(rock).unwrap().vel = Vec2::new(0.0, 3.0);
    actor
}

fn assert_stun_entry(game: &Game, id: u64, object: &str) {
    let actor = game
        .world
        .entity(id)
        .expect("Rock hit must leave a living stunned actor");
    assert_eq!(actor.state, 98, "{object}: real oItem Step entered STUNNED");
    assert_eq!(
        actor.sprite,
        if object == "oYeti" {
            "sYetiStunL"
        } else {
            "sHawkStunL"
        }
    );
    assert_eq!(actor.pos, Vec2::new(148.0, 58.0));
    assert_eq!(
        count(game, "oBlood"),
        1,
        "only the source item-impact blood"
    );
}

#[test]
fn ordinary_sixteen_pixel_enemies_keep_the_original_fluid_edge() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oYeti", "oHawkman"] {
            for (water_x, wet) in [(9, true), (10, false)] {
                let mut game = room(rules);
                game.world.set_tile(water_x, 4, "oWaterSwim");
                let id = game.spawn(object, Vec2::new(148.0, 64.0));
                game.step(&InputFrame::default());
                let actor = game.world.entity(id).unwrap();
                // The incoming ordinary image is16px: its probe is156,72.
                assert_eq!(
                    actor.value("swimming"),
                    f64::from(wet),
                    "{rules:?} {object} water{water_x}"
                );
                assert_eq!(
                    actor.value("inherited_gravity"),
                    if wet { 0.2 } else { 0.6 }
                );
                assert_eq!(count(&game, "oSplash"), usize::from(wet));
            }
        }
    }
}

#[test]
fn natural_rock_stun_uses_twenty_four_pixel_canvas_on_the_following_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oYeti", "oHawkman"] {
            for (water_x, second_wet) in [(9, false), (10, true)] {
                let mut game = room(rules);
                game.world.set_tile(water_x, 4, "oWaterSwim");
                let id = stunned_by_rock(&mut game, object);
                game.step(&InputFrame::default());
                assert_stun_entry(&game, id, object);
                assert_eq!(
                    game.world.entity(id).unwrap().value("swimming"),
                    f64::from(!second_wet)
                );
                game.step(&InputFrame::default());
                let actor = game.world.entity(id).unwrap();
                // Stun origin4 is deliberately NOT subtracted: source samples
                // x+12=160, rather than visual/mask center x+8=156.
                assert_eq!(
                    actor.value("swimming"),
                    f64::from(second_wet),
                    "{rules:?} {object} water{water_x}"
                );
                assert_eq!(
                    actor.value("inherited_gravity"),
                    if second_wet { 0.2 } else { 0.6 }
                );
                assert_eq!(count(&game, "oSplash"), 1);
                assert_eq!(count(&game, "oBlood"), 1, "no second STUNNED damage blood");
            }
        }
    }
}

#[test]
fn natural_stun_canvas_width_controls_both_lava_probes() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oYeti", "oHawkman"] {
            for (lava_y, top) in [(3, true), (4, false)] {
                let mut game = room(rules);
                game.world.set_tile(10, lava_y, "oLava");
                let id = stunned_by_rock(&mut game, object);
                game.step(&InputFrame::default());
                assert_stun_entry(&game, id, object);
                game.step(&InputFrame::default());
                if top {
                    assert!(
                        game.world.entity(id).is_none(),
                        "{rules:?} {object}: top probe160,57 destroys"
                    );
                } else {
                    let actor = game
                        .world
                        .entity(id)
                        .expect("bottom lava enters the retained corpse state");
                    assert_eq!(
                        actor.health, 0,
                        "{rules:?} {object}: foot probe160,72 burns"
                    );
                    assert_eq!(actor.value("counts_as_kill"), 0.0);
                    assert_eq!(actor.value("burning"), 1.0);
                    assert_eq!(actor.value("inherited_gravity"), 0.0);
                }
                assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
                assert_eq!(
                    count(&game, "oBlood"),
                    1,
                    "lava does not repeat item-hit blood"
                );
            }
        }
    }
}

fn meal(game: &mut Game) -> u64 {
    let trap = game.spawn("oManTrap", Vec2::new(142.0, 80.0));
    let prey = game.spawn("oCaveman", Vec2::new(146.0, 80.0));
    game.step(&InputFrame::default());
    assert!(
        game.world.entity(prey).is_none(),
        "ordinary collision must actually eat the prey"
    );
    let actor = game.world.entity(trap).unwrap();
    assert_eq!(actor.state, 97);
    assert_eq!(actor.sprite, "sManTrapEatCavemanL");
    assert_eq!(actor.pos.y, 80.0);
    assert!((144.0..=145.0).contains(&actor.pos.x));
    assert_eq!(
        actor.value("swimming"),
        0.0,
        "the incoming16px sprite was dry"
    );
    assert_eq!(count(game, "oBlood"), 0);
    assert_eq!(count(game, "oLeaf"), 0);
    trap
}

#[test]
fn natural_mantrap_meal_samples_full_canvas_without_subtracting_its_origin() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        game.world.set_tile(10, 6, "oWaterSwim");
        let id = meal(&mut game);
        game.step(&InputFrame::default());
        let actor = game.world.entity(id).unwrap();
        // Eat origin8,16 would incorrectly place a visual-center probe at
        // x+8,y; the source canvas probe is x+16,y+16 in cell10,6.
        assert_eq!(actor.value("swimming"), 1.0, "{rules:?}");
        assert_eq!(actor.value("inherited_gravity"), 0.2);
        assert_eq!(count(&game, "oSplash"), 1);
        assert_eq!(count(&game, "oBlood"), 0);
        assert_eq!(count(&game, "oLeaf"), 0);
    }
}

#[test]
fn natural_mantrap_meal_lava_uses_canvas_width_and_thirty_pixel_foot() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (lava_y, expected_leaves, expected_blood) in [(4, 0, 0), (6, 3, 1)] {
            let mut game = room(rules);
            game.world.set_tile(10, lava_y, "oLava");
            let id = meal(&mut game);
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(id).is_none(),
                "{rules:?} lava row{lava_y}"
            );
            // Top-only destruction keeps HP>0, while foot lava setsHP0 and
            // the continuing child event runs its ordinary death effects.
            assert_eq!(count(&game, "oLeaf"), expected_leaves);
            assert_eq!(count(&game, "oBlood"), expected_blood);
            assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        }
    }
}
