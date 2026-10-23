//! Modern transition screen using Classic's staged costs and donation controls.
//! The completed level remains frozen until the player continues.
use spelunky_core::{Game, InputFrame, progression};
use std::collections::BTreeMap;

pub struct TransitionDonation {
    pub stage: u8,
    pub destination: u32,
    pub amount: u32,
    up_held: u32,
    down_held: u32,
    previous_up: bool,
    previous_down: bool,
}
impl TransitionDonation {
    pub fn begin(game: &Game) -> Option<Self> {
        let offer = progression::tunnel_offer(game)?;
        Some(Self {
            stage: offer.stage,
            destination: offer.destination,
            amount: 0,
            up_held: 0,
            down_held: 0,
            previous_up: false,
            previous_down: false,
        })
    }
    pub fn remaining(&self, game: &Game) -> u32 {
        game.data
            .get(&format!("tunnel{}_remaining", self.stage))
            .copied()
            .unwrap_or(f64::from(self.stage) * 100_000.)
            .clamp(0., f64::from(self.stage) * 100_000.) as u32
    }
    pub fn complete(&self, game: &Game) -> bool {
        game.flags
            .contains(&format!("shortcut_{}", self.destination))
    }
    /// Called at the original 30 Hz cadence. Press changes selection by $100;
    /// hold repeats after 20 ticks and accelerates after 100 ticks.
    pub fn tick(&mut self, game: &mut Game, input: &InputFrame) -> u32 {
        let Some(offer) = progression::tunnel_offer(game).filter(|offer| offer.stage == self.stage)
        else {
            self.amount = 0;
            return 0;
        };
        // oTransition handles fresh press edges independently from oTunnelMan's
        // held-repeat counters, including presses from a second device.
        let up_pressed = input.up_pressed || (input.up && !self.previous_up);
        let down_pressed = input.down_pressed || (input.down && !self.previous_down);
        if up_pressed {
            self.amount = self.amount.saturating_add(100);
        } else if down_pressed {
            self.amount = self.amount.saturating_sub(100);
        }
        // Source oTransition clamps its fresh selection before the later
        // oTunnelMan held-repeat Step, including an opposing short tap.
        self.amount = self.amount.min(game.player.money).min(offer.remaining);
        if input.up {
            self.up_held = self.up_held.saturating_add(1);
            self.down_held = 0;
            if self.up_held > 20 {
                self.amount =
                    self.amount
                        .saturating_add(if self.up_held > 100 { 1_000 } else { 100 });
            }
        } else if input.down {
            self.down_held = self.down_held.saturating_add(1);
            self.up_held = 0;
            if self.down_held > 20 {
                self.amount =
                    self.amount
                        .saturating_sub(if self.down_held > 100 { 1_000 } else { 100 });
            }
        } else {
            self.up_held = 0;
            self.down_held = 0;
        }
        self.previous_up = input.up;
        self.previous_down = input.down;
        self.amount = self.amount.min(game.player.money).min(offer.remaining);
        if (input.attack_pressed || input.pay_pressed) && self.amount > 0 {
            let paid = progression::donate_to_tunnel(game, self.amount);
            self.amount = 0;
            paid
        } else {
            0
        }
    }
}

pub fn hydrate(game: &mut Game, remaining: &BTreeMap<String, u32>) {
    for (key, amount) in remaining {
        // Saved balances initialize a fresh run. When hub navigation reuses a
        // live Game, its unsaved source globals remain authoritative until the
        // next high-score update instead of being replaced by an older file.
        game.data
            .entry(key.clone())
            .or_insert_with(|| f64::from(*amount));
    }
}
pub fn progress(game: &Game) -> BTreeMap<String, u32> {
    let mut remaining = BTreeMap::new();
    for (stage, destination) in [(1, 5), (2, 9), (3, 13)] {
        let key = format!("tunnel{stage}_remaining");
        if game.flags.contains(&format!("shortcut_{destination}")) {
            remaining.insert(key, 0);
        } else if let Some(value) = game.data.get(&key).filter(|value| value.is_finite()) {
            remaining.insert(key, value.clamp(0., f64::from(stage) * 100_000.) as u32);
        }
    }
    remaining
}

#[cfg(test)]
mod tests {
    use super::*;
    use spelunky_core::{RulesPreset, RunState, Vec2};
    fn exited(level: u32, money: u32) -> Game {
        let mut game = Game::shell(99, RulesPreset::Classic);
        game.world.level = level;
        game.state = RunState::Transition;
        game.player.money = money;
        game
    }
    #[test]
    fn completed_level_exposes_source_donation_controls_and_preserves_transition() {
        let mut game = exited(4, 500);
        game.flags.insert("carry:oShotgun".into());
        let position = Vec2::new(75., 90.);
        game.player.pos = position;
        let mut ui = TransitionDonation::begin(&game).expect("first eligible exit exposes donor");
        assert_eq!(
            (ui.stage, ui.destination, ui.remaining(&game)),
            (1, 5, 100_000)
        );
        ui.tick(
            &mut game,
            &InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(ui.amount, 100);
        assert_eq!(
            ui.tick(
                &mut game,
                &InputFrame {
                    attack_pressed: true,
                    ..Default::default()
                }
            ),
            100
        );
        assert_eq!(game.player.money, 400);
        assert_eq!(ui.remaining(&game), 99_900);
        assert_eq!(game.state, RunState::Transition);
        assert_eq!(game.player.pos, position);
        assert!(game.flags.contains("carry:oShotgun"));
        assert_eq!(game.tick, 0);
        let mut save = crate::storage::SaveData::default();
        save.tunnel_remaining = progress(&game);
        let decoded = crate::storage::SaveData::from_json(&save.to_json().unwrap()).unwrap();
        let mut reloaded = exited(4, 200);
        hydrate(&mut reloaded, &decoded.tunnel_remaining);
        assert_eq!(
            TransitionDonation::begin(&reloaded)
                .unwrap()
                .remaining(&reloaded),
            99_900
        );
    }

    #[test]
    fn source_first_encounter_marker_hides_the_donation_screen() {
        let mut game = exited(4, 500);
        game.data.insert("tunnel1_remaining".into(), 100001.);
        assert!(TransitionDonation::begin(&game).is_none());
    }
    #[test]
    fn saved_balances_fill_missing_run_state_without_overwriting_live_donations() {
        let saved = BTreeMap::from([
            ("tunnel1_remaining".into(), 100_000),
            ("tunnel2_remaining".into(), 200_000),
        ]);
        let mut game = exited(4, 100_000);
        game.data.insert("tunnel1_remaining".into(), 99_000.);

        hydrate(&mut game, &saved);

        assert_eq!(game.data["tunnel1_remaining"], 99_000.);
        assert_eq!(game.data["tunnel2_remaining"], 200_000.);
    }
    #[test]
    fn exit_input_is_cleared_before_transition_selection() {
        let mut input = crate::input::InputState::default();
        input.key(winit::keyboard::KeyCode::ArrowUp, true, false);
        input.clear_gameplay();
        let mut game = exited(4, 100_000);
        let mut ui = TransitionDonation::begin(&game).unwrap();
        ui.tick(&mut game, &input.take_tick());
        assert_eq!(ui.amount, 0);
        assert_eq!(game.player.money, 100_000);
    }
    #[test]
    fn selector_accelerates_clamps_and_stops_after_unlock() {
        let mut game = exited(4, 200_000);
        let mut ui = TransitionDonation::begin(&game).unwrap();
        for _ in 0..20 {
            ui.tick(
                &mut game,
                &InputFrame {
                    up: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(ui.amount, 100);
        ui.tick(
            &mut game,
            &InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(ui.amount, 200);
        for _ in 21..101 {
            ui.tick(
                &mut game,
                &InputFrame {
                    up: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(ui.amount, 9_100);
        for _ in 0..100 {
            ui.tick(
                &mut game,
                &InputFrame {
                    up: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(ui.amount, 100_000);
        assert_eq!(
            ui.tick(
                &mut game,
                &InputFrame {
                    pay_pressed: true,
                    ..Default::default()
                }
            ),
            100_000
        );
        assert!(ui.complete(&game));
        assert_eq!(
            ui.tick(
                &mut game,
                &InputFrame {
                    up: true,
                    pay_pressed: true,
                    ..Default::default()
                }
            ),
            0
        );
    }
    #[test]
    fn shortcut_offers_require_actual_boundaries_and_earlier_work() {
        assert!(TransitionDonation::begin(&exited(3, 999_999)).is_none());
        assert!(TransitionDonation::begin(&exited(8, 999_999)).is_none());
        let mut game = exited(8, 999_999);
        game.flags.insert("shortcut_5".into());
        let offer = TransitionDonation::begin(&game).unwrap();
        assert_eq!((offer.stage, offer.destination), (2, 9));
        game.world.level = 12;
        assert!(TransitionDonation::begin(&game).is_none());
        game.flags.insert("shortcut_9".into());
        assert_eq!(TransitionDonation::begin(&game).unwrap().destination, 13);
    }
}
