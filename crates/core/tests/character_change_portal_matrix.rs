//! The authored character-change portals toggle distinct Classic player flags.
//! Drive each source Animation End branch through `Game::step` from every
//! starting character so reverse changes stay covered as the adapters evolve.
use spelunky_core::{
    Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2,
    characters::{self, Character},
    hub::HubKind,
};

const CHARACTERS: [Character; 3] = [Character::Explorer, Character::Damsel, Character::TunnelMan];

#[test]
fn character_change_doors_match_classic_toggle_matrix_through_game_step() {
    // oPlayer1 Animation End.xml: xCHANGE toggles Damsel; xCHANGE2 toggles
    // Tunnel Man. Both branches restore character-specific starting inventory.
    for (room, unlock, door, changes_to) in [
        (
            HubKind::Scores,
            "unlock_damsel",
            "oXChange",
            Character::Damsel,
        ),
        (
            HubKind::Title,
            "unlock_tunnel_man",
            "oXChange2",
            Character::TunnelMan,
        ),
    ] {
        for initial in CHARACTERS {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.flags.insert(unlock.into());
            characters::apply(&mut game, initial);
            game.load_hub(room);

            let door_pos = game
                .world
                .entities
                .iter()
                .find(|entity| entity.object == door)
                .unwrap_or_else(|| panic!("{room:?} does not contain unlocked {door}"))
                .pos;
            game.player.pos = Vec2::new(door_pos.x + 8.0, door_pos.y + 8.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.health = 1.0;
            game.player.bombs = 1.0;
            game.player.ropes = 1.0;

            game.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert!(game.player.is_exiting(), "{room:?} {initial:?}");
            assert_eq!(game.state, RunState::Ending, "{room:?} {initial:?}");

            for _ in 0..80 {
                if game.state != RunState::Ending {
                    break;
                }
                game.step(&InputFrame::default());
            }

            let expected = if initial == changes_to {
                Character::Explorer
            } else {
                changes_to
            };
            assert_eq!(game.state, RunState::Playing, "{room:?} {initial:?}");
            assert_eq!(
                characters::selected(&game),
                expected,
                "{room:?} {initial:?}"
            );
            assert_eq!(
                game.player.sprite,
                match expected {
                    Character::Explorer => "sStandLeft",
                    Character::Damsel => "sDamselLeft",
                    Character::TunnelMan => "sTunnelLeft",
                },
                "{room:?} {initial:?}"
            );
            assert_eq!(
                game.player.health,
                if expected == Character::TunnelMan {
                    2.0
                } else {
                    4.0
                }
            );
            assert_eq!(
                game.player.bombs,
                if expected == Character::TunnelMan {
                    0.0
                } else {
                    4.0
                }
            );
            assert_eq!(
                game.player.ropes,
                if expected == Character::TunnelMan {
                    0.0
                } else {
                    4.0
                }
            );
            assert_eq!(
                game.flags.contains("character_damsel"),
                expected == Character::Damsel,
                "{room:?} {initial:?}"
            );
            assert_eq!(
                game.flags.contains("character_tunnelman"),
                expected == Character::TunnelMan,
                "{room:?} {initial:?}"
            );
            assert_eq!(game.player.facing, if door == "oXChange" { -1 } else { 1 });
        }
    }
}
