//! Authored/default Frog, ordinary jump/stomp, no post-start receiver writes.
//! This is a natural airborne death control; the retired tail draws no RNG.
use spelunky_core::{level::Level, *};

#[test]
fn ordinary_jump_stomp_kills_default_frog_and_airborne_recover_death_has_no_tail_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut level = Level::default();
        level.set_tile(6, 8, '@').unwrap();
        level.set_tile(34, 8, 'X').unwrap();
        level.set_tile(8, 8, 'f').unwrap();
        for x in 0..spelunky_core::level::LEVEL_WIDTH {
            level.set_tile(x, 9, '1').unwrap();
        }
        let mut game = Game::shell(42, rules);
        game.load_custom(&level, "frog-natural-stomp-death", "");
        let frog = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oFrog")
            .unwrap();
        assert_eq!(frog.pos, Vec2::new(144.0, 144.0));
        assert_eq!(frog.health, 1);
        assert_eq!(frog.state, 0);
        assert_eq!(frog.timer, 0);
        assert_eq!(frog.vel, Vec2::default());
        let id = frog.id;
        assert_eq!(game.player.pos, Vec2::new(120.0, 152.0));

        for tick in 0..9 {
            game.step(&InputFrame {
                jump: tick < 6,
                jump_pressed: tick == 0,
                jump_released: tick == 6,
                ..Default::default()
            });
        }
        let frog = game.world.entity(id).unwrap();
        assert_eq!(
            frog.health, 0,
            "ordinary Character collision must stomp the Frog"
        );
        assert_eq!(frog.state, 2, "natural jump reaches source RECOVER");
        assert_eq!(frog.pos, Vec2::new(123.0, 127.0));
        assert!(frog.vel.y < 0.0);
        assert!(!game.world.collides(Rect::new(
            frog.bounds().x,
            frog.bounds().y + frog.bounds().h,
            frog.bounds().w,
            1.0,
        )));
        assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        let first_birth = game.world.next_id;
        let mut expected = game.rng.clone();
        // Three synchronous Blood Creates, each with exactly four raw draws.
        // The airborne RECOVER tail performs no rand(10,40).
        for _ in 0..12 {
            expected.next_u64();
        }

        game.step(&InputFrame::default());

        assert!(game.world.entity(id).is_none());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood" && e.id >= first_birth)
                .count(),
            3
        );
        assert!(
            !game
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndFrog"))
        );
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    }
}
