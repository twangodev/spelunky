//! Shared native/web sandbox menu. Choices last for this application session.
use crate::{atlas::Atlas, renderer::DrawQuad, scene};
use spelunky_core::{Game, cheats::Cheats};
use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct CheatUi {
    selected: usize,
    pub cheats: Cheats,
}

impl CheatUi {
    /// Returns true when the player closes the menu.
    pub fn key(&mut self, key: KeyCode, game: &mut Game) -> bool {
        match key {
            KeyCode::Escape | KeyCode::F6 => return true,
            KeyCode::ArrowUp => self.selected = (self.selected + 7) % 8,
            KeyCode::ArrowDown => self.selected = (self.selected + 1) % 8,
            KeyCode::Enter
            | KeyCode::NumpadEnter
            | KeyCode::Space
            | KeyCode::KeyZ
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight => {
                match self.selected {
                    0 => self.cheats.infinite_health = !self.cheats.infinite_health,
                    1 => self.cheats.infinite_bombs = !self.cheats.infinite_bombs,
                    2 => self.cheats.infinite_ropes = !self.cheats.infinite_ropes,
                    3 => self.cheats.infinite_money = !self.cheats.infinite_money,
                    4 => self.cheats.free_shops = !self.cheats.free_shops,
                    5 => Cheats::refill(game),
                    6 => Cheats::calm_shopkeepers(game),
                    _ => return true,
                }
                self.cheats.apply(game);
                game.draw_view = None;
            }
            _ => {}
        }
        false
    }

    pub fn draw(&self, atlas: &Atlas, quads: &mut Vec<DrawQuad>) {
        let gold = [1.0, 0.81, 0.32, 1.0];
        let white = [0.94, 0.95, 0.98, 1.0];
        scene::rect(quads, [8.0, 8.0, 304.0, 224.0], [0.03, 0.04, 0.07, 0.97]);
        scene::text(quads, atlas, "CHEATS", 120.0, 20.0, 1.5, gold);
        let c = &self.cheats;
        let rows = [
            ("INFINITE HEALTH", Some(c.infinite_health)),
            ("INFINITE BOMBS", Some(c.infinite_bombs)),
            ("INFINITE ROPES", Some(c.infinite_ropes)),
            ("INFINITE MONEY", Some(c.infinite_money)),
            ("FREE SHOPS", Some(c.free_shops)),
            ("HEAL + REFILL EVERYTHING", None),
            ("CALM SHOPKEEPERS", None),
            ("BACK", None),
        ];
        for (index, (label, enabled)) in rows.into_iter().enumerate() {
            let y = 50.0 + index as f32 * 17.0;
            let color = if index == self.selected {
                scene::rect(quads, [16.0, y - 3.0, 288.0, 14.0], [0.18, 0.2, 0.27, 1.0]);
                gold
            } else {
                white
            };
            scene::text(quads, atlas, label, 24.0, y, 0.9, color);
            if let Some(on) = enabled {
                scene::text(
                    quads,
                    atlas,
                    if on { "ON" } else { "OFF" },
                    265.0,
                    y,
                    0.9,
                    color,
                );
            }
        }
        scene::text(
            quads,
            atlas,
            "ARROWS: SELECT   ENTER: TOGGLE / APPLY",
            20.0,
            197.0,
            0.75,
            white,
        );
        scene::text(
            quads,
            atlas,
            "ESC / F6: BACK   SESSION ONLY",
            20.0,
            211.0,
            0.75,
            white,
        );
        scene::text(
            quads,
            atlas,
            "CHEATED RUNS DO NOT SET HIGH SCORES",
            20.0,
            223.0,
            0.7,
            gold,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cheat_menu_applies_toggles_and_actions_and_returns_without_gameplay() {
        let mut game = Game::new(7, spelunky_core::RulesPreset::Classic);
        let mut menu = CheatUi::default();
        let tick = game.tick;
        assert!(!menu.key(KeyCode::Enter, &mut game));
        assert!(game.player.cheat_invulnerable);
        menu.key(KeyCode::Enter, &mut game);
        assert!(!game.player.cheat_invulnerable);
        menu.key(KeyCode::ArrowUp, &mut game);
        assert!(menu.key(KeyCode::Enter, &mut game));
        assert_eq!(game.tick, tick);
        assert!(menu.key(KeyCode::Escape, &mut game));
    }
}
