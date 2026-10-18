//! Candidate tests for the five literal scrCreateFlame callers. These drive
//! production Game::step; no alarm, velocity, HP, or fuse is injected mid-run.
//! Initial room/inventory and explicitly noted arriving actors are fixtures.
use spelunky_core::{Area, Entity, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

// Independent literal RNG adapter. Expected effects never call the production
// RNG's unit/range methods or the production flame/debris creation helpers.
// This checks source expression order over the port's SplitMix stream; it does
// not claim that SplitMix reproduces GameMaker's own PRNG algorithm.
struct Oracle(u64);
impl Oracle {
    fn checkpoint(game: &Game) -> Self {
        Self(
            serde_json::to_value(&game.rng).unwrap()["state"]
                .as_u64()
                .unwrap(),
        )
    }
    fn raw(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        x ^ (x >> 31)
    }
    fn real(&mut self) -> f64 {
        (self.raw() >> 11) as f64 / 9_007_199_254_740_992.0
    }
    fn detritus_create(&mut self) -> (Vec2, f64) {
        let x = self.real() * 4.0 - self.real() * 4.0;
        let y = -1.0 - self.real() * 2.0;
        let gravity = (1 + self.raw() % 6) as f64 * 0.1;
        (Vec2::new(x, y), gravity)
    }
    fn assert_tail(&mut self, game: &Game) {
        assert_eq!(
            game.rng.clone().next_u64(),
            self.raw(),
            "unexpected RNG consumption in birth frame"
        );
    }
}

fn room(low_graphics: bool) -> Game {
    let mut game = Game::legacy_shell(0x89ab_cdef_1357_2468, RulesPreset::Classic);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.player.facing = 1;
    game.player.health = 20.0;
    if low_graphics {
        game.flags.insert("low_graphics".into());
    }
    game
}

fn live<'a>(game: &'a Game, object: &str) -> Vec<&'a Entity> {
    let mut entities: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .collect();
    entities.sort_by_key(|e| e.id);
    entities
}

fn wait_for_burst(game: &mut Game, input: InputFrame) -> (Oracle, u64) {
    for elapsed in 0..145 {
        let before = Oracle::checkpoint(game);
        // Escape the blast, then stop while it remains inside the source view.
        // Cooking keeps Attack held for its entire natural countdown.
        let active_input = if elapsed >= 30 && (input.left || input.right) {
            InputFrame::default()
        } else {
            input
        };
        game.step(&active_input);
        if let Some(explosion) = live(game, "oExplosion").first() {
            return (before, explosion.id);
        }
    }
    panic!("natural explosive producer never reached its source expiry");
}

fn displacement(v: f64, time: u64) -> f64 {
    let fraction = v.abs().fract();
    let extra = fraction != 0.0 && time % ((1.0 / fraction).round().max(1.0) as u64) == 0;
    (v.abs().floor() + f64::from(extra)) * v.signum()
}

fn assert_flames(
    game: &Game,
    explosion_id: u64,
    count: usize,
    oracle: &mut Oracle,
    stepped: bool,
    alarm1: i32,
) {
    let explosion = game.world.entity(explosion_id).unwrap();
    let flames: Vec<_> = live(game, "oFlame")
        .into_iter()
        .filter(|e| e.id > explosion_id)
        .collect();
    assert_eq!(flames.len(), count);
    for flame in flames {
        let (velocity, gravity) = oracle.detritus_create();
        assert_eq!(
            flame.previous_pos, explosion.pos,
            "synchronous birth origin/order"
        );
        assert_eq!(flame.vel.x, velocity.x);
        assert_eq!(flame.value("gravity"), gravity);
        assert_eq!(flame.value("life"), if stepped { 59.0 } else { 60.0 });
        assert_eq!(flame.alarms[0], 2);
        assert_eq!(flame.alarms[1], alarm1);
        assert_eq!(flame.image_speed(), 0.3);
        assert!((flame.animation - 0.3).abs() < 1e-12);
        let expected = if stepped {
            let time = (game.level_tick + 1) % 100_000_001;
            Vec2::new(
                explosion.pos.x + displacement(velocity.x, time),
                explosion.pos.y + displacement(velocity.y, time),
            )
        } else {
            explosion.pos
        };
        assert_eq!(flame.pos, expected);
        assert!(
            expected.y + 4.0 < 160.0,
            "oracle fixture must avoid floor bounce"
        );
        assert_eq!(
            flame.vel.y,
            velocity.y + if stepped { gravity } else { 0.0 }
        );
    }
}

fn drop_inventory_bomb(game: &mut Game) -> u64 {
    game.step(&InputFrame {
        down: true,
        bomb_pressed: true,
        ..Default::default()
    });
    let bomb = live(game, "oBomb")[0];
    assert_eq!(bomb.alarms[0], 80);
    assert_eq!(game.player.bombs, 3.0);
    bomb.id
}

fn draw_and_arm_bow(game: &mut Game) -> u64 {
    let bow = game.spawn("oBow", game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(bow),
        "pickup must occur through input"
    );
    assert_eq!(game.data["arrows"], 6.0);
    game.step(&InputFrame::default());
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    // The drawing onset already executed one literal +.2 charge. With
    // stored IEEE doubles, addition60 is still just below12; addition61
    // crosses it. Fully charge BEFORE arming: Player Step1505–1515 assigns
    // holdArrow=NORM on every held charging Step, including the crossing one.
    assert_eq!(game.player.data["bow_strength"], 0.2);
    for _ in 0..60 {
        game.step(&InputFrame {
            attack: true,
            ..Default::default()
        });
    }
    assert!(game.player.data["bow_strength"] >= 12.0);
    assert!(game.player.data["bow_strength"] < 12.21);
    assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    assert_eq!(game.player.held, Some(bow));
    assert_eq!(game.player.pos, Vec2::new(240.0, 152.0));
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.facing, 1);
    assert_eq!(game.player.health, 20.0);
    assert_eq!(game.data["arrows"], 6.0);
    game.step(&InputFrame {
        attack: true,
        bomb_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.data["bow_bomb_fuse"], 80.0);
    assert_eq!(game.player.bombs, 4.0);
    assert_eq!(game.player.data["bow_bomb_armed"], 1.0);
    bow
}

#[test]
fn natural_inventory_bomb_alarm_births_and_graphics_gate_match_literal_rng() {
    for low_graphics in [false, true] {
        let mut game = room(low_graphics);
        let bomb = drop_inventory_bomb(&mut game);
        let (mut oracle, explosion) = wait_for_burst(
            &mut game,
            InputFrame {
                right: true,
                ..Default::default()
            },
        );
        assert!(game.world.entity(bomb).is_none());
        assert_eq!(
            game.player.health, 20.0,
            "escape must avoid unrelated hurt RNG"
        );
        assert_flames(
            &game,
            explosion,
            if low_graphics { 0 } else { 3 },
            &mut oracle,
            true,
            49,
        );
        oracle.assert_tail(&game);
    }
}

#[test]
fn natural_fired_bomb_arrow_alarm_births_and_graphics_gate_match_literal_rng() {
    for low_graphics in [false, true] {
        let mut game = room(low_graphics);
        draw_and_arm_bow(&mut game);
        game.step(&InputFrame::default());
        let arrow = live(&game, "oArrow")[0].id;
        assert_eq!(game.world.entity(arrow).unwrap().value("bomb_arrow"), 1.0);
        let (mut oracle, explosion) = wait_for_burst(
            &mut game,
            InputFrame {
                left: true,
                ..Default::default()
            },
        );
        assert!(game.world.entity(arrow).is_none());
        assert_eq!(game.player.health, 20.0);
        // Arrow465 creates Flame456 after that Alarm1 resource has been visited.
        assert_flames(
            &game,
            explosion,
            if low_graphics { 0 } else { 3 },
            &mut oracle,
            true,
            50,
        );
        oracle.assert_tail(&game);
    }
}

#[test]
fn natural_held_cooking_expiry_births_precede_player_knockback_rng() {
    for low_graphics in [false, true] {
        let mut game = room(low_graphics);
        let bow = draw_and_arm_bow(&mut game);
        let (mut oracle, explosion) = wait_for_burst(
            &mut game,
            InputFrame {
                attack: true,
                ..Default::default()
            },
        );
        assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
        assert_eq!(game.player.health, 10.0);
        assert_flames(
            &game,
            explosion,
            if low_graphics { 0 } else { 3 },
            &mut oracle,
            true,
            50,
        );
        // Source Player Step's next rand(1,3) follows all flame Creates.
        // Later dropped-Bow movement/collision is outside this prefix oracle.
        let speed = 4.0 + (oracle.raw() % 3) as f64;
        assert_eq!(game.player.vel, Vec2::new(-speed, -6.0));
        assert!(!game.world.entity(bow).unwrap().held);
    }
}

fn stomp_fire_frog(game: &mut Game) -> u64 {
    // Initial falling-player arrival over a live, undamaged source-created frog.
    game.player.pos.y = 144.0;
    game.player.previous_pos = game.player.pos;
    game.player.grounded = false;
    game.player.state = PlayerState::Falling;
    let frog = game.spawn("oFireFrog", Vec2::new(232.0, 144.0));
    for _ in 0..6 {
        game.step(&InputFrame::default());
        if !live(game, "oFireFrogBomb").is_empty() {
            break;
        }
    }
    assert!(
        game.world.entity(frog).is_none(),
        "actual stomp must cause death"
    );
    live(game, "oFireFrogBomb")[0].id
}

#[test]
fn natural_fire_frog_stomp_and_delayed_alarm_emit_blood_before_guarded_flames() {
    for low_graphics in [false, true] {
        let mut game = room(low_graphics);
        let bomb = stomp_fire_frog(&mut game);
        let (mut oracle, explosion) = wait_for_burst(
            &mut game,
            InputFrame {
                right: true,
                ..Default::default()
            },
        );
        assert!(game.world.entity(bomb).is_none());
        let blood = live(&game, "oBlood");
        assert_eq!(blood.len(), 3, "Blood3 is outside graphicsHigh");
        for drop in blood {
            let (velocity, gravity) = oracle.detritus_create();
            assert!(drop.id > explosion);
            assert_eq!(drop.vel, velocity, "Blood inherits bounce=false");
            assert_eq!(drop.value("gravity"), gravity);
            assert_eq!(drop.value("life"), 59.0);
            assert_eq!(drop.alarms[1], 1);
        }
        let first_flame = live(&game, "oFlame").first().map(|e| e.id);
        if let Some(first_flame) = first_flame {
            assert!(live(&game, "oBlood").iter().all(|e| e.id < first_flame));
        }
        assert_flames(
            &game,
            explosion,
            if low_graphics { 0 } else { 3 },
            &mut oracle,
            true,
            50,
        );
        oracle.assert_tail(&game);
    }
}

#[test]
fn natural_inventory_alarm_rechecks_shared_detritus_cap_per_birth() {
    for (existing, expected) in [(14, 2), (15, 1), (16, 0), (17, 0)] {
        for low_graphics in [false, true] {
            let mut game = room(low_graphics);
            let bomb = drop_inventory_bomb(&mut game);
            for elapsed in 0..140 {
                if game.world.entity(bomb).unwrap().alarms[1] == 1 {
                    break;
                }
                game.step(&InputFrame {
                    right: elapsed < 30,
                    ..Default::default()
                });
            }
            assert_eq!(game.world.entity(bomb).unwrap().alarms[1], 1);
            // Controlled arriving effects, created normally before the natural
            // alarm phase. Their lifetimes/velocities/alarms remain source values.
            let arrival = Vec2::new(game.camera.x + 240.0, 80.0);
            for _ in 0..existing {
                game.spawn("oFlame", arrival);
            }
            let mut oracle = Oracle::checkpoint(&game);
            game.step(&InputFrame::default());
            let explosion = live(&game, "oExplosion")[0].id;
            assert_flames(
                &game,
                explosion,
                if low_graphics { 0 } else { expected },
                &mut oracle,
                true,
                49,
            );
            oracle.assert_tail(&game);
        }
    }
}

#[test]
fn natural_fire_frog_alarm_spends_shared_cap_on_blood_before_any_flames() {
    for existing in [12usize, 13, 14, 15, 16, 17] {
        for low_graphics in [false, true] {
            let mut game = room(low_graphics);
            let bomb = stomp_fire_frog(&mut game);
            for elapsed in 0..140 {
                if game.world.entity(bomb).unwrap().alarms[1] == 1 {
                    break;
                }
                game.step(&InputFrame {
                    right: elapsed < 30,
                    ..Default::default()
                });
            }
            assert_eq!(game.world.entity(bomb).unwrap().alarms[1], 1);
            let arrival = Vec2::new(game.camera.x + 240.0, 80.0);
            for _ in 0..existing {
                game.spawn("oFlame", arrival);
            }
            let mut oracle = Oracle::checkpoint(&game);
            game.step(&InputFrame::default());
            let explosion = live(&game, "oExplosion")[0].id;
            let available = 16usize.saturating_sub(existing);
            let expected_blood = available.min(3);
            let blood = live(&game, "oBlood");
            assert_eq!(blood.len(), expected_blood);
            for drop in blood {
                let (velocity, gravity) = oracle.detritus_create();
                assert!(drop.id > explosion);
                assert_eq!(drop.vel, velocity);
                assert_eq!(drop.value("gravity"), gravity);
            }
            let expected_flames = if low_graphics {
                0
            } else {
                (available - expected_blood).min(3)
            };
            assert_flames(&game, explosion, expected_flames, &mut oracle, true, 50);
            oracle.assert_tail(&game);
        }
    }
}

#[test]
fn naturally_fired_bomb_arrow_enemy_query_then_damsel_query_emit_only_one_burst() {
    for enemy_overlap in [false, true] {
        for low_graphics in [false, true] {
            let mut game = room(low_graphics);
            draw_and_arm_bow(&mut game);
            // scrFireBow32–58 births at x+14 with speed1+strength; Item's
            // same-frame moveTo uses the next source engine clock. Preserve
            // the original arrival geometry relative to that endpoint: Snake
            // origin nine pixels left, Damsel origin two pixels left. These
            // actors still use their real Create and later own Step callbacks.
            // No projectile position, velocity, fuse or HP is assigned.
            let source_spawn = Vec2::new(game.player.pos.x + 14.0, game.player.pos.y);
            let source_speed = 1.0 + game.player.data["bow_strength"];
            let next_time = game.engine_time().wrapping_add(1) % 100_000_001;
            let endpoint = Vec2::new(
                source_spawn.x + displacement(source_speed, next_time),
                source_spawn.y,
            );
            assert!(source_speed > 13.0 && source_speed < 13.21);
            assert!(
                !game.world.rectangle_family_except(
                    Vec2::new(source_spawn.x - 4.0, source_spawn.y - 4.0),
                    Vec2::new(endpoint.x + 4.0, endpoint.y + 3.0),
                    "oSolid",
                    None,
                    spelunky_core::CollisionPrecision::Precise,
                ),
                "source Arrow's entire first horizontal path must be clear"
            );
            let snake = enemy_overlap
                .then(|| game.spawn("oSnake", Vec2::new(endpoint.x - 9.0, endpoint.y - 8.0)));
            let damsel = game.spawn("oDamsel", Vec2::new(endpoint.x - 2.0, endpoint.y));
            let mut oracle = Oracle::checkpoint(&game);
            game.step(&InputFrame::default());
            assert!(live(&game, "oArrow").is_empty());
            assert!(
                game.world.entity(damsel).unwrap().health < 4,
                "independent Damsel query must continue after Arrow destruction"
            );
            let explosions = live(&game, "oExplosion");
            assert_eq!(explosions.len(), usize::from(enemy_overlap));
            if let Some(snake) = snake {
                assert!(
                    game.world.entity(snake).is_none(),
                    "natural arrow kills snake"
                );
                // Enemy Blood1 precedes Explosion and Flame3; Damsel Blood1
                // follows those flame Creates, even though the Arrow is dead.
                let blood = live(&game, "oBlood");
                assert!(blood[0].id < explosions[0].id);
                oracle.detritus_create();
                assert_flames(
                    &game,
                    explosions[0].id,
                    if low_graphics { 0 } else { 3 },
                    &mut oracle,
                    false,
                    50,
                );
                let flames = live(&game, "oFlame");
                let continuation_blood = blood[1];
                if let Some(last_flame) = flames.last() {
                    assert!(last_flame.id < continuation_blood.id);
                    let (velocity, gravity) = oracle.detritus_create();
                    assert_eq!(continuation_blood.vel, velocity);
                    assert_eq!(continuation_blood.value("gravity"), gravity);
                }
                // This route checks the exact Create prefix. Subsequent snake
                // death and automatic NPC/item collision callbacks also draw
                // RNG; their whole-frame budget belongs to separate audits.
            } else {
                assert!(
                    live(&game, "oFlame").is_empty(),
                    "Damsel-only impact has no flame caller"
                );
                assert_eq!(game.world.entity(damsel).unwrap().health, 3);
            }
        }
    }
}
