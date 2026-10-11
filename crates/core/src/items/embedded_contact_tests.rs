//! Source-only regressions, not executed during preparation.
//! oItem Step.xml:202-229 retains the pre-escape edge-probe snapshot.
use super::*;

const CONTACTS: [&str; 4] = ["colLeft", "colRight", "colBot", "colTop"];

fn fixture(preset: RulesPreset) -> Game {
    let mut game = Game::shell(7, preset);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.world.name = "fixture".into();
    game.world.set_tile(12, 10, "oBrick");
    game.player.pos = Vec2::new(200.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game
}

fn seed_opposite_contacts(game: &mut Game, id: u64, expected: [bool; 4]) {
    let entity = game.world.entity_mut(id).unwrap();
    for (name, expected) in CONTACTS.into_iter().zip(expected) {
        entity.data.insert(name.into(), f64::from(!expected));
    }
}

fn assert_contacts(entity: &Entity, expected: [bool; 4]) {
    for (name, expected) in CONTACTS.into_iter().zip(expected) {
        assert_eq!(entity.value(name), f64::from(expected), "{name}");
    }
}

fn embedded_contacts(object: &str) {
    // Nine rectangular Bricks occupy x/y 96..143. Ordinary/Arrow/Flare/Bomb
    // bounds are [-4,4); FireFrogBomb bounds are [-6,-4,6,8). Both sets have
    // the same source contact booleans at the chosen embedded coordinates.
    // Deliberately opposite old flags expose both false->true and true->false
    // writes. All assertions concern the first real authoritative Game.step.
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (start, expected_position, contacts, stopped) in [
            (
                Vec2::new(104.0, 104.0),
                Vec2::new(104.0, 104.0),
                [true, true, true, true],
                true,
            ),
            (
                Vec2::new(104.0, 143.0),
                Vec2::new(104.0, 144.0),
                [true, true, false, true],
                false,
            ),
            (
                Vec2::new(96.0, 104.0),
                Vec2::new(95.0, 104.0),
                [false, true, true, true],
                false,
            ),
            (
                Vec2::new(143.0, 104.0),
                Vec2::new(144.0, 104.0),
                [true, false, true, true],
                false,
            ),
        ] {
            let mut game = fixture(preset);
            for y in 6..9 {
                for x in 6..9 {
                    game.world.set_tile(x, y, "oBrick");
                }
            }
            let rock = game.spawn(object, start);
            let velocity = Vec2::new(3.0, 2.0);
            game.world.entity_mut(rock).unwrap().vel = velocity;
            seed_opposite_contacts(&mut game, rock, contacts);
            game.step(&InputFrame::default());
            let entity = game.world.entity(rock).unwrap();
            assert_eq!(
                entity.pos, expected_position,
                "{preset:?} {object} {start:?}"
            );
            assert_eq!(
                entity.vel,
                if stopped { Vec2::default() } else { velocity },
                "embedded escape must not run free-body gravity/bounces"
            );
            assert_contacts(entity, contacts);
        }
    }
}

#[test]
fn game_step_embedded_gold_idol_retains_support_for_its_child_trigger() {
    // GoldIdol Step55-58 reads parent colBot after the inherited Step. This
    // initial embedded placement reproduces the lost-probe consumer without
    // mixing in the separately audited shop-redemption/Destroy continuation.
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = fixture(preset);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBrickSmooth");
        }
        let idol = game.spawn("oGoldIdol", Vec2::new(140.0, 160.0));
        assert_eq!(game.world.entity(idol).unwrap().value("trigger"), 1.0);
        assert_eq!(game.world.entity(idol).unwrap().value("colBot"), 0.0);
        game.step(&InputFrame::default());
        let entity = game.world.entity(idol).unwrap();
        assert_eq!(entity.pos, Vec2::new(140.0, 160.0));
        assert_contacts(entity, [true, true, true, false]);
        assert_eq!(entity.value("trigger"), 1.0);
        assert_eq!(game.data.get("idols_grabbed").copied().unwrap_or(0.0), 0.0);
        assert_eq!(
            game.lifetime_stats.idols_grabbed, 0,
            "embedded support must not count an unsupported Idol"
        );
    }
}

fn free_contacts(object: &str) {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for grounded in [false, true] {
            let mut game = fixture(preset);
            game.world.set_tile(6, 6, "oBrick");
            let floor_y = if object == "oFireFrogBomb" {
                88.0
            } else {
                92.0
            };
            let start = Vec2::new(104.0, if grounded { floor_y } else { 72.0 });
            let rock = game.spawn(object, start);
            let expected_contacts = [false, false, grounded, false];
            seed_opposite_contacts(&mut game, rock, expected_contacts);
            if !grounded {
                game.world.entity_mut(rock).unwrap().vel = Vec2::new(2.0, 1.0);
            }
            game.step(&InputFrame::default());
            let entity = game.world.entity(rock).unwrap();
            assert_eq!(
                entity.pos,
                if grounded {
                    start
                } else {
                    Vec2::new(106.0, 73.0)
                }
            );
            assert_eq!(entity.vel.x, if grounded { 0.0 } else { 2.0 });
            // Arrow Create selects myGrav=.2; the other source Creates use .6.
            // Parent Step resets myGrav=.6 only after this first acceleration.
            let expected_y_velocity = if grounded {
                0.0
            } else if object == "oArrow" {
                1.2
            } else {
                1.6
            };
            assert!((entity.vel.y - expected_y_velocity).abs() < 1e-9);
            assert_contacts(entity, expected_contacts);
        }
    }
}

#[test]
fn game_step_embedded_rock_refreshes_source_contacts() {
    embedded_contacts("oRock");
}

#[test]
fn game_step_free_rock_retains_source_probe_snapshot() {
    free_contacts("oRock");
}

#[test]
fn game_step_embedded_arrow_refreshes_source_contacts() {
    embedded_contacts("oArrow");
}

#[test]
fn game_step_free_arrow_retains_source_probe_snapshot() {
    free_contacts("oArrow");
}

#[test]
fn game_step_embedded_flare_refreshes_source_contacts() {
    embedded_contacts("oFlare");
}

#[test]
fn game_step_free_flare_retains_source_probe_snapshot() {
    free_contacts("oFlare");
}

#[test]
fn game_step_embedded_bomb_refreshes_source_contacts() {
    embedded_contacts("oBomb");
}

#[test]
fn game_step_free_bomb_retains_source_probe_snapshot() {
    free_contacts("oBomb");
}

#[test]
fn game_step_embedded_fire_frog_bomb_refreshes_source_contacts() {
    embedded_contacts("oFireFrogBomb");
}

#[test]
fn game_step_free_fire_frog_bomb_retains_source_probe_snapshot() {
    free_contacts("oFireFrogBomb");
}
