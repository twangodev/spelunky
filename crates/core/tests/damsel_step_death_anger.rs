//! The source Damsel Step calls reason-3 anger for cost>0 and hp<=0 before
//! its hazard statements. Authored stock/free-loot metadata is initial context;
//! controller grounding, weapon acquisition and blade attacks are ordinary
//! inputs. The Exit test explicitly authors a dead Exit receiver to isolate
//! the existing early-return path. No actor or RNG is edited after first Step.
use spelunky_core::{Area, Game, GameEvent, InputFrame, RngAlgorithm, RulesPreset, Vec2, World};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const ALGORITHMS: [RngAlgorithm; 2] = [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1];

struct Scene {
    target: Vec2,
    floor: usize,
    cost: f64,
    for_sale: bool,
    keeper: bool,
    lava: bool,
    blade: bool,
    controlled_dead_exit: bool,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            target: Vec2::new(104.0, 152.0),
            floor: 10,
            cost: 15000.0,
            for_sale: true,
            keeper: true,
            lava: false,
            blade: true,
            controlled_dead_exit: false,
        }
    }
}

fn arena(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    scene: &Scene,
) -> (Game, u64, Option<u64>, Option<u64>) {
    let mut game = Game::shell_with_algorithm(23, rules, algorithm).unwrap();
    game.world = World::new(42, 34, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..42 {
        game.world.set_tile(x, scene.floor, "oHardBlock");
    }
    let player_y = scene.floor as f64 * 16.0 - 8.0;
    game.player.pos = Vec2::new(if scene.lava { 40.0 } else { 88.0 }, player_y);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    let keeper = scene.keeper.then(|| {
        game.spawn(
            "oShopkeeper",
            Vec2::new(144.0, scene.floor as f64 * 16.0 - 16.0),
        )
    });
    if scene.lava {
        let x = (scene.target.x / 16.0) as usize;
        // The source fluid pass drains uncontained cells before the Damsel
        // Step. Native side walls and the floor below preserve this surface.
        game.world.set_tile(x - 1, 10, "oHardBlock");
        game.world.set_tile(x + 1, 10, "oHardBlock");
        game.world.set_tile(x, 10, "oLava");
    }
    let target = game.spawn("oDamsel", scene.target);
    {
        let actor = game.world.entity_mut(target).unwrap();
        // Source shop stock: SLAVE5 and forSale, with paired free or unsold
        // metadata controls. Default health/motion/timers are preserved.
        actor.state = if scene.for_sale { 5 } else { 0 };
        actor.data.insert("cost".into(), scene.cost);
        actor
            .data
            .insert("forSale".into(), f64::from(scene.for_sale));
        if scene.controlled_dead_exit {
            actor.health = 0;
            actor.state = 4;
            actor.set_sprite("sDamselExit2");
        }
    }
    let blade = scene.blade.then(|| {
        let id = game.spawn("oMachete", game.player.pos);
        let item = game.world.entity_mut(id).unwrap();
        // Native free-loot/editor overrides, not untouched cost7000 Create.
        item.data.insert("cost".into(), 0.0);
        item.data.insert("forSale".into(), 0.0);
        id
    });
    (game, target, keeper, blade)
}

fn reason_three(game: &Game) -> bool {
    game.events.iter().any(|event| {
        matches!(event,
        GameEvent::Dialogue(message) if message.first == "HEY, ONLY I CAN DO THAT!"
            && message.second.is_empty() && message.ticks == 80)
    })
}

fn no_reaction(game: &Game, keeper: Option<u64>) {
    if let Some(id) = keeper {
        assert_eq!(game.world.entity(id).unwrap().state, 0);
    }
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
    assert!(!game.flags.contains("wanted"));
    assert!(!game.flags.contains("murderer"));
    assert!(
        !game.flags.contains("shopkeepers_angry"),
        "no invented global anger flag"
    );
    assert!(!reason_three(game));
    assert_eq!(game.player.health, 4.0);
}

fn reaction(game: &Game, keeper: u64) {
    assert_eq!(game.world.entity(keeper).unwrap().state, 2);
    assert_eq!(game.world.entity(keeper).unwrap().health, 20);
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 2.0);
    assert!(game.flags.contains("wanted"));
    assert!(!game.flags.contains("murderer"));
    assert!(
        !game.flags.contains("shopkeepers_angry"),
        "source script does not set this flag"
    );
    assert!(
        reason_three(game),
        "source cost-death Step must call reason3"
    );
    assert_eq!(game.player.health, 4.0);
}

fn acquire_blade(game: &mut Game, blade: u64, target: u64, keeper: Option<u64>, cost: f64) {
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(blade),
        "ordinary Machete acquisition is required"
    );
    assert!(game.world.entity(blade).unwrap().held);
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(blade));
    assert_eq!(game.world.entity(target).unwrap().health, 4);
    assert_eq!(
        game.world.entity(target).unwrap().value("cost"),
        cost,
        "same-room keeper must preserve supported initial stock price"
    );
    no_reaction(game, keeper);
}

fn blade_hit(game: &mut Game, target: u64, expected_hp: i32) {
    let initial_hp = game.world.entity(target).unwrap().health;
    let initial_state = game.world.entity(target).unwrap().state;
    for frame in 0..18 {
        game.step(&InputFrame {
            attack: frame == 0,
            attack_pressed: frame == 0,
            ..Default::default()
        });
        let actor = game
            .world
            .entity(target)
            .expect("default stock survives the blade callback");
        if actor.health != initial_hp {
            assert_eq!(
                actor.health, expected_hp,
                "one ordinary native Machete damage2 hit"
            );
            // Source Slash AnimationEnd runs after collision and Draw; read
            // the exact contacting producer before that retirement phase.
            let shown = game.presentation();
            let shown_actor = shown.world.entity(target).unwrap();
            assert!(
                shown.world.entities.iter().any(|e| {
                    e.alive && e.object == "oSlash" && e.event_intersects(shown_actor)
                })
            );
            assert_eq!(actor.value("hit_cooldown"), 10.0);
            assert_eq!(
                actor.state, initial_state,
                "blade callback preserves the initial source status"
            );
            return;
        }
    }
    panic!("ordinary front blade never hit its authored target");
}

fn wait_next_attack(game: &mut Game, target: u64) {
    for _ in 0..24 {
        if game.player.attack_timer == 0
            && game.world.entity(target).unwrap().value("hit_cooldown") == 0.0
        {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.attack_timer, 0);
    assert_eq!(
        game.world.entity(target).unwrap().value("hit_cooldown"),
        0.0
    );
}

#[test]
fn priced_stock_two_blade_hits_trigger_reason_three_in_the_next_damsel_step() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for sale in [true, false] {
                let scene = Scene {
                    for_sale: sale,
                    ..Default::default()
                };
                let (mut game, target, keeper, blade) = arena(rules, algorithm, &scene);
                acquire_blade(&mut game, blade.unwrap(), target, keeper, scene.cost);
                blade_hit(&mut game, target, 2);
                no_reaction(&game, keeper);
                wait_next_attack(&mut game, target);
                blade_hit(&mut game, target, 0);
                assert_eq!(game.world.entity(target).unwrap().value("cost"), 15000.0);
                no_reaction(&game, keeper); // Collision follows the normal Step guard.
                game.step(&InputFrame::default());
                reaction(&game, keeper.unwrap());
                assert_eq!(game.world.entity(target).unwrap().health, 0);
            }
        }
    }
}

#[test]
fn free_stock_blade_death_does_not_react_even_with_for_sale_true() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            let scene = Scene {
                cost: 0.0,
                ..Default::default()
            };
            let (mut game, target, keeper, blade) = arena(rules, algorithm, &scene);
            acquire_blade(&mut game, blade.unwrap(), target, keeper, scene.cost);
            blade_hit(&mut game, target, 2);
            wait_next_attack(&mut game, target);
            blade_hit(&mut game, target, 0);
            assert_eq!(game.world.entity(target).unwrap().value("forSale"), 1.0);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(target).unwrap().health, 0);
            no_reaction(&game, keeper);
        }
    }
}

#[test]
fn surviving_priced_stock_after_one_blade_hit_does_not_react() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            let scene = Scene::default();
            let (mut game, target, keeper, blade) = arena(rules, algorithm, &scene);
            acquire_blade(&mut game, blade.unwrap(), target, keeper, scene.cost);
            blade_hit(&mut game, target, 2);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(target).unwrap().health, 2);
            assert_eq!(game.world.entity(target).unwrap().value("cost"), 15000.0);
            no_reaction(&game, keeper);
        }
    }
}

#[test]
fn shallow_lava_stock_death_waits_until_the_next_step_to_react() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            let scene = Scene {
                target: Vec2::new(104.0, 154.0),
                floor: 11,
                lava: true,
                blade: false,
                ..Default::default()
            };
            let (mut game, target, keeper, _) = arena(rules, algorithm, &scene);
            assert_eq!(game.world.entity(target).unwrap().health, 4);
            game.step(&InputFrame::default());
            let actor = game
                .world
                .entity(target)
                .expect("shallow surface avoids parent lava destruction");
            assert_eq!(
                actor.health, 0,
                "own lava hazard must naturally cause the lethal change"
            );
            assert_eq!(actor.state, 99);
            assert_eq!(actor.value("cost"), 15000.0);
            assert!(
                actor.pos.y - 5.0 < 160.0,
                "parent's above-origin lava destroy query stays clear"
            );
            assert!(actor.pos.y + 6.0 >= 160.0, "child's foot query enters lava");
            no_reaction(&game, keeper);
            game.step(&InputFrame::default());
            reaction(&game, keeper.unwrap());
            assert_eq!(game.world.entity(target).unwrap().health, 0);
        }
    }
}

#[test]
fn free_stock_shallow_lava_death_has_no_cost_based_reaction() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            let scene = Scene {
                target: Vec2::new(104.0, 154.0),
                floor: 11,
                cost: 0.0,
                lava: true,
                blade: false,
                ..Default::default()
            };
            let (mut game, target, keeper, _) = arena(rules, algorithm, &scene);
            for _ in 0..2 {
                game.step(&InputFrame::default());
                let actor = game.world.entity(target).unwrap();
                assert_eq!(actor.health, 0);
                assert_eq!(actor.value("cost"), 0.0);
                assert_eq!(actor.value("forSale"), 1.0);
                no_reaction(&game, keeper);
            }
        }
    }
}

#[test]
fn shallow_lava_dead_priced_damsel_outside_item_view_has_no_keeper_to_anger() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            let scene = Scene {
                target: Vec2::new(400.0, 154.0),
                floor: 11,
                keeper: false,
                for_sale: false,
                lava: true,
                blade: false,
                ..Default::default()
            };
            let (mut game, target, keeper, _) = arena(rules, algorithm, &scene);
            for _ in 0..2 {
                game.step(&InputFrame::default());
                let actor = game
                    .world
                    .entity(target)
                    .expect("engine-active corpse remains alive");
                assert!(actor.engine_active);
                assert_eq!(actor.health, 0);
                assert_eq!(
                    actor.value("cost"),
                    15000.0,
                    "parent viewport gate must preserve price despite missing keeper"
                );
                assert!(actor.pos.x >= game.camera.x + spelunky_core::VIEW_WIDTH + 16.0);
                assert!(
                    actor.pos.x < game.camera.x + spelunky_core::VIEW_WIDTH + 96.0,
                    "source activation region still includes the Damsel"
                );
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.alive && spelunky_core::catalog::is_a(&e.object, "oShopkeeper"))
                );
                no_reaction(&game, keeper);
            }
        }
    }
}

#[test]
fn controlled_initial_dead_exit_applies_only_the_literal_cost_guard() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for cost in [15000.0, 0.0] {
                let scene = Scene {
                    cost,
                    blade: false,
                    controlled_dead_exit: true,
                    ..Default::default()
                };
                let (mut game, target, keeper, _) = arena(rules, algorithm, &scene);
                game.step(&InputFrame::default());
                let actor = game.world.entity(target).unwrap();
                assert_eq!(actor.health, 0);
                assert_eq!(actor.state, 4);
                assert_eq!(actor.value("invincible"), 1.0);
                if cost > 0.0 {
                    reaction(&game, keeper.unwrap());
                } else {
                    no_reaction(&game, keeper);
                }
            }
        }
    }
}
