//! Natural full-step witnesses for two sparse Alien family tails.
//!
//! Both actors use source Create defaults. Terrain is the authored scene
//! configuration; no post-birth status, health, or motion fields are changed.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

fn room(seed: u32) -> Game {
    let mut game = Game::shell(seed, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 9..12 {
        game.world.set_tile(x, 8, "oBlock");
    }
    game.player.pos = Vec2::new(160.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game
}

#[test]
fn actual_step_alien_solid_crush_runs_its_death_tail_before_destroy_finishes() {
    let mut game = room(42);
    // Alien's source crush probe is (x+8,y+8). Starting the authored enemy in
    // this fixed Solid leaves that probe occupied after moveTo resolves its
    // blocked horizontal displacement.
    game.world.set_tile(4, 4, "oBlock");
    let alien_id = game.spawn("oAlien", Vec2::new(64.0, 64.0));
    let alien = game.world.entity(alien_id).unwrap();
    assert_eq!((alien.health, alien.state), (1, 0));
    assert_eq!(alien.vel.x, 2.5);
    let mut expected_rng = game.rng.clone();
    for _ in 0..12 {
        expected_rng.next_u64();
    }
    expected_rng.range(2);

    game.step(&InputFrame::default());

    assert!(game.world.entity(alien_id).is_none());
    assert_eq!(game.data.get("aliens"), Some(&1.0));
    assert_eq!(game.data.get("kills"), Some(&1.0));
    assert_eq!(
        game.lifetime_stats.enemy_kills[spelunky_core::statistics::EnemyKill::Alien as usize],
        1
    );
    // The old source receiver is retired by cleanup, so its post-Destroy IDLE
    // branch is witnessed by its rand(0,1) after three synchronous Blood
    // Creates (four raw draws each), rather than by reading the tombstone.
    assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
}

#[test]
fn actual_step_alien_eject_stops_at_side_wall_then_converts_on_floor() {
    let mut selected = None;
    // The source Create's random dir chooses LEFT/RIGHT. Search only seeds;
    // the selected actor is otherwise born and simulated normally.
    for seed in 0..128 {
        let mut game = room(seed);
        // The vertical wall is one tile to the right of the 8px side probe;
        // its 4px clearance keeps the EJECT top probe clear. The floor begins
        // below at y=80, so FLOAT has time to hit the wall before its floor
        // point can create the Alien.
        for y in 0..5 {
            game.world.set_tile(5, y, "oBlock");
        }
        game.world.set_tile(4, 5, "oBlock");
        let eject_id = game.spawn("oAlienEject", Vec2::new(72.0, 90.0));
        if game.world.entity(eject_id).unwrap().facing == 1 {
            selected = Some((game, eject_id));
            break;
        }
    }
    let (mut game, eject_id) = selected.expect("a source Create seed faces the authored wall");
    let initial = game.world.entity(eject_id).unwrap();
    assert_eq!(initial.state, 0, "source EJECT default");
    assert_eq!(initial.vel, Vec2::new(0.0, -6.0));
    assert_eq!(initial.facing, 1, "source dir=1 points toward the wall");

    let mut saw_wall_stop = false;
    let mut converted = false;
    let mut conversion_alien_pos = None;
    let mut trace = Vec::new();
    for _ in 0..80 {
        let eject_before = game.world.entity(eject_id).unwrap();
        let conversion_pos = Vec2::new(
            eject_before.pos.x + eject_before.vel.x - 8.0,
            eject_before.pos.y + eject_before.vel.y - 12.0,
        );
        game.step(&InputFrame::default());
        let Some(eject) = game.world.entity(eject_id) else {
            converted = true;
            conversion_alien_pos = Some(conversion_pos);
            break;
        };
        trace.push((eject.state, eject.pos, eject.vel, eject.facing, eject.alive));
        saw_wall_stop |= eject.state == 2 && eject.facing == 0;
    }

    assert!(
        saw_wall_stop,
        "FLOAT reaches the side probe and source dir=99 stop; trace={trace:?}"
    );
    assert!(
        converted,
        "the continuing FLOAT path reaches the floor point; trace={trace:?}"
    );
    let aliens: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oAlien")
        .collect();
    assert_eq!(aliens.len(), 1, "the floor conversion creates one Alien");
    assert_eq!(aliens[0].pos, conversion_alien_pos.unwrap());
    assert_eq!(game.data.get("kills"), None, "conversion is not a kill");
}
