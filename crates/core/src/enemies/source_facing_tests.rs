//! Source-facing regressions: authored initial scenes and ordinary full steps.
use super::*;

fn room(seed: u32, rules: RulesPreset, player_x: f64) -> Game {
    let mut game = Game::shell(seed, rules);
    game.world = World::new(20, 16, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(player_x, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game
}

fn boings(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBoing"))
        .count()
}

#[test]
fn natural_scarab_spring_uses_inherited_left() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(42, rules, 40.0);
        let spring = game.spawn("oSpringTrap", Vec2::new(96.0, 144.0));
        let scarab = game.spawn("oScarab", Vec2::new(96.0, 147.0));
        // Source frame0 RECTANGLE masks overlap at y156..159. The Scarab
        // movement bottom159 stays above the floor; no actor is adjusted.
        assert!(
            game.world
                .entity(scarab)
                .unwrap()
                .event_intersects(game.world.entity(spring).unwrap())
        );
        game.step(&crate::InputFrame::default());
        let scarab = game.world.entity(scarab).unwrap();
        assert_eq!(scarab.pos, Vec2::new(96.0, 131.0));
        assert_eq!(scarab.vel, Vec2::new(-1.0, -8.0));
        assert_eq!(scarab.health, 1);
        let spring = game.world.entity(spring).unwrap();
        assert_eq!((spring.state, spring.timer), (1, 10));
        assert_eq!(boings(&game), 1);
        assert_eq!(game.player.health, 4);
    }
}

#[test]
fn natural_spider_spring_uses_inherited_left() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for with_spring in [false, true] {
            let mut game = room(42, rules, 240.0);
            let spring = with_spring.then(|| game.spawn("oSpringTrap", Vec2::new(96.0, 144.0)));
            let spider = game.spawn("oSpider", Vec2::new(96.0, 147.0));
            // Source initial sSpiderFlip frame0 reaches y156, touching the
            // Spring mask at156..159 without touching the floor at160.
            // Its separate movement bounds reach the floor on both sides:
            // Spider Step sets xVel=+1 then-1. The no-Spring scene proves
            // that shared behavior; Spring applies one further LEFT boost.
            if let Some(spring) = spring {
                assert!(
                    game.world
                        .entity(spider)
                        .unwrap()
                        .event_intersects(game.world.entity(spring).unwrap())
                );
            }
            game.step(&crate::InputFrame::default());
            let spider = game.world.entity(spider).unwrap();
            assert_eq!(spider.health, 1);
            assert_eq!(game.player.health, 4);
            if let Some(spring) = spring {
                assert_eq!(spider.pos, Vec2::new(96.0, 131.0));
                assert_eq!(spider.vel, Vec2::new(-2.0, -8.0));
                let spring = game.world.entity(spring).unwrap();
                assert_eq!((spring.state, spring.timer), (1, 10));
                assert_eq!(boings(&game), 1);
            } else {
                assert_eq!(spider.pos, Vec2::new(96.0, 147.0));
                assert_eq!(spider.vel.x, -1.0);
                assert_eq!(boings(&game), 0);
            }
        }
    }
}

#[test]
fn natural_snake_spring_uses_step_facing_after_its_right_create() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Snake Create finishes RIGHT, but its first IDLE Step selects a
        // new facing before Spring's automatic Collision with oEnemy.
        // Independent GM8 LCG first draws: seed0 -> 1 (LEFT),
        // seed16 -> 2_156_413_009 (RIGHT). No Create in this room draws RNG.
        for (seed, expected_facing, expected_x_vel) in [(0_u32, -1, 1.5), (16, 1, 3.5)] {
            let mut game = room(seed, rules, 40.0);
            let spring = game.spawn("oSpringTrap", Vec2::new(96.0, 144.0));
            let snake = game.spawn("oSnake", Vec2::new(96.0, 144.0));
            let created = game.world.entity(snake).unwrap();
            assert_eq!(created.facing, 1);
            assert_eq!(created.vel.x, 2.5);
            assert_eq!((created.state, created.timer), (0, 0));
            let first = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
            let direction = ((u64::from(first) * 2) >> 32) as u32;
            assert_eq!(if direction == 0 { -1 } else { 1 }, expected_facing);
            assert_eq!(game.rng.clone().next_u64(), u64::from(first));
            game.step(&crate::InputFrame::default());
            let snake = game.world.entity(snake).unwrap();
            assert_eq!(snake.pos, Vec2::new(99.0, 128.0));
            assert_eq!(snake.facing, expected_facing);
            assert_eq!(snake.state, 1);
            assert_eq!(snake.vel, Vec2::new(expected_x_vel, -8.0));
            assert_eq!(game.world.entity(spring).unwrap().state, 1);
            assert_eq!(boings(&game), 1);
            assert_eq!(game.player.health, 4);
            let next = first.wrapping_mul(0x0808_8405).wrapping_add(1);
            assert_eq!(game.rng.clone().next_u64(), u64::from(next));
        }
    }
}

#[test]
fn natural_spider_alarm_still_jumps_toward_player_on_the_right() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(42, rules, 240.0);
        let spider = game.spawn("oSpider", Vec2::new(96.0, 144.0));
        let mut jumped = false;
        for _ in 0..24 {
            game.step(&crate::InputFrame::default());
            let spider = game.world.entity(spider).unwrap();
            if spider.pos.x > 96.0 && spider.vel.y < 0.0 {
                assert_eq!(spider.vel.x, 2.5);
                jumped = true;
                break;
            }
        }
        assert!(jumped, "the source5..20tick alarm must launch naturally");
    }
}

#[test]
fn natural_fish_swimming_directions_remain_independent_of_inherited_left() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Source Piranha rand(1,2)==1 selects dir180; otherwise dir0.
        // Independent GM8 LCG first draws: seed0 -> 1 (mulhi2=0),
        // seed16 -> 2_156_413_009 (mulhi2=1), exercising both source
        // Create outcomes. DeadFish selects dir0 without consuming a draw.
        for (object, seed, delta) in [
            ("oPiranha", 0, -3.0),
            ("oPiranha", 16, 3.0),
            ("oDeadFish", 42, 3.0),
        ] {
            let mut game = room(seed, rules, 40.0);
            for y in 4..=8 {
                for x in 4..=10 {
                    game.world.set_tile(x, y, "oWater");
                }
            }
            let fish = game.spawn(object, Vec2::new(96.0, 96.0));
            for _ in 0..3 {
                game.step(&crate::InputFrame::default());
            }
            let fish = game.world.entity(fish).unwrap();
            assert_eq!(fish.pos, Vec2::new(96.0 + delta, 96.0), "{object}");
            assert_eq!(fish.state, 0, "{object}");
            assert_eq!(game.player.health, 4);
        }
    }
}
