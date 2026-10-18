//! Source ending and inter-level cinematic controllers.
use crate::model::{Entity, Game, InputFrame, Rect, RunState, Vec2, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndingKind {
    Campaign,
    Custom,
    TitleCredits,
    Transition,
    HubPortal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndingStage {
    Exit,
    Chamber,
    Volcano,
    Landing,
    Transition,
    Summary,
    Credits,
    Complete,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndingState {
    pub kind: EndingKind,
    pub stage: EndingStage,
    pub stage_tick: u64,
    pub draw_status: u8,
    pub money_count: u32,
    pub fade_level: f64,
    #[serde(default)]
    pub previous_fade_level: f64,
    pub fade_in: bool,
    pub fade_out: bool,
    pub credits_page: u8,
    pub source_time_ms: u64,
    pub shortcut_used: bool,
    pub bonus_awarded: bool,
    pub record_ready: bool,
    pub draw_loot: i8,
    pub is_loot: bool,
    pub is_kills: bool,
    pub level_time_ms: u64,
    pub completed_level: u32,
    pub completed_name: String,
    pub music: String,
    /// Source playMusic argument1; false requests one playback, not a loop.
    #[serde(default = "EndingState::default_music_looping")]
    pub music_looping: bool,
    #[serde(default)]
    pub portal: Option<crate::hub::HubPortal>,
    alarms: [i32; 12],
    actor_alarms: BTreeMap<u64, [i32; 12]>,
    #[serde(default)]
    animation_order: Vec<u64>,
    bonus_scheduled: bool,
    hurry_up: bool,
    draw_pos: Vec2,
    level_money: u32,
    pickup_item: String,
    scrolling: bool,
    scroll_started: bool,
    previous_up: bool,
    previous_down: bool,
    transition_tunnel: bool,
}
impl EndingState {
    fn default_music_looping() -> bool {
        true
    }
    pub fn level_money(&self) -> u32 {
        self.level_money
    }
    pub fn new(game: &Game, kind: EndingKind, stage: EndingStage) -> Self {
        Self {
            kind,
            stage,
            stage_tick: 0,
            draw_status: 0,
            money_count: 0,
            fade_level: 0.,
            previous_fade_level: 0.,
            fade_in: false,
            fade_out: false,
            credits_page: 0,
            source_time_ms: game.data.get("time").copied().unwrap_or(0.).max(0.) as u64,
            shortcut_used: game.flags.contains("used_shortcut")
                || game.flags.contains("usedShortcut"),
            bonus_awarded: false,
            record_ready: false,
            draw_loot: -2,
            is_loot: false,
            is_kills: false,
            level_time_ms: game.data.get("xtime").copied().unwrap_or(0.).max(0.) as u64,
            completed_level: game.world.level,
            completed_name: if game.flags.contains("custom") {
                game.world.name.clone()
            } else {
                String::new()
            },
            music: String::new(),
            music_looping: true,
            portal: None,
            alarms: [-1; 12],
            actor_alarms: BTreeMap::new(),
            animation_order: Vec::new(),
            bonus_scheduled: false,
            hurry_up: false,
            draw_pos: Vec2::new(100., 83.),
            level_money: game.data.get("xmoney").copied().unwrap_or(0.).max(0.) as u32,
            pickup_item: game
                .player
                .held
                .and_then(|id| game.world.entity(id))
                .map(|e| e.object.clone())
                .or_else(|| {
                    game.flags
                        .iter()
                        .find_map(|f| f.strip_prefix("carry:").map(str::to_owned))
                })
                .unwrap_or_default(),
            scrolling: false,
            scroll_started: false,
            previous_up: false,
            previous_down: false,
            transition_tunnel: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Area, RulesPreset};
    fn game() -> Game {
        let mut g = Game::shell(19, RulesPreset::Classic);
        g.world.area = Area::Boss;
        g.world.level = 16;
        g.player.money = 150_000;
        g.data.insert("kills".into(), 120.);
        g.data.insert("damsels".into(), 8.);
        g.data.insert("time".into(), 549_000.);
        g
    }

    #[test]
    fn treasure_source_collision_lines_round_each_fractional_endpoint() {
        let mut game = Game::shell(19, RulesPreset::Classic);
        game.world = World::new(20, 16, Area::Boss, 16);
        game.world.set_tile(7, 4, "oBlock");
        let mut ending = EndingState::new(&game, EndingKind::Campaign, EndingStage::Landing);
        let mut treasure = Entity::new(900, "oBigTreasure", Vec2::new(80.25, 81.0));
        treasure.vel = Vec2::new(0.25, -1.0);
        treasure.data.insert("gravity".into(), 0.6);

        treasure_step(&mut game, &mut ending, &mut treasure);

        assert_eq!(treasure.pos, Vec2::new(80.5, 80.0));
        assert!(
            (treasure.vel.y - 0.32).abs() < 1e-12,
            "rounded source top line hits x=112"
        );
    }

    #[test]
    fn actual_tutorial_rock_pickup_clears_transition_state_and_all_dummy_copies() {
        let mut game = Game::tutorial(73, RulesPreset::Classic);
        // Controlled player placement at the existing tutorial Rock. Pickup,
        // both original doors, and the complete exit animation stay real.
        game.player.pos = Vec2::new(648., 264.);
        game.player.previous_pos = game.player.pos;
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        game.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        let held = game
            .player
            .held
            .expect("controller Down+Attack picks up the authored Rock");
        assert_eq!(game.world.entity(held).unwrap().object, "oRock");
        assert_eq!(game.player.pickup_item_type, "Rock");
        game.player.equipment.insert("Cape".into());
        game.player.pos = Vec2::new(56., 248.);
        game.player.previous_pos = game.player.pos;
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.held, Some(held));
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        let exit = game
            .ending
            .as_ref()
            .expect("original lower exit starts animation");
        assert_eq!(exit.stage, EndingStage::Exit);
        assert!(matches!(exit.pickup_item.as_str(), "Rock" | "oRock"));
        for _ in 0..100 {
            game.step(&InputFrame::default());
            if game.world.name == "rTransition1" {
                break;
            }
        }
        assert_eq!(game.world.name, "rTransition1");
        let transition = game.ending.as_ref().unwrap();
        assert_eq!(transition.stage, EndingStage::Transition);
        assert!(transition.pickup_item.is_empty());
        let dummies: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|actor| matches!(actor.object.as_str(), "oPDummy" | "oPDummy2" | "oPDummy6"))
            .collect();
        assert!(
            !dummies.is_empty(),
            "the authored transition dummy is present"
        );
        for dummy in dummies {
            assert_eq!(dummy.text.get("pickup_item").map(String::as_str), Some(""));
        }
        assert!(
            game.world
                .entities
                .iter()
                .all(|actor| actor.text.get("pickup_item").is_none_or(String::is_empty))
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|actor| actor.object == "oCape")
                .count(),
            1,
            "source Cape Create precedes global equipment clear"
        );
        assert!(!game.player.has("Cape"));
        assert!(game.player.held.is_none());
        assert!(!game.flags.iter().any(|flag| flag.starts_with("carry:")));
    }

    fn until(g: &mut Game, limit: usize, predicate: impl Fn(&EndingState) -> bool) {
        for _ in 0..limit {
            if predicate(g.ending.as_ref().unwrap()) {
                return;
            }
            g.step(&InputFrame::default());
        }
        panic!(
            "cinematic failed to reach source event: {:?}; actor states {:?}",
            g.ending,
            g.world
                .entities
                .iter()
                .filter(|e| matches!(
                    e.object.as_str(),
                    "oPDummy" | "oPDummy2" | "oBigTreasure" | "oLavaSpray"
                ))
                .map(|e| (&e.object, e.pos, e.vel, e.state))
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn natural_campaign_chamber_volcano_landing_bonus_and_credits_record_boundary() {
        let mut g = game();
        begin_exit(&mut g, EndingKind::Campaign);
        until(&mut g, 100, |s| s.stage == EndingStage::Chamber);
        assert_eq!(g.player.money, 150_000);
        until(&mut g, 1000, |s| s.stage == EndingStage::Volcano);
        until(&mut g, 300, |s| s.stage == EndingStage::Landing);
        // Treasure (resource 603) runs before the landing dummy (612),
        // although the dummy's Alarm 0 created it later in creation order.
        for _ in 0..250 {
            g.step(&InputFrame::default());
            if g.world.entities.iter().any(|e| e.object == "oBigTreasure") {
                break;
            }
        }
        let treasure = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oBigTreasure")
            .unwrap();
        assert_eq!(treasure.pos.y, -32.);
        assert_eq!(treasure.vel.y, 0.6);
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oPDummy2")
                .unwrap()
                .state,
            2,
            "the dummy waits for the treasure to land instead of jumping on its birth frame"
        );
        until(&mut g, 1000, |s| {
            s.stage == EndingStage::Summary && s.draw_status == 3
        });
        assert_eq!(g.player.money, 150_000);
        assert_eq!(g.ending.as_ref().unwrap().music, "musVictory");
        assert!(!g.ending.as_ref().unwrap().music_looping);
        assert!(!g.ending.as_ref().unwrap().record_ready);
        until(&mut g, 500, |s| s.draw_status == 7);
        assert_eq!(g.player.money, 200_000);
        assert!(g.ending.as_ref().unwrap().bonus_awarded);
        assert_eq!(g.state, RunState::Ending);
        tick(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        until(&mut g, 20, |s| s.draw_status == 8);
        tick(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Credits);
        assert!(!g.ending.as_ref().unwrap().record_ready);
        until(&mut g, 300, |s| s.music == "musCredits");
        assert!(!g.ending.as_ref().unwrap().music_looping);
        until(&mut g, 3000, |s| s.stage == EndingStage::Complete);
        assert_eq!(g.state, RunState::Won);
        assert!(g.ending.as_ref().unwrap().record_ready);
        assert_eq!(g.player.money, 200_000);
        assert_eq!(g.data["kills"], 120.);
        assert_eq!(g.data["damsels"], 8.);
        assert!(
            crate::hub::record_unlocks(g.player.money, Some(549.), 120, 8).contains("unlock_sun")
        );
    }
    #[test]
    fn chamber_skip_is_gated_and_cannot_bypass_landing_or_bonus() {
        let mut g = game();
        begin_exit(&mut g, EndingKind::Campaign);
        until(&mut g, 100, |s| s.stage == EndingStage::Chamber);
        for _ in 0..50 {
            tick(
                &mut g,
                &InputFrame {
                    attack_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Chamber);
        }
        tick(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Landing);
        assert_eq!(g.player.money, 150_000);
        until(&mut g, 1000, |s| s.draw_status == 7);
        for _ in 0..150 {
            tick(&mut g, &InputFrame::default());
        }
        assert_eq!(g.player.money, 200_000);
        assert!(!g.ending.as_ref().unwrap().record_ready);
    }
    #[test]
    fn custom_bonus_and_summary_leave_without_campaign_record() {
        let mut g = game();
        g.flags.insert("custom".into());
        begin_custom(&mut g);
        for _ in 0..49 {
            tick(
                &mut g,
                &InputFrame {
                    attack_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.ending.as_ref().unwrap().draw_status, 0);
        }
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.ending.as_ref().unwrap().draw_status, 1);
        assert_eq!(g.ending.as_ref().unwrap().music, "musVictory");
        assert!(!g.ending.as_ref().unwrap().music_looping);
        until(&mut g, 500, |s| s.draw_status == 7);
        assert_eq!(g.player.money, 200_000);
        tick(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        until(&mut g, 20, |s| s.draw_status == 8);
        tick(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Complete);
        assert!(!g.ending.as_ref().unwrap().record_ready);
        assert_eq!(g.state, RunState::Ending);
        assert!(!g.events.contains(&crate::model::GameEvent::Won));
    }
    #[test]
    fn exit_uses_existing_selected_animation_phase_not_fixed_delay() {
        for character in [
            crate::characters::Character::Explorer,
            crate::characters::Character::Damsel,
            crate::characters::Character::TunnelMan,
        ] {
            let mut g = game();
            crate::characters::apply(&mut g, character);
            g.player.sprite = selected_sprite(&g, "sRunLeft", "sDamselRunL", "sTunnelRunL").into();
            g.player.animation = 3.5;
            begin_exit(&mut g, EndingKind::Campaign);
            let frames = crate::catalog::sprite_frame_count(&g.player.sprite);
            let ticks = ((frames as f64 - 3.5) / 0.5).ceil() as usize;
            for _ in 0..ticks.saturating_sub(1) {
                g.step(&InputFrame::default());
                assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Exit);
            }
            g.step(&InputFrame::default());
            assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Chamber);
        }
    }
    #[test]
    fn natural_right_and_up_walk_preserves_prior_phase_before_exit_animation() {
        let mut g = Game::shell(23, RulesPreset::Classic);
        g.world = World::new(40, 20, Area::Mines, 1);
        for x in 0..40 {
            g.world.set_tile(x, 3, "oBlock");
        }
        g.player.pos = Vec2::new(40., 40.);
        g.player.previous_pos = g.player.pos;
        g.world.spawn("oExit", Vec2::new(280., 32.));
        let input = InputFrame {
            right: true,
            up: true,
            ..Default::default()
        };
        let mut old_phase = 0.;
        let mut old_frames = 0.;
        for _ in 0..250 {
            old_phase = g.player.animation;
            old_frames = crate::catalog::sprite_frame_count(&g.player.sprite) as f64;
            g.step(&input);
            if g.state == RunState::Ending {
                break;
            }
        }
        assert_eq!(g.state, RunState::Ending);
        assert!(
            old_phase >= 0. && old_phase < old_frames,
            "the canonical endframe phase already loops the old running sprite"
        );
        let phase = g.player.animation;
        let run_frames = crate::catalog::sprite_frame_count("sRunLeft") as f64;
        assert!(
            (phase - (old_phase.rem_euclid(old_frames) + 0.5)).abs() < 1e-9,
            "entry frame advances the new Exit, rather than the old run sprite"
        );
        assert!(phase >= 0. && phase < run_frames);
        let exit_frames = crate::catalog::sprite_frame_count(&g.player.sprite) as f64;
        let duration = ((exit_frames - phase) / 0.5).ceil() as usize;
        assert!(duration > 1);
        for _ in 0..duration - 1 {
            g.step(&input);
            assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Exit);
        }
        g.step(&input);
        assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Transition);
    }
    #[test]
    fn custom_ending_preserves_completed_link_context_after_transition_room() {
        let mut g = game();
        g.flags.insert("custom".into());
        g.world.name = "last playable level".into();
        let previous = EndingState::new(&g, EndingKind::Transition, EndingStage::Complete);
        g.ending = Some(previous);
        g.world.name = "rTransition1".into();
        g.world.level = 1;
        g.data.insert("time".into(), 0.);
        begin_custom(&mut g);
        let s = g.ending.as_ref().unwrap();
        assert_eq!(s.completed_name, "last playable level");
        assert_eq!(s.completed_level, 16);
        assert_eq!(s.source_time_ms, 549_000);
        assert_eq!(g.world.name, "rEndCustom");
    }
    #[test]
    fn title_rope_ascent_fade_night_credits_and_quit_never_record_a_win() {
        let mut g = Game::shell(21, RulesPreset::Classic);
        g.player.pos = Vec2::new(176., 31.);
        begin_title_credits(&mut g);
        assert!(g.world.entities.iter().any(|e| e.object == "oPDummy5"));
        for _ in 0..63 {
            tick(
                &mut g,
                &InputFrame {
                    attack_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Exit);
        }
        until(&mut g, 30, |s| s.stage == EndingStage::Credits);
        assert_eq!(g.world.name, "rCredits1");
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.object == "oDesertScroll" && e.sprite == "sDesertNight")
        );
        until(&mut g, 3000, |s| s.stage == EndingStage::Complete);
        assert!(g.flags.contains("app_exit_requested"));
        assert!(!g.ending.as_ref().unwrap().record_ready);
        assert_eq!(g.state, RunState::Ending);
    }
    #[test]
    fn alarm_number_passes_decrement_later_alarms_set_by_earlier_callbacks() {
        let mut g = game();
        begin_custom(&mut g);
        for _ in 0..50 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.ending.as_ref().unwrap().draw_status, 1);
        assert_eq!(g.ending.as_ref().unwrap().alarms[1], 49);
        for _ in 0..48 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.ending.as_ref().unwrap().draw_status, 1);
        g.step(&InputFrame::default());
        assert_eq!(g.ending.as_ref().unwrap().draw_status, 2);
        assert_eq!(g.ending.as_ref().unwrap().alarms[2], 49);
    }
    #[test]
    fn credits_controller_interleaves_with_scroll_and_camel_resources() {
        for kind in [EndingKind::TitleCredits, EndingKind::Campaign] {
            let mut g = game();
            let mut s = EndingState::new(&g, kind, EndingStage::Credits);
            enter(
                &mut g,
                &mut s,
                EndingStage::Credits,
                if kind == EndingKind::TitleCredits {
                    "rCredits1"
                } else {
                    "rCredits2"
                },
            );
            s.fade_in = false;
            s.alarms = [-1; 12];
            let camel = spawn(&mut g, &mut s, "oCamel", Vec2::new(161., 144.));
            let desert = g
                .world
                .entities
                .iter()
                .find(|e| e.object == "oDesertScroll")
                .unwrap()
                .id;
            let x = g.world.entity(desert).unwrap().pos.x;
            g.ending = Some(s);
            g.state = RunState::Ending;
            g.step(&InputFrame::default());
            assert_eq!(g.world.entity(camel).unwrap().pos.x, 160.);
            assert_eq!(g.ending.as_ref().unwrap().music, "musCredits");
            assert!(!g.ending.as_ref().unwrap().music_looping);
            assert_eq!(g.world.entity(desert).unwrap().pos.x, x);
            assert_eq!(
                g.ending.as_ref().unwrap().scrolling,
                kind == EndingKind::Campaign
            );
            g.step(&InputFrame::default());
            assert!(g.ending.as_ref().unwrap().scrolling);
            assert_eq!(
                g.world.entity(desert).unwrap().pos.x,
                x + if kind == EndingKind::Campaign { 1. } else { 0. }
            );
            g.step(&InputFrame::default());
            assert_eq!(
                g.world.entity(desert).unwrap().pos.x,
                x + if kind == EndingKind::Campaign { 2. } else { 1. }
            );
        }
    }
    #[test]
    fn credits_skip_uses_authored_keyboard_edges_or_held_attack_only() {
        for (kind, room) in [
            (EndingKind::TitleCredits, "rCredits1"),
            (EndingKind::Campaign, "rCredits2"),
        ] {
            for (input, expected) in [
                (
                    InputFrame {
                        start_pressed: true,
                        ..Default::default()
                    },
                    false,
                ),
                (
                    InputFrame {
                        attack_pressed: true,
                        ..Default::default()
                    },
                    false,
                ),
                (
                    InputFrame {
                        attack: true,
                        ..Default::default()
                    },
                    true,
                ),
                (
                    InputFrame {
                        credits_keyboard_skip_pressed: true,
                        ..Default::default()
                    },
                    true,
                ),
            ] {
                let mut g = game();
                let mut s = EndingState::new(&g, kind, EndingStage::Credits);
                enter(&mut g, &mut s, EndingStage::Credits, room);
                s.fade_in = false;
                s.fade_level = 0.;
                s.alarms = [-1; 12];
                spawn(&mut g, &mut s, "oCamel", Vec2::new(200., 144.));
                g.ending = Some(s);
                g.state = RunState::Ending;

                g.step(&input);

                assert_eq!(
                    g.ending.as_ref().unwrap().fade_out,
                    expected,
                    "{room} should match keyboard Enter/Escape or held checkAttack"
                );
            }
        }
    }
    #[test]
    fn title_credits_intro_backdrop_runs_its_authored_step() {
        let mut g = game();
        let mut s = EndingState::new(&g, EndingKind::TitleCredits, EndingStage::Credits);
        enter(&mut g, &mut s, EndingStage::Credits, "rCredits1");
        s.fade_in = false;
        s.alarms = [-1; 12];
        s.alarms[0] = 1;
        let backdrop = g
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oIntroBG")
            .expect("oCredits1 Create adds the authored intro backdrop")
            .id;
        assert_eq!(g.world.entity(backdrop).unwrap().pos.x, -120.);
        g.ending = Some(s);
        g.state = RunState::Ending;

        // oCredits1 Alarm 0 creates the Camel, then the fresh Step phase runs
        // oIntroBG's lower-resource Step and pins x to view_xview + xOff.
        g.step(&InputFrame::default());

        assert!(
            g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oCamel")
        );
        assert_eq!(g.world.entity(backdrop).unwrap().pos.x, 0.);

        g.camera.x = 8.;
        g.step(&InputFrame::default());
        assert_eq!(
            g.world.entity(backdrop).unwrap().pos.x,
            8.,
            "the camel branch leaves xOff unchanged before pinning to the view"
        );
        g.world.entities.retain(|entity| entity.object != "oCamel");
        g.step(&InputFrame::default());
        assert_eq!(
            g.world.entity(backdrop).unwrap().pos.x,
            7.98,
            "without a camel, a nonzero view decrements xOff by 0.02"
        );
    }
    #[test]
    fn leaving_r_end_runs_its_room_end_shake_reset() {
        for (source_room, expected_shake) in [("rEnd", 0.), ("rEnd2", 17.)] {
            let mut g = game();
            g.world.name = source_room.into();
            g.data.insert("shake".into(), 17.);
            let mut s = EndingState::new(&g, EndingKind::Campaign, EndingStage::Landing);

            enter(&mut g, &mut s, EndingStage::Landing, "rEnd3");

            assert_eq!(
                g.data["shake"], expected_shake,
                "only oEnd's Room End clears the source shake global"
            );
        }
    }
    #[test]
    fn higher_resource_newborn_steps_and_animation_follow_the_source_phases() {
        let mut g = game();
        begin_custom(&mut g);
        let mut s = g.ending.take().unwrap();
        let kiss = spawn(&mut g, &mut s, "oDamselKiss", Vec2::new(184., 184.));
        let actor = g.world.entity_mut(kiss).unwrap();
        actor.sprite = "sDamselKissL".into();
        actor.animation = 7.;
        g.ending = Some(s);
        g.step(&InputFrame::default());
        let heart = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oHeart")
            .unwrap();
        assert_eq!(
            heart.pos,
            Vec2::new(176., 175.),
            "Heart624 steps on its birth frame after Kiss623"
        );
        assert_eq!(g.world.entity(kiss).unwrap().animation, 7.5);

        let mut g = Game::shell(29, RulesPreset::Classic);
        g.player.equipment.insert("Cape".into());
        begin_exit(&mut g, EndingKind::Transition);
        until(&mut g, 100, |s| s.stage == EndingStage::Transition);
        let dummy = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oPDummy")
            .unwrap()
            .id;
        let actor = g.world.entity_mut(dummy).unwrap();
        actor.state = 99;
        actor.sprite = "sPExit".into();
        actor.animation = crate::catalog::sprite_frame_count("sPExit") as f64 - 1.;
        let expected = Vec2::new(actor.pos.x, actor.pos.y + 4.);
        g.step(&InputFrame::default());
        assert!(g.world.entity(dummy).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oCape")
                .unwrap()
                .pos,
            expected,
            "Cape Step runs before the later global AnimationEnd phase destroys Dummy"
        );
        g.step(&InputFrame::default());
        assert!(!g.world.entities.iter().any(|e| e.object == "oCape"));
    }
    #[test]
    fn transition_tally_hurry_and_kiss_reward_remain_independent_of_animation() {
        let mut g = Game::shell(22, RulesPreset::Classic);
        g.world.level = 2;
        g.data.insert("gold".into(), 2.);
        g.data.insert("bats".into(), 1.);
        g.data.insert("xmoney".into(), 350.);
        g.data.insert("pending_damsel_kisses".into(), 3.);
        begin_exit(&mut g, EndingKind::Transition);
        until(&mut g, 100, |s| s.stage == EndingStage::Transition);
        tick(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        for _ in 0..28 {
            tick(&mut g, &InputFrame::default());
            assert_eq!(g.ending.as_ref().unwrap().draw_loot, -2);
        }
        until(&mut g, 100, |s| s.draw_loot == 2 && s.money_count == 350);
        assert!(g.ending.as_ref().unwrap().is_loot);
        assert!(g.ending.as_ref().unwrap().is_kills);
        assert_eq!(g.player.health, 4.0);
        assert_eq!(g.data["pending_damsel_kisses"], 3.);
        tick(
            &mut g,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.state, RunState::Transition);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.object == "oDamselKiss" && e.value("kissed") == 0.)
        );
        g.next_level();
        assert_eq!(g.player.health, 5.0);
    }
    #[test]
    fn cinematic_particles_use_source_motion_and_event_lifetimes() {
        let mut g = game();
        begin_custom(&mut g);
        let mut s = g.ending.take().unwrap();
        let heart = spawn(&mut g, &mut s, "oHeart", Vec2::new(200., 80.));
        let poof = spawn(&mut g, &mut s, "oPoof", Vec2::new(220., 80.));
        let flame = spawn(&mut g, &mut s, "oVolcanoFlame", Vec2::new(240., 100.));
        let initial = g.world.entity(flame).unwrap().clone();
        assert!((-4.0..=4.0).contains(&initial.vel.x));
        assert!((-3.0..=-1.0).contains(&initial.vel.y));
        assert!((0.1..=0.6 + f64::EPSILON).contains(&initial.value("gravity")));
        g.ending = Some(s);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(heart).unwrap().pos.y, 79.);
        let moved = g.world.entity(flame).unwrap();
        assert_eq!(
            moved.pos,
            Vec2::new(initial.pos.x + initial.vel.x, initial.pos.y + initial.vel.y)
        );
        assert_eq!(moved.vel.y, initial.vel.y + initial.value("gravity"));
        for _ in 1..14 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(poof).is_some());
        g.step(&InputFrame::default());
        assert!(g.world.entity(poof).is_none());
        for _ in 15..29 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(heart).unwrap().pos.y, 51.);
        g.step(&InputFrame::default());
        assert!(g.world.entity(heart).is_none());
    }
    fn activate_fixture_exit(g: &mut Game) {
        for x in 0..g.world.width {
            g.world.set_tile(x, 3, "oBlock");
        }
        g.player.pos = Vec2::new(40., 40.);
        g.player.previous_pos = g.player.pos;
        g.world.spawn("oExit", Vec2::new(32., 32.));
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Exit);
    }
    #[test]
    fn cinematic_animation_keeps_stable_draw_depth_order_across_frames() {
        let mut g = game();
        begin_custom(&mut g);
        let first = g.world.spawn("oPoof", Vec2::new(40., 40.));
        let later = g.world.spawn("oPoof", Vec2::new(80., 40.));
        g.world
            .entity_mut(first)
            .unwrap()
            .data
            .insert("depth".into(), 1.);
        g.world
            .entity_mut(later)
            .unwrap()
            .data
            .insert("depth".into(), 100.);
        g.step(&InputFrame::default());
        let order = &g.ending.as_ref().unwrap().animation_order;
        assert!(
            order.iter().position(|id| *id == later) < order.iter().position(|id| *id == first)
        );
        g.world
            .entity_mut(first)
            .unwrap()
            .data
            .insert("depth".into(), 100.);
        g.step(&InputFrame::default());
        let order = &g.ending.as_ref().unwrap().animation_order;
        assert!(
            order.iter().position(|id| *id == later) < order.iter().position(|id| *id == first),
            "equal depths preserve the prior global order rather than reverting to creation order"
        );
    }
    #[test]
    fn cinematic_draw_view_precedes_actor_animation() {
        let mut g = game();
        begin_exit(&mut g, EndingKind::Campaign);
        until(&mut g, 100, |s| s.stage == EndingStage::Chamber);
        let id = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oPDummy")
            .unwrap()
            .id;
        let before = g.world.entity(id).unwrap().animation;
        g.step(&InputFrame::default());
        assert_eq!(g.presentation().world.entity(id).unwrap().animation, before);
        assert!(g.presentation().ending.is_some());
        assert_ne!(g.world.entity(id).unwrap().animation, before);
    }
    #[test]
    fn cinematic_fade_snapshots_tick_start_and_resets_on_room_entry() {
        let mut g = game();
        begin_custom(&mut g);
        assert_eq!(
            g.ending.as_ref().unwrap().previous_fade_level,
            g.ending.as_ref().unwrap().fade_level
        );
        let before = g.ending.as_ref().unwrap().fade_level;
        g.step(&InputFrame::default());
        assert_eq!(g.ending.as_ref().unwrap().previous_fade_level, before);
        let mut state = g.ending.take().unwrap();
        state.fade_level = 0.37;
        state.previous_fade_level = 0.62;
        enter(&mut g, &mut state, EndingStage::Credits, "rCredits1");
        assert_eq!(state.previous_fade_level, state.fade_level);
        assert_eq!(state.fade_level, 1.);
    }
    #[test]
    fn tunnel_first_visit_sentinel_and_dialogue_hold_repeat_and_payment() {
        let mut first = Game::shell(24, RulesPreset::Classic);
        first.world.level = 4;
        first.player.money = 100_000;
        first.data.insert("tunnel1_remaining".into(), 100_001.);
        assert_eq!(
            crate::progression::apply_donation(&mut first, 1, 100_000),
            0
        );
        assert_eq!(first.player.money, 100_000);
        activate_fixture_exit(&mut first);
        until(&mut first, 100, |s| s.stage == EndingStage::Transition);
        assert_eq!(first.data["tunnel1_remaining"], 100_000.);
        assert!(
            !first
                .world
                .entities
                .iter()
                .any(|e| e.object == "oTunnelMan")
        );

        let mut g = Game::shell(25, RulesPreset::Classic);
        g.world.level = 4;
        g.player.money = 100_000;
        g.data
            .insert("tunnel1_remaining".into(), first.data["tunnel1_remaining"]);
        activate_fixture_exit(&mut g);
        until(&mut g, 100, |s| s.stage == EndingStage::Transition);
        let tunnel = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oTunnelMan")
            .unwrap()
            .id;
        for _ in 0..60 {
            if g.world.entity(tunnel).unwrap().value("talk") == 1. {
                break;
            }
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(tunnel).unwrap().value("talk"), 1.);
        let start = InputFrame {
            start_pressed: true,
            ..Default::default()
        };
        g.step(&start);
        assert_eq!(
            g.world.entity(tunnel).unwrap().value("talk"),
            1.,
            "Start accelerates the tally without advancing source Attack-only dialogue"
        );
        assert!(g.ending.as_ref().unwrap().hurry_up);
        let attack = InputFrame {
            attack_pressed: true,
            ..Default::default()
        };
        g.step(&attack);
        assert_eq!(g.world.entity(tunnel).unwrap().value("talk"), 2.);
        let up = InputFrame {
            up: true,
            ..Default::default()
        };
        for _ in 0..20 {
            g.step(&up);
        }
        assert_eq!(g.world.entity(tunnel).unwrap().value("donate"), 100.);
        g.step(&up);
        assert_eq!(g.world.entity(tunnel).unwrap().value("donate"), 200.);
        for _ in 21..100 {
            g.step(&up);
        }
        let before = g.world.entity(tunnel).unwrap().value("donate");
        g.step(&up);
        assert_eq!(
            g.world.entity(tunnel).unwrap().value("donate"),
            before + 1000.
        );
        let donation = g.world.entity(tunnel).unwrap().value("donate");
        g.step(&start);
        assert_eq!(g.world.entity(tunnel).unwrap().value("talk"), 2.);
        assert_eq!(
            g.player.money, 100_000,
            "Start must not pay the requested donation"
        );
        g.step(&attack);
        assert_eq!(g.world.entity(tunnel).unwrap().value("talk"), 3.);
        assert_eq!(g.data["tunnel1_remaining"], 100_000. - donation);
        assert_eq!(g.player.money as f64, 100_000. - donation);
        assert_eq!(
            g.state,
            RunState::Ending,
            "payment click cannot also advance the room"
        );
        g.step(&start);
        assert_eq!(g.state, RunState::Transition);
    }
    #[test]
    fn transition_dark_level_eligibility_uses_source_remainder_seconds() {
        for (milliseconds, disabled) in [
            (21_000, false),
            (61_000, true),
            (80_999, true),
            (81_000, false),
        ] {
            let mut g = Game::shell(26, RulesPreset::Classic);
            g.flags.insert("no_dark_level".into());
            g.data.insert("xtime".into(), milliseconds as f64 - 960.);
            activate_fixture_exit(&mut g);
            until(&mut g, 100, |s| s.stage == EndingStage::Transition);
            assert_eq!(g.ending.as_ref().unwrap().level_time_ms, milliseconds);
            until(&mut g, 100, |s| s.draw_loot > -2);
            assert_eq!(
                g.flags.contains("no_dark_level"),
                disabled,
                "source milliseconds {milliseconds}"
            );
        }
    }
    #[test]
    fn natural_treasure_pickup_and_door_show_and_consume_transition_icon_once() {
        for (object, category, sprite, lifetime, expected_money) in [
            ("oRuby", "rubies", "sRuby", Some("collected:oRuby"), 400),
            ("oScarab", "scarabs", "sScarabDisp", None, 4000),
        ] {
            let mut g = Game::shell(27, RulesPreset::Classic);
            g.world = World::new(40, 20, Area::Mines, 1);
            for x in 0..40 {
                g.world.set_tile(x, 3, "oBlock");
            }
            g.player.pos = Vec2::new(40., 40.);
            g.player.previous_pos = g.player.pos;
            let pos = Vec2::new(32., 32.);
            let item = g.world.spawn(object, pos);
            g.world
                .entity_mut(item)
                .unwrap()
                .data
                .insert("gravity".into(), 0.);
            g.world.spawn("oExit", Vec2::new(280., 32.));
            g.step(&InputFrame::default());
            assert_eq!(g.player.money, 0, "{object} queues its collection payout");
            for _ in 1..75 {
                g.step(&InputFrame::default());
            }
            assert_eq!(g.data[category], 1., "source category for {object}");
            if let Some(key) = lifetime {
                assert_eq!(g.data[key], 1.);
            }
            assert_eq!(g.player.money, expected_money, "source payout for {object}");
            let walk = InputFrame {
                right: true,
                up: true,
                ..Default::default()
            };
            for _ in 0..250 {
                g.step(&walk);
                if g.state == RunState::Ending {
                    break;
                }
            }
            until(&mut g, 100, |s| s.stage == EndingStage::Transition);
            until(&mut g, 100, |s| s.draw_loot == 2);
            assert_eq!(g.ending.as_ref().unwrap().level_money(), expected_money);
            assert!(g.ending.as_ref().unwrap().is_loot);
            assert_eq!(g.data[category], 0.);
            if let Some(key) = lifetime {
                assert_eq!(g.data[key], 1.);
            }
            assert_eq!(g.player.money, expected_money);
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oSprite" && e.sprite == sprite)
                    .count(),
                1,
                "one source icon for {object}"
            );
        }
    }

    #[test]
    fn source_transition_room_steps_apply_the_authored_wall_boundary() {
        let cases: &[(Area, u32, &str, &[&str])] = &[
            (Area::Jungle, 5, "rTransition1x", &["oLush"]),
            (Area::Jungle, 6, "rTransition2", &["oLush"]),
            (Area::Jungle, 9, "rTransition2x", &["oLush", "oDark"]),
            (Area::Ice, 10, "rTransition3", &["oDark"]),
            (Area::Ice, 13, "rTransition3x", &["oDark", "oTemple"]),
            (Area::Temple, 14, "rTransition4", &["oTemple"]),
        ];

        for (index, (area, next, room, families)) in cases.iter().enumerate() {
            let mut game = Game::shell(index as u32 + 1, RulesPreset::Classic);
            game.world = World::new(40, 20, *area, *next - 1);
            game.world.name = "rLevel".into();
            game.data.insert("next_level".into(), f64::from(*next));
            begin_exit(&mut game, EndingKind::Transition);

            for _ in 0..100 {
                if game.world.name == *room
                    && game
                        .ending
                        .as_ref()
                        .is_some_and(|ending| ending.stage == EndingStage::Transition)
                {
                    break;
                }
                game.step(&InputFrame::default());
            }
            assert_eq!(game.world.name, *room);
            assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);

            for family in *families {
                let floor_index = game
                    .world
                    .tiles
                    .iter()
                    .enumerate()
                    .find(|(index, tile)| {
                        tile.object == *family && *index / game.world.width * 16 == 224
                    })
                    .map(|(index, _)| index)
                    .expect("authored transition terrain reaches scrSetupWalls(224)");
                let floor = Vec2::new(
                    (floor_index % game.world.width) as f64 * 16.0 + 8.0,
                    (floor_index / game.world.width) as f64 * 16.0 + 8.0,
                );
                let metadata = game
                    .world
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.alive && entity.object == "oTerrainMetadata" && entity.pos == floor
                    })
                    .expect("SetupWalls materializes terrain metadata");
                assert_eq!(
                    metadata.value("terrain_setup_initialized"),
                    1.0,
                    "{room} {family}"
                );
                assert_eq!(metadata.value("down"), 1.0, "{room} {family} floor bound");
            }
        }
    }

    #[test]
    fn transition_cape_and_punishment_ball_chain_follow_the_running_dummy() {
        let mut g = Game::shell(28, RulesPreset::Classic);
        g.player.equipment.insert("Cape".into());
        g.data.insert("kali_punish".into(), 2.);
        begin_exit(&mut g, EndingKind::Transition);
        until(&mut g, 100, |s| s.stage == EndingStage::Transition);
        let ball = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oBall2")
            .unwrap()
            .id;
        let start = g.world.entity(ball).unwrap().pos;
        for _ in 0..40 {
            g.step(&InputFrame::default());
        }
        let dummy = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oPDummy")
            .unwrap();
        let cape = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oCape")
            .unwrap();
        assert_eq!(cape.pos, Vec2::new(dummy.pos.x - 4., dummy.pos.y - 2.));
        assert_eq!(cape.sprite, "sCapeRight");
        assert_eq!(cape.value("depth"), 100.);
        let ball = g.world.entity(ball).unwrap();
        assert!(
            ball.pos.x > start.x,
            "inherited physics plus tether must advance the ball"
        );
        assert!(
            ball.pos.y > start.y,
            "inherited gravity must lower the ball to the floor"
        );
        let links = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oChain2")
            .collect::<Vec<_>>();
        assert_eq!(links.len(), 4);
        for link in links {
            let fraction = link.value("linkVal") / 4.;
            assert_eq!(
                link.pos,
                Vec2::new(
                    ball.pos.x + (dummy.pos.x - ball.pos.x) * fraction,
                    ball.pos.y + (dummy.pos.y - ball.pos.y) * fraction
                )
            );
        }
        for _ in 0..150 {
            g.step(&InputFrame::default());
        }
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.object == "oPDummy" || e.object == "oCape")
        );
        let last = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oChain2" && e.value("linkVal") == 4.)
            .unwrap();
        assert_eq!(last.pos, Vec2::new(280., 184.));
    }
    #[test]
    fn real_exit_lava_death_enters_a_running_tally_without_reviving_player() {
        use crate::characters::{self, Character};
        use crate::model::GameEvent;
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
                // Natural door/animation route; only the arriving hazard is
                // controlled. Neither player life nor its Exit phase is set.
                for (arriving_lava, ankh, last_exit_frame) in [
                    (false, false, false),
                    (true, false, false),
                    (true, false, true),
                    (true, true, false),
                ] {
                    let mut g = Game::shell(11, rules);
                    g.world = crate::model::World::new(40, 20, Area::Mines, 1);
                    g.world.name = "Mines".into();
                    for x in 0..40 {
                        g.world.set_tile(x, 9, "oHardBlock");
                    }
                    // Controlled initial campaign room geometry. Ordinary
                    // campaign exits use rTransition; no custom adapter flags.
                    g.world.entrance = Vec2::new(120., 136.);
                    g.player.pos = g.world.entrance;
                    g.player.previous_pos = g.player.pos;
                    g.spawn("oEntrance", Vec2::new(112., 128.));
                    g.spawn("oExit", Vec2::new(128., 128.));
                    characters::apply(&mut g, character);
                    for _ in 0..8 {
                        g.step(&InputFrame::default());
                    }
                    let initial_health = g.player.health;
                    if ankh {
                        let id = g.spawn("oAnkh", Vec2::new(g.player.pos.x, g.player.pos.y + 4.));
                        // Let oItem's real no-shop Step clear its Create cost.
                        g.step(&InputFrame::default());
                        assert_eq!(g.world.entity(id).unwrap().value("cost"), 0.);
                        g.step(&InputFrame {
                            down: true,
                            attack: true,
                            attack_pressed: true,
                            ..Default::default()
                        });
                        assert!(g.player.has("Ankh"));
                    }
                    for _ in 0..30 {
                        g.step(&InputFrame {
                            right: true,
                            up: true,
                            ..Default::default()
                        });
                        if g.player.is_exiting() {
                            break;
                        }
                    }
                    assert!(g.player.is_exiting(), "{rules:?}/{character:?}");
                    assert_eq!(g.state, RunState::Ending);
                    let exit_sprite = g.player.sprite.clone();
                    if last_exit_frame {
                        // Source Step1809..1878 selects a Die sprite on the
                        // next dead Step. A fatal last Exit frame instead
                        // reaches Animation End in this same engine frame.
                        let frames = crate::catalog::sprite_frame_count(&exit_sprite) as f64;
                        for _ in 0..40 {
                            if (g.player.animation + g.player.image_speed()).floor() >= frames {
                                break;
                            }
                            g.step(&InputFrame::default());
                            assert_eq!(g.player.sprite, exit_sprite);
                            assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Exit);
                        }
                        assert!((g.player.animation + g.player.image_speed()).floor() >= frames);
                    }
                    if arriving_lava {
                        g.spawn("oLava", Vec2::new(g.player.pos.x - 8., g.player.pos.y - 8.));
                        g.step(&InputFrame::default());
                        assert!(g.events.contains(&GameEvent::Damage { amount: 99 }));
                        assert_eq!(g.player.sprite, exit_sprite);
                        assert_eq!(g.player.health, if ankh { 4.0 } else { 0.0 });
                        assert_eq!(
                            g.state,
                            if ankh || last_exit_frame {
                                RunState::Ending
                            } else {
                                RunState::Dead
                            }
                        );
                        assert!(!g.player.has("Ankh"));
                        assert_eq!(g.events.contains(&GameEvent::Died), !ankh);
                        // Room replacement intentionally discards draw_view,
                        // so the pre-Animation snapshot cannot be read after
                        // this full Step. Damage/Died plus the unchanged Exit
                        // sprite and HP0 witness the lethal callback boundary.
                        if !last_exit_frame && !ankh {
                            // Early lava leaves enough time for the NEXT dead
                            // Step to select Die. It must NOT finish the exit.
                            // Lava's early source action still subtracts 99
                            // from dead/hidden players. DIED clamps only the
                            // first fatal Step; later Steps retain the damage.
                            for corpse_step in 1..=40 {
                                g.step(&InputFrame::default());
                                assert!(g.events.contains(&GameEvent::Damage { amount: 99 }));
                                assert_eq!(g.player.health, -99.0 * f64::from(corpse_step));
                                assert!(!g.events.contains(&GameEvent::Died));
                                assert_eq!(
                                    g.player.data.get("hidden").copied().unwrap_or(0.0),
                                    if corpse_step == 1 { 0.0 } else { 1.0 }
                                );
                            }
                            assert_eq!(g.world.name, "Mines");
                            assert_eq!(g.state, RunState::Dead);
                            assert_eq!(g.player.health, -99.0 * 40.0);
                            assert!(!g.player.is_exiting());
                            assert_eq!(
                                g.player.sprite,
                                selected_sprite(&g, "sDieL", "sDamselDieL", "sTunnelDieL")
                            );
                            assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Exit);
                            assert!(g.game_over.destination.is_none());
                            continue;
                        }
                    }
                    let expected_health = if arriving_lava {
                        if ankh { 4.0 } else { 0.0 }
                    } else {
                        initial_health
                    };
                    for _ in 0..40 {
                        if g.ending.as_ref().unwrap().stage != EndingStage::Exit {
                            break;
                        }
                        g.step(&InputFrame::default());
                    }
                    assert_eq!(g.world.name, "rTransition1");
                    assert_eq!(g.ending.as_ref().unwrap().stage, EndingStage::Transition);
                    assert_eq!(g.state, RunState::Ending);
                    assert_eq!(g.player.health, expected_health);
                    assert!(
                        g.game_over.destination.is_none(),
                        "no GameOver acknowledgment was sent"
                    );
                    let stage_tick = g.ending.as_ref().unwrap().stage_tick;
                    for _ in 0..60 {
                        g.step(&InputFrame::default());
                    }
                    let tally = g.ending.as_ref().unwrap();
                    assert_eq!(tally.stage_tick, stage_tick + 60);
                    assert!(
                        tally.draw_loot >= 0,
                        "the new room's controller runs after dead-player departure"
                    );
                    assert_eq!(
                        g.player.health, expected_health,
                        "room routing must not revive the departed player"
                    );
                }
            }
        }
    }
    #[test]
    fn lava_spray_requests_flame_only_if_idle_each_source_step() {
        let mut g = game();
        begin_custom(&mut g);
        let mut s = g.ending.take().unwrap();
        spawn(&mut g, &mut s, "oLavaSpray", Vec2::new(40., 220.));
        g.ending = Some(s);
        for _ in 0..3 {
            g.step(&InputFrame::default());
            assert_eq!(g.events.iter().filter(|event|matches!(event,crate::model::GameEvent::SoundIfIdle{name,..} if name=="sndFlame")).count(),1);
            assert!(!g.events.iter().any(
                |event| matches!(event,crate::model::GameEvent::Sound{name,..} if name=="sndFlame")
            ));
        }
    }
}
pub fn begin_exit(game: &mut Game, kind: EndingKind) {
    let ending = EndingState::new(game, kind, EndingStage::Exit);
    let sprite = selected_sprite(game, "sPExit", "sDamselExit", "sTunnelExit");
    game.player.set_sprite(sprite);
    game.player.set_image_speed(0.5);
    game.player.data.insert("active".into(), 0.);
    game.player.data.insert("depth".into(), 999.);
    game.player.invincible = 999;
    game.ending = Some(ending);
    game.state = RunState::Ending;
}
pub fn begin_hub_exit(game: &mut Game, portal: crate::hub::HubPortal) {
    begin_exit(game, EndingKind::HubPortal);
    game.ending.as_mut().unwrap().portal = Some(portal);
}
pub fn begin_custom(game: &mut Game) {
    if game
        .ending
        .as_ref()
        .is_some_and(|s| s.kind == EndingKind::Custom)
    {
        return;
    }
    let mut ending = EndingState::new(game, EndingKind::Custom, EndingStage::Summary);
    if let Some(previous) = &game.ending {
        ending.completed_name = previous.completed_name.clone();
        ending.completed_level = previous.completed_level;
        ending.source_time_ms = previous.source_time_ms;
        ending.level_time_ms = previous.level_time_ms;
        ending.shortcut_used = previous.shortcut_used;
        ending.pickup_item = previous.pickup_item.clone();
    }
    enter(game, &mut ending, EndingStage::Summary, "rEndCustom");
    game.ending = Some(ending);
    game.state = RunState::Ending;
}
pub fn begin_title_credits(game: &mut Game) {
    let mut ending = EndingState::new(game, EndingKind::TitleCredits, EndingStage::Exit);
    let id = spawn(game, &mut ending, "oPDummy5", game.player.pos);
    let tunnel = crate::characters::selected(game) == crate::characters::Character::TunnelMan;
    let actor = game.world.entity_mut(id).unwrap();
    // The original has two independent ifs; the second else overwrites the
    // Damsel sprite. Preserve that source behavior in this title-only dummy.
    actor.sprite = if tunnel { "sTunnelClimb2" } else { "sClimbUp2" }.into();
    actor.vel.y = -1.;
    actor.data.insert("animation_speed".into(), 0.6);
    game.player.data.insert("hidden".into(), 1.);
    game.player.data.insert("active".into(), 0.);
    game.ending = Some(ending);
    game.state = RunState::Ending;
}
fn selected_sprite<'a>(
    game: &Game,
    explorer: &'a str,
    damsel: &'a str,
    tunnel: &'a str,
) -> &'a str {
    match crate::characters::selected(game) {
        crate::characters::Character::Explorer => explorer,
        crate::characters::Character::Damsel => damsel,
        crate::characters::Character::TunnelMan => tunnel,
    }
}

/// Load the authored instances, including their original offsets and layers.
/// Gameplay counters intentionally survive until the host consumes the record.
pub fn load_room(game: &mut Game, name: &str) {
    static ROOMS: OnceLock<serde_json::Value> = OnceLock::new();
    let rooms = ROOMS.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/cinematics.json"))
            .expect("checked cinematic assets")
    });
    let room = &rooms["rooms"][name];
    assert!(room.is_object(), "unknown cinematic room {name}");
    let mut world = World::new(
        room["width"].as_u64().unwrap() as usize / 16,
        (room["height"].as_u64().unwrap() as usize).div_ceil(16),
        game.world.area,
        game.world.level,
    );
    world.name = name.into();
    for instance in room["instances"].as_array().unwrap() {
        if instance["object"].as_str() == Some("oEndPlat") {
            // This authored oSolid is an eighty-pixel moving platform. A
            // sixteen-pixel terrain cell loses both its mask and carry motion.
            world.spawn(
                "oEndPlat",
                Vec2::new(
                    instance["x"].as_f64().unwrap(),
                    instance["y"].as_f64().unwrap(),
                ),
            );
            continue;
        }
        crate::hub::add_instance(
            &mut world,
            instance["object"].as_str().unwrap(),
            Vec2::new(
                instance["x"].as_f64().unwrap(),
                instance["y"].as_f64().unwrap(),
            ),
            false,
        );
    }
    game.world = world;
    crate::hub::initialize_brick_variants(game);
    let city = game.flags.contains("cityOfGold") || game.flags.contains("city_of_gold");
    let cemetery = game.flags.contains("cemetary") || game.flags.contains("cemetery");
    if !crate::generation::initialize_authored_setup_walls(
        &mut game.world,
        &mut game.rng,
        game.generation_revision,
        !game.flags.contains("low_graphics"),
        city,
        cemetery,
    ) {
        crate::hub::setup_wall_sprites(&mut game.world);
    }
    game.camera = Vec2::default();
    game.previous_camera = game.camera;
    game.level_tick = 0;
    game.player.data.insert("hidden".into(), 1.);
    game.data.insert("darkness".into(), 0.);
    game.data.insert("darkness_lerp".into(), 0.);
    game.flags.insert("render_discontinuity".into());
}

fn enter(game: &mut Game, s: &mut EndingState, stage: EndingStage, room: &str) {
    // oEnd.events/Room End clears global.shake when leaving rEnd.
    if game.world.name == "rEnd" {
        game.data.insert("shake".into(), 0.);
    }
    load_room(game, room);
    s.stage = stage;
    s.stage_tick = 0;
    s.alarms = [-1; 12];
    s.actor_alarms.clear();
    s.animation_order.clear();
    s.music.clear();
    let ids: Vec<_> = game.world.entities.iter().map(|e| e.id).collect();
    for id in ids {
        init_actor(game, s, id);
    }
    if matches!(
        stage,
        EndingStage::Chamber | EndingStage::Transition | EndingStage::Credits
    ) {
        crate::music::stop_all(game);
    }
    match stage {
        EndingStage::Chamber => {
            if let Some(actor) = game
                .world
                .entities
                .iter_mut()
                .find(|e| e.object == "oPDummy")
            {
                actor.state = 2;
            }
            add_carry_ornaments(game, s);
        }
        EndingStage::Volcano => {
            s.alarms[0] = 50;
            s.alarms[2] = 10;
        }
        EndingStage::Landing | EndingStage::Summary => {
            s.draw_status = 0;
            s.money_count = 0;
            s.fade_out = false;
            s.fade_in = false;
            s.fade_level = 0.;
            s.bonus_scheduled = false;
            s.alarms[0] = 50;
            set_menu(game, false);
        }
        EndingStage::Credits => {
            s.fade_in = true;
            s.fade_out = false;
            s.fade_level = 1.;
            s.credits_page = 0;
            s.scroll_started = false;
            s.scrolling = false;
            if s.kind == EndingKind::TitleCredits {
                spawn(game, s, "oIntroBG", Vec2::new(-120., 112.));
            }
        }
        EndingStage::Transition => {
            // oTransition Create clears the tutorial run after its ordinary
            // exit has advanced currLevel from zero to one. The authored
            // manual oXTitle portal takes its separate Title path instead.
            if s.completed_level == 0 && game.flags.contains("tutorial") {
                // Source creates the Cape before scrClearGlobals. Punishment
                // and kiss actors are considered afterward, using reset globals.
                if game.player.has("Cape") {
                    spawn(game, s, "oCape", Vec2::default());
                }
                game.clear_run_globals();
                game.flags.remove("tutorial");
                game.flags.remove("first_time");
                game.flags.insert("onboarding_consumed".into());
                game.data.insert("next_level".into(), 1.);
                s.source_time_ms = 0;
                s.level_time_ms = 0;
                s.level_money = 0;
                s.pickup_item.clear();
                // init_actor ran before this source Create boundary. Its Draw
                // cache must observe the same cleared global pickupItem.
                for actor in &mut game.world.entities {
                    if let Some(item) = actor.text.get_mut("pickup_item") {
                        item.clear();
                    }
                }
            }
            s.draw_loot = -2;
            s.money_count = 0;
            s.hurry_up = false;
            s.draw_pos = Vec2::new(100., 83.);
            s.alarms[0] = 10;
            s.alarms[1] = 30;
            if game
                .data
                .get("pending_damsel_kisses")
                .copied()
                .unwrap_or(0.)
                > 0.
                || game.flags.contains("damsel_rescued")
            {
                spawn(game, s, "oDamselKiss", Vec2::new(184., 184.));
            }
            add_carry_ornaments(game, s);
            setup_tunnel(game, s, room);
        }
        _ => {}
    }
    s.previous_fade_level = s.fade_level;
}
fn set_menu(game: &mut Game, visible: bool) {
    for e in &mut game.world.entities {
        if crate::catalog::is_a(&e.object, "oMenu") {
            e.data.insert("visible".into(), f64::from(visible));
        }
    }
}
fn init_actor(game: &mut Game, s: &mut EndingState, id: u64) {
    let run = selected_sprite(game, "sRunLeft", "sDamselRunL", "sTunnelRunL").to_owned();
    let standing = selected_sprite(game, "sStandLeft", "sDamselLeft", "sTunnelLeft").to_owned();
    let damsel = crate::characters::selected(game) == crate::characters::Character::Damsel;
    let tunnel = crate::characters::selected(game) == crate::characters::Character::TunnelMan;
    let has_damsels = game.data.get("damsels").copied().unwrap_or(0.) > 0.;
    let flame = game
        .world
        .entity(id)
        .is_some_and(|e| e.object == "oVolcanoFlame");
    let flame_velocity = if flame {
        Some((
            Vec2::new(
                game.rng.unit() * 4. - game.rng.unit() * 4.,
                -1. - game.rng.unit() * 2.,
            ),
            (1 + game.rng.range(6)) as f64 * 0.1,
        ))
    } else {
        None
    };
    let Some(e) = game.world.entity_mut(id) else {
        return;
    };
    e.data.insert("animation_speed".into(), 1.);
    match e.object.as_str() {
        "oPDummy" => {
            e.sprite = run;
            e.facing = 1;
            e.text.insert("pickup_item".into(), s.pickup_item.clone());
        }
        "oPDummy2" => {
            e.sprite = run;
            e.facing = -1;
            e.text.insert("pickup_item".into(), s.pickup_item.clone());
        }
        "oPDummy6" => {
            e.sprite = standing;
            e.facing = -1;
            e.text.insert("pickup_item".into(), s.pickup_item.clone());
        }
        "oDamselKiss" => {
            e.sprite = if damsel { "sStandLeft" } else { "sDamselLeft" }.into();
            e.facing = -1;
            e.data.insert("animation_speed".into(), 0.5);
        }
        "oBigTreasure" => {
            e.data.insert("gravity".into(), 0.6);
        }
        "oBall2" => {
            crate::items::init_entity(e);
            for (key, value) in [
                ("bboxLeft", -5.),
                ("bboxTop", -5.),
                ("bboxRight", 5.),
                ("bboxBottom", 5.),
                ("heavy", 1.),
                ("gravity", 1.),
                ("depth", 101.),
            ] {
                e.data.insert(key.into(), value);
            }
        }
        "oPlayerSil" | "oTreasureSil" => {
            e.vel = Vec2::new(-6., -8.);
            e.data.insert("gravity".into(), 0.6);
        }
        "oCamel" => {
            if damsel {
                e.sprite = "sCamelDamsel".into();
            } else if tunnel {
                e.sprite = "sCamelTunnel".into();
            }
            e.data.insert("animation_speed".into(), 0.5);
        }
        "oCaravan" => {
            if has_damsels {
                e.sprite = if damsel { "sCaravan3" } else { "sCaravan2" }.into();
            }
            e.data.insert("animation_speed".into(), 0.5);
        }
        "oWalDamsel" => {
            e.data.insert("animation_speed".into(), 0.5);
        }
        "oVolcanoFlame" => {
            e.data.insert("animation_speed".into(), 0.3);
            let (velocity, gravity) = flame_velocity.unwrap();
            e.vel = velocity;
            e.data.insert("gravity".into(), gravity);
        }
        "oPoof" | "oBurn" => {
            e.data.insert("animation_speed".into(), 0.4);
            if e.object == "oBurn" {
                e.vel.y = -0.1;
            }
        }
        "oHeart" => {
            alarm(s, id, 0, 30);
        }
        "oBGEnd3" => {
            e.pos.x -= 64.;
            e.previous_pos = e.pos;
            e.data.insert("x_off".into(), -64.);
        }
        "oIntroBG" => {
            // oIntroBG Create initializes xOff before the Credits1 Step pins
            // the sprite to the view. Credits1 creates it at x=-120, but
            // its first source Step moves it to view_xview + xOff (zero).
            e.data.insert("x_off".into(), 0.);
        }
        "oSprite" => {
            e.data.insert("animation_speed".into(), 0.);
        }
        "oLavaSpray" => {
            e.data.insert("y_acc".into(), 0.);
        }
        "oBricks" | "oGame" | "oEnd" | "oEnd2" | "oEnd3" | "oEndCustom" | "oTransition"
        | "oCredits1" | "oCredits2" => {
            e.data.insert("hidden".into(), 1.);
        }
        _ => {}
    }
    if s.kind == EndingKind::TitleCredits {
        let night = match e.object.as_str() {
            "oDesertScroll" => Some("sDesertNight"),
            "oDesertScroll2" => Some("sDesertNight2"),
            "oDesertTopScroll" => Some("sDesertTopNight"),
            "oPalmTreeScroll" => Some("sPalmTreeDark"),
            "oShrubScroll" => Some("sShrubDark"),
            _ => None,
        };
        if let Some(sprite) = night {
            e.sprite = sprite.into();
        }
    }
    if e.object.starts_with("oPDummy") {
        e.data
            .insert("image_xscale".into(), if e.facing > 0 { -1. } else { 1. });
    }
}
fn spawn(game: &mut Game, s: &mut EndingState, object: &str, pos: Vec2) -> u64 {
    let id = game.world.spawn(object, pos);
    init_actor(game, s, id);
    id
}
fn add_carry_ornaments(game: &mut Game, s: &mut EndingState) {
    let Some(pos) = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oPDummy")
        .map(|e| e.pos)
    else {
        return;
    };
    if game.player.has("Cape") {
        spawn(game, s, "oCape", Vec2::default());
    }
    if game.data.get("kali_punish").copied().unwrap_or(0.) >= 2. {
        spawn(game, s, "oBall2", Vec2::new(pos.x, pos.y + 2.));
        for link in 1..=4 {
            let id = spawn(game, s, "oChain2", pos);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("linkVal".into(), link as f64);
        }
    }
}
fn alarm(s: &mut EndingState, id: u64, index: usize, value: i32) {
    s.actor_alarms.entry(id).or_insert([-1; 12])[index] = value;
}
fn action(input: &InputFrame) -> bool {
    input.attack_pressed || input.start_pressed
}

/// Cinematics have their own source Step/Alarm events. They do not run enemy
/// combat, hazard damage, campaign clocks, or normal player input.
pub fn tick(game: &mut Game, input: &InputFrame) {
    // Preserve the direct cinematic driver's held-only input contract using
    // the same engine-frame normalization as Game.step (idempotent after it).
    let input = game.normalize_action_edges(input);
    tick_inner(game, &input, false);
}
/// Called after the live room has completed its original Step events during
/// the player's exit animation. Keep its events, clocks and previous poses.
pub(crate) fn live_exit_step(game: &mut Game) {
    if let Some(s) = &mut game.ending {
        if s.stage == EndingStage::Exit && s.kind != EndingKind::TitleCredits {
            s.stage_tick += 1;
            s.source_time_ms = game.data.get("time").copied().unwrap_or(0.).max(0.) as u64;
            s.level_time_ms = game.data.get("xtime").copied().unwrap_or(0.).max(0.) as u64;
        }
    }
}

/// Source Animation End callback after the common engine animation phase has
/// advanced and wrapped the player's image_index exactly once.
pub(crate) fn player_animation_end(game: &mut Game) {
    let Some(mut s) = game.ending.take() else {
        return;
    };
    if s.stage != EndingStage::Exit || s.kind == EndingKind::TitleCredits {
        game.ending = Some(s);
        return;
    }
    let collect = game.data.remove("collect").unwrap_or(0.).max(0.) as u32;
    game.player.money = game.player.money.saturating_add(collect);
    game.player.held = None;
    if let Some(portal) = s.portal {
        if portal == crate::hub::HubPortal::End {
            // Classic xEND follows the shared exit animation directly into
            // rEnd; it is not a normal title-start portal.
            s.kind = EndingKind::Campaign;
        } else {
            crate::hub::complete_portal(game, portal);
            return;
        }
    }
    // oPlayer1 Animation End: the ordinary exit advances currLevel before
    // this callback, so City remains active until the level-16 transition.
    if s.kind == EndingKind::Transition && game.data.get("next_level") == Some(&16.) {
        game.flags.remove("city_of_gold");
        game.flags.remove("cityOfGold");
    }
    if game.flags.contains("editor_test") {
        game.state = RunState::Transition;
        return;
    }
    if s.kind == EndingKind::Campaign {
        enter(game, &mut s, EndingStage::Chamber, "rEnd");
    } else {
        let room = transition_room(game);
        enter(game, &mut s, EndingStage::Transition, room);
    }
    // Animation End replaces the departing, nonpersistent Player/Game with
    // the authored cinematic controller even if lava killed that player.
    // Keep global life/death consequences; this selects the new room's Step.
    game.state = RunState::Ending;
    game.ending = Some(s);
}
fn tick_inner(game: &mut Game, input: &InputFrame, world_already_ticked: bool) {
    let Some(mut s) = game.ending.take() else {
        return;
    };
    if s.stage == EndingStage::Complete {
        game.ending = Some(s);
        return;
    }
    s.previous_fade_level = s.fade_level;
    if !world_already_ticked {
        game.events.clear();
        game.tick = game.tick.wrapping_add(1);
        game.level_tick = game.level_tick.wrapping_add(1);
        game.previous_camera = game.camera;
        game.player.previous_pos = game.player.pos;
        for e in &mut game.world.entities {
            e.previous_pos = e.pos;
        }
    }
    s.stage_tick += 1;
    if s.stage == EndingStage::Exit && s.kind != EndingKind::TitleCredits {
        s.source_time_ms = game.data.get("time").copied().unwrap_or(0.).max(0.) as u64;
        s.level_time_ms = game.data.get("xtime").copied().unwrap_or(0.).max(0.) as u64;
        game.ending = Some(s);
        return;
    }
    let room = game.world.name.clone();
    // GM8 dispatches one alarm number across object resources before moving
    // to the next alarm number. Fire immediately: a callback can set a later
    // alarm which is then decremented during this same source frame.
    for index in 0..12 {
        let mut queue = dispatch_queue(game, &s);
        let mut next_id = game.world.next_id;
        while let Some((key, target)) = queue.pop_first() {
            let due = match target {
                DispatchTarget::Controller => advance_alarm(&mut s.alarms[index]),
                DispatchTarget::Actor(id) => {
                    game.world.entity(id).is_some()
                        && s.actor_alarms
                            .get_mut(&id)
                            .is_some_and(|alarms| advance_alarm(&mut alarms[index]))
                }
            };
            if due {
                match target {
                    DispatchTarget::Controller => controller_alarm(game, &mut s, index),
                    DispatchTarget::Actor(id) => actor_alarm(game, &mut s, id, index),
                }
                if game.world.name != room || s.stage == EndingStage::Complete {
                    game.ending = Some(s);
                    return;
                }
                register_created(game, &mut queue, key, &mut next_id);
            }
        }
    }
    if input.music_pressed {
        crate::music::key_pressed(game);
    }
    // Step follows the same resource/creation ordering. The controller is an
    // instance at its own resource position, not a before/after-actor hook.
    let mut queue = dispatch_queue(game, &s);
    let mut next_id = game.world.next_id;
    while let Some((key, target)) = queue.pop_first() {
        match target {
            DispatchTarget::Controller => controller_step(game, &mut s, input),
            DispatchTarget::Actor(id) => {
                if let Some(index) = game
                    .world
                    .entities
                    .iter()
                    .position(|e| e.id == id && e.alive)
                {
                    let mut entity = game.world.entities.remove(index);
                    actor_step(game, &mut s, &mut entity, input);
                    if game.world.name == room && s.stage != EndingStage::Complete && entity.alive {
                        game.world
                            .entities
                            .insert(index.min(game.world.entities.len()), entity);
                    }
                }
            }
        }
        if game.world.name != room || s.stage == EndingStage::Complete {
            game.ending = Some(s);
            return;
        }
        register_created(game, &mut queue, key, &mut next_id);
    }
    prepare_draw_order(game, &mut s);
    // Preserve the source Draw boundary before animation/callbacks mutate
    // actors. The state is temporarily restored so the immutable view includes
    // the same controller fields as the authored room's Draw event.
    game.ending = Some(s.clone());
    game.capture_draw_view();
    game.ending = None;
    // Animation traverses the same stable descending-depth global array as
    // Draw. Its resource Step arrays remain separate, and equal-depth actors
    // retain their preceding frame's global ordering.
    let animation_ids = s.animation_order.clone();
    for id in animation_ids {
        if let Some(index) = game
            .world
            .entities
            .iter()
            .position(|entity| entity.id == id && entity.alive)
        {
            let mut entity = game.world.entities.remove(index);
            animate(game, &mut s, &mut entity);
            if entity.alive {
                game.world
                    .entities
                    .insert(index.min(game.world.entities.len()), entity);
            }
        }
    }
    s.previous_up = input.up;
    s.previous_down = input.down;
    game.world.entities.retain(|e| e.alive);
    game.ending = Some(s);
}

fn prepare_draw_order(game: &Game, state: &mut EndingState) {
    state
        .animation_order
        .retain(|id| game.world.entity(*id).is_some());
    let mut births: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && !state.animation_order.contains(&entity.id))
        .map(|entity| (entity.creation_serial, entity.id))
        .collect();
    births.sort_unstable();
    state
        .animation_order
        .extend(births.into_iter().map(|(_, id)| id));
    let depth = |id| {
        game.world.entity(id).map_or(0., |entity| {
            entity.data.get("depth").copied().unwrap_or_else(|| {
                crate::catalog::object(&entity.object).map_or(0., |object| object.depth as f64)
            })
        })
    };
    state
        .animation_order
        .sort_by(|a, b| depth(*b).total_cmp(&depth(*a)));
}

#[derive(Clone, Copy)]
enum DispatchTarget {
    Controller,
    Actor(u64),
}
type DispatchQueue = BTreeMap<(i32, u64), DispatchTarget>;
fn resource_index(object: &str) -> i32 {
    crate::catalog::object(object).map_or(i32::MAX, |definition| definition.id)
}
fn controller_object(state: &EndingState) -> &'static str {
    match state.stage {
        EndingStage::Exit => "oTitle",
        EndingStage::Chamber => "oEnd",
        EndingStage::Volcano => "oEnd2",
        EndingStage::Landing => "oEnd3",
        EndingStage::Summary if state.kind == EndingKind::Custom => "oEndCustom",
        EndingStage::Summary => "oEnd3",
        EndingStage::Transition => "oTransition",
        EndingStage::Credits if state.kind == EndingKind::TitleCredits => "oCredits1",
        EndingStage::Credits => "oCredits2",
        EndingStage::Complete => "oEnd3",
    }
}
fn dispatch_queue(game: &Game, state: &EndingState) -> DispatchQueue {
    let controller = controller_object(state);
    let mut queue = DispatchQueue::new();
    let mut controller_id = 0;
    for entity in game.world.entities.iter().filter(|entity| entity.alive) {
        if entity.object == controller {
            controller_id = entity.id;
        } else {
            queue.insert(
                (resource_index(&entity.object), entity.id),
                DispatchTarget::Actor(entity.id),
            );
        }
    }
    queue.insert(
        (resource_index(controller), controller_id),
        DispatchTarget::Controller,
    );
    queue
}
fn register_created(game: &Game, queue: &mut DispatchQueue, cursor: (i32, u64), next_id: &mut u64) {
    if game.world.next_id == *next_id {
        return;
    }
    for entity in game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.id >= *next_id)
    {
        let key = (resource_index(&entity.object), entity.id);
        if key.0 > cursor.0 {
            queue.insert(key, DispatchTarget::Actor(entity.id));
        }
    }
    *next_id = game.world.next_id;
}
fn advance_alarm(value: &mut i32) -> bool {
    if *value < 0 {
        return false;
    }
    *value -= 1;
    // Source leaves an expired alarm at zero for the rest of this frame;
    // a subsequent alarm pass changes zero to -1 without another callback.
    *value == 0
}

fn transition_room(game: &Game) -> &'static str {
    let next = game
        .data
        .get("next_level")
        .copied()
        .unwrap_or(game.world.level as f64 + 1.) as u32;
    match next {
        5 => "rTransition1x",
        9 => "rTransition2x",
        13 => "rTransition3x",
        _ => match game.world.area {
            crate::model::Area::Mines => "rTransition1",
            crate::model::Area::Jungle => "rTransition2",
            crate::model::Area::Ice => "rTransition3",
            _ => "rTransition4",
        },
    }
}
fn controller_alarm(game: &mut Game, s: &mut EndingState, i: usize) {
    match s.stage {
        EndingStage::Volcano => match i {
            0 => {
                spawn(game, s, "oPlayerSil", Vec2::new(240., 132.));
                s.alarms[1] = 30;
            }
            1 => {
                spawn(game, s, "oTreasureSil", Vec2::new(240., 132.));
            }
            2 => {
                let x = 224. + game.rng.range(49) as f64;
                let y = 144. + game.rng.range(9) as f64;
                spawn(game, s, "oVolcanoFlame", Vec2::new(x, y));
                s.alarms[2] = 10 + game.rng.range(11) as i32;
            }
            _ => {}
        },
        EndingStage::Landing if i == 0 => {
            spawn(game, s, "oPDummy2", Vec2::new(144., -32.));
            game.sound("sndPFall", Vec2::new(144., -32.));
        }
        EndingStage::Summary => match i {
            0 => {
                s.draw_status = 1;
                s.alarms[1] = 50;
                s.music = "musVictory".into();
                s.music_looping = false;
                crate::music::play(game, "musVictory", false);
                set_menu(game, true);
            }
            1 => {
                s.draw_status = 2;
                s.alarms[2] = 50;
            }
            2 => s.draw_status = 3,
            11 => {
                s.draw_status = 4;
                if !s.bonus_awarded {
                    game.player.money = game.player.money.saturating_add(50_000);
                    s.money_count = s.money_count.saturating_add(50_000);
                    s.bonus_awarded = true;
                    game.sound("sndThump", Vec2::new(160., 120.));
                }
                s.alarms[3] = 50;
            }
            3 => {
                s.draw_status = 5;
                s.alarms[4] = 10;
            }
            4 => {
                s.draw_status = 6;
                s.alarms[5] = 10;
            }
            5 => s.draw_status = 7,
            _ => {}
        },
        EndingStage::Transition => match i {
            0 => transition_icon(game, s),
            1 => {
                s.draw_loot += 1;
                if s.draw_loot < 0 {
                    s.alarms[1] = if s.hurry_up { 1 } else { 30 };
                }
            }
            _ => {}
        },
        EndingStage::Credits => match i {
            0 => {
                spawn(game, s, "oCamel", Vec2::new(320., 144.));
                if s.kind != EndingKind::TitleCredits {
                    spawn(game, s, "oCaravan", Vec2::new(384., 128.));
                }
            }
            1 => {
                if s.scrolling {
                    let scenery = if game.rng.chance(1, 8) {
                        Some(("oShrubScroll", Vec2::new(-16., 176.)))
                    } else if game.rng.chance(1, 12) {
                        Some(("oPalmTreeScroll", Vec2::new(-32., 64.)))
                    } else {
                        None
                    };
                    if let Some((object, pos)) = scenery {
                        spawn(game, s, object, pos);
                    }
                    for (object, y) in [
                        ("oDesertTopScroll", 176.),
                        ("oDesertScroll2", 192.),
                        ("oDesertScroll", 208.),
                        ("oDesertScroll", 224.),
                    ] {
                        spawn(game, s, object, Vec2::new(-16., y));
                    }
                    s.alarms[1] = 16;
                }
            }
            2..=7 => {
                s.credits_page = (i - 1) as u8;
                s.alarms[11] = if i >= 6 { 240 } else { 140 };
                s.alarms[i + 1] = if i >= 6 { 280 } else { 180 };
            }
            8 => {
                s.scrolling = false;
                for e in &mut game.world.entities {
                    if matches!(e.object.as_str(), "oCamel" | "oCaravan") {
                        e.state = 2;
                    }
                }
            }
            11 => s.credits_page = 0,
            _ => {}
        },
        _ => {}
    }
}

fn actor_alarm(game: &mut Game, s: &mut EndingState, id: u64, i: usize) {
    let Some((object, pos)) = game.world.entity(id).map(|e| (e.object.clone(), e.pos)) else {
        return;
    };
    match object.as_str() {
        "oPDummy" => match i {
            0 => alarm(s, id, 1, 100),
            1 => {
                if let Some(chest) = game
                    .world
                    .entities
                    .iter_mut()
                    .find(|e| e.object == "oBigChest")
                {
                    chest.sprite = "sBigChestOpen".into();
                    let pos = chest.pos;
                    let treasure = spawn(game, s, "oBigTreasure", pos);
                    game.world.entity_mut(treasure).unwrap().vel = Vec2::new(-3., -4.);
                }
                game.sound("sndClick", pos);
                alarm(s, id, 2, 20);
            }
            2 => {
                if let Some(e) = game.world.entity_mut(id) {
                    e.state = 3;
                }
            }
            3 => {
                if let Some(pos) = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == "oEndPlat")
                    .map(|e| e.pos)
                {
                    spawn(game, s, "oLavaSpray", Vec2::new(pos.x, pos.y + 30.));
                }
                game.data.insert("shake".into(), 9999.);
                alarm(s, id, 4, 10);
            }
            4 => {
                if let Some(e) = game
                    .world
                    .entities
                    .iter_mut()
                    .find(|e| e.object == "oLavaSpray")
                {
                    e.data.insert("y_acc".into(), -0.1);
                }
            }
            5 => {
                let run =
                    selected_sprite(game, "sRunLeft", "sDamselRunL", "sTunnelRunL").to_owned();
                if let Some(e) = game.world.entity_mut(id) {
                    e.state = 0;
                    e.sprite = run;
                }
            }
            _ => {}
        },
        "oPDummy2" => match i {
            0 => {
                spawn(game, s, "oBigTreasure", Vec2::new(160., -32.));
                game.sound("sndTFall", Vec2::new(160., -32.));
            }
            1 => {
                let sprite =
                    selected_sprite(game, "sStandLeft", "sDamselLeft", "sTunnelLeft").to_owned();
                if let Some(e) = game.world.entity_mut(id) {
                    e.sprite = sprite;
                }
            }
            2 => {
                s.stage = EndingStage::Summary;
                s.draw_status = 1;
                s.alarms[1] = 50;
                s.music = "musVictory".into();
                s.music_looping = false;
                crate::music::play(game, "musVictory", false);
            }
            _ => {}
        },
        "oLavaSpray" if i == 0 => enter(game, s, EndingStage::Volcano, "rEnd2"),
        "oHeart" if i == 0 => {
            if let Some(e) = game.world.entity_mut(id) {
                e.alive = false;
            }
        }
        _ => {}
    }
}

fn object_at(game: &Game, point: Vec2, object: &str) -> bool {
    let tx = (point.x / 16.).floor() as i32;
    let ty = (point.y / 16.).floor() as i32;
    game.world
        .tile(tx, ty)
        .is_some_and(|t| crate::catalog::is_a(&t.object, object))
        || game.world.entities.iter().any(|e| {
            e.alive && crate::catalog::is_a(&e.object, object) && e.sprite_bounds().contains(point)
        })
}
fn actor_step(game: &mut Game, s: &mut EndingState, e: &mut Entity, input: &InputFrame) {
    match e.object.as_str() {
        "oPDummy" => dummy_step(game, s, e),
        "oPDummy2" => landing_step(game, s, e),
        "oPDummy5" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
            if e.pos.y < -32. {
                s.fade_out = true;
            }
        }
        "oBigTreasure" => treasure_step(game, s, e),
        "oBall2" => {
            crate::items::tick_single_item_body(game, e);
            let family = crate::dispatcher::descendant_instances(game, "oPDummy");
            let mut targets: Vec<_> = game
                .world
                .entities
                .iter()
                .enumerate()
                .filter(|(_, target)| {
                    target.alive
                        && target.engine_active
                        && crate::catalog::is_a(&target.object, "oPDummy")
                })
                .collect();
            // Match the first current-family qualified getter. Raw pending
            // births follow registered entries by creation serial; the query
            // never registers or reactivates instances. Storage only breaks
            // synthetic equal-serial ties, as in moai_revival_anchor.
            targets.sort_by_key(|(storage, target)| {
                family
                    .iter()
                    .position(|instance| {
                        *instance == crate::dispatcher::Instance::Entity(target.id)
                    })
                    .map_or((1_u8, target.creation_serial, *storage), |rank| {
                        (0, rank as u64, *storage)
                    })
            });
            if let Some((_, dummy)) = targets.first() {
                let source = e.collision_mask().point_bounds();
                let distance = targets
                    .iter()
                    .fold(1_000_000.0_f64, |distance, (_, target)| {
                        distance.min(crate::world_events::mask_distance(
                            source,
                            target.collision_mask().point_bounds(),
                        ))
                    });
                if distance >= 24. {
                    // Qualified x uses the first active dummy, independently
                    // of which family member minimized the bounding-box gap.
                    e.pos.x = dummy.pos.x - 24.;
                }
            }
        }
        "oLavaSpray" => {
            e.pos.y += e.vel.y;
            if e.vel.y > -6. {
                e.vel.y += e.value("y_acc");
            }
            if object_at(game, Vec2::new(e.pos.x, e.pos.y - 1.), "oEndPlat") {
                for carried in &mut game.world.entities {
                    if matches!(
                        carried.object.as_str(),
                        "oEndPlat" | "oPDummy" | "oBigTreasure"
                    ) {
                        carried.vel.y = e.vel.y;
                        if carried.object == "oBigTreasure" {
                            carried.data.insert("gravity".into(), 0.);
                        }
                    }
                }
            }
            if e.pos.y < -16. && e.state == 0 {
                e.vel.y = 0.;
                e.data.insert("y_acc".into(), 0.);
                alarm(s, e.id, 0, 40);
                e.state = 1;
            }
            game.sound_if_idle("sndFlame", e.pos);
        }
        "oEndPlat" => e.pos.y += e.vel.y,
        "oPlayerSil" | "oTreasureSil" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
            if e.vel.x < 0. {
                e.vel.x += 0.1;
            }
            if e.vel.y < 6. {
                e.vel.y += 0.6;
            }
            if e.object == "oTreasureSil" && e.pos.y > 240. {
                enter(game, s, EndingStage::Landing, "rEnd3");
            }
        }
        "oDamselKiss" => {
            if matches!(e.sprite.as_str(), "sDamselKissL" | "sPKissL") && e.animation == 7. {
                spawn(game, s, "oHeart", Vec2::new(e.pos.x - 8., e.pos.y - 8.));
                game.sound("sndKiss", e.pos);
            }
        }
        "oWalDamsel" => {
            if e.animation == 11. {
                spawn(game, s, "oHeart", Vec2::new(e.pos.x, e.pos.y + 16.));
                if !s.fade_out {
                    game.sound("sndKiss", e.pos);
                }
            }
        }
        "oCamel" | "oCaravan" => {
            let limit = if e.object == "oCamel" { 160. } else { 224. };
            match e.state {
                0 => {
                    e.pos.x -= 1.;
                    if e.pos.x <= limit {
                        e.state = 1;
                        if e.object == "oCamel" {
                            s.music = "musCredits".into();
                            s.music_looping = false;
                            crate::music::play(game, "musCredits", false);
                        }
                    }
                }
                1 => e.pos.x -= 0.01,
                2 => e.pos.x -= 2.,
                _ => {}
            }
            if e.pos.x < -64. && (e.object == "oCaravan" || s.kind == EndingKind::TitleCredits) {
                s.fade_out = true;
            }
        }
        "oBGEnd3" => {
            if s.stage == EndingStage::Credits
                && game.world.entities.iter().any(|e| e.object == "oCamel")
            {
                let offset = e.value("x_off");
                e.pos.x = game.camera.x + offset;
                e.data.insert("x_off".into(), offset + 0.02);
            }
        }
        "oIntroBG" => {
            // Its source Step first adjusts xOff when the view moves, except
            // in rCredits1 while the Camel exists; it then pins x to the view.
            let has_credits_camel = game.world.name == "rCredits1"
                && game
                    .world
                    .entities
                    .iter()
                    .any(|entity| entity.alive && entity.object == "oCamel");
            let mut offset = e.value("x_off");
            if game.camera.x != 0. && !has_credits_camel {
                offset -= 0.02;
                e.data.insert("x_off".into(), offset);
            }
            e.pos.x = game.camera.x + offset;
        }
        "oDesertScroll" | "oDesertScroll2" | "oDesertTopScroll" | "oShrubScroll"
        | "oPalmTreeScroll" => {
            if s.scrolling {
                e.pos.x += 1.;
            }
            if e.pos.x > 320. {
                e.alive = false;
            }
        }
        "oTunnelMan" if s.stage == EndingStage::Transition => tunnel_step(game, s, e, input),
        "oHeart" => e.pos.y -= 1.,
        "oPoof" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
        }
        "oBurn" => {
            e.pos.y += e.vel.y;
            if game.world.solid_at(e.pos.x, e.pos.y) {
                e.alive = false;
            }
        }
        "oVolcanoFlame" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
            if e.vel.y < 6. {
                e.vel.y += e.value("gravity");
            }
            if !e.sprite_bounds().intersects(Rect::new(
                0.,
                0.,
                game.world.width as f64 * 16.,
                game.world.height as f64 * 16.,
            )) {
                e.alive = false;
            }
        }
        "oCape" => {
            if let Some(dummy) = game.world.entities.iter().find(|a| a.object == "oPDummy") {
                let exit = matches!(
                    dummy.sprite.as_str(),
                    "sPExit" | "sDamselExit" | "sTunnelExit"
                );
                e.pos = Vec2::new(
                    dummy.pos.x + if exit { 0. } else { -4. },
                    dummy.pos.y + if exit { 4. } else { -2. },
                );
                e.sprite = if exit {
                    "sCapeBack"
                } else if matches!(
                    dummy.sprite.as_str(),
                    "sRunLeft" | "sDamselRunL" | "sTunnelRunL"
                ) {
                    "sCapeRight"
                } else {
                    "sCapeDR"
                }
                .into();
                e.data.insert("depth".into(), if exit { 0. } else { 100. });
            } else if s.stage == EndingStage::Transition {
                e.alive = false;
            }
        }
        "oChain2" => {
            if let Some(ball) = game.world.entities.iter().find(|a| a.object == "oBall2") {
                let target = game
                    .world
                    .entities
                    .iter()
                    .find(|a| a.object == "oPDummy")
                    .map(|a| a.pos)
                    .unwrap_or(Vec2::new(280., 184.));
                e.pos = Vec2::new(
                    ball.pos.x + (target.x - ball.pos.x) / 4. * e.value("linkVal"),
                    ball.pos.y + (target.y - ball.pos.y) / 4. * e.value("linkVal"),
                );
            }
        }
        _ => {}
    }
}
fn animate(_game: &mut Game, _s: &mut EndingState, e: &mut Entity) {
    let speed = e.data.get("animation_speed").copied().unwrap_or(0.);
    if speed <= 0. {
        return;
    }
    e.animation += speed;
    let count = crate::catalog::sprite_frame_count(&e.sprite) as f64;
    if count > 0. && e.animation >= count {
        e.animation -= count;
        if e.object == "oPDummy"
            && matches!(e.sprite.as_str(), "sPExit" | "sDamselExit" | "sTunnelExit")
        {
            e.alive = false;
        }
        if e.object == "oDamselKiss" && matches!(e.sprite.as_str(), "sDamselKissL" | "sPKissL") {
            e.data.insert("kissed".into(), 1.);
            e.sprite = if e.sprite == "sPKissL" {
                "sStandLeft"
            } else {
                "sDamselLeft"
            }
            .into();
        }
        if matches!(e.object.as_str(), "oPoof" | "oBurn") {
            e.alive = false;
        }
    }
}

fn dummy_step(game: &mut Game, s: &mut EndingState, e: &mut Entity) {
    e.pos.y += e.vel.y;
    if e.state != 99 {
        let target = game
            .world
            .entities
            .iter()
            .find(|person| {
                person.object == "oDamselKiss"
                    && person.value("kissed") == 0.
                    && person
                        .sprite_bounds()
                        .contains(Vec2::new(e.pos.x + 8., e.pos.y))
            })
            .map(|p| p.id);
        if let Some(id) = target {
            e.state = 99;
            e.vel = Vec2::default();
            e.sprite = selected_sprite(game, "sStandLeft", "sDamselLeft", "sTunnelLeft").into();
            let sprite =
                if crate::characters::selected(game) == crate::characters::Character::Damsel {
                    "sPKissL"
                } else {
                    "sDamselKissL"
                };
            game.world.entity_mut(id).unwrap().sprite = sprite.into();
            alarm(s, e.id, 5, 30);
        }
        let target = game
            .world
            .entities
            .iter()
            .find(|person| {
                person.object == "oTunnelMan"
                    && person.value("talk") == 0.
                    && person
                        .sprite_bounds()
                        .contains(Vec2::new(e.pos.x + 8., e.pos.y))
            })
            .map(|p| p.id);
        if let Some(id) = target {
            e.state = 99;
            e.vel = Vec2::default();
            e.sprite = selected_sprite(game, "sStandLeft", "sDamselLeft", "sTunnelLeft").into();
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("talk".into(), 1.);
        }
    }
    match e.state {
        0 => {
            if e.pos.x >= 280. {
                if !matches!(e.sprite.as_str(), "sPExit" | "sDamselExit" | "sTunnelExit") {
                    e.sprite = selected_sprite(game, "sPExit", "sDamselExit", "sTunnelExit").into();
                    game.sound("sndSteps", e.pos);
                }
            } else {
                e.pos.x += 2.;
            }
        }
        2 => {
            if !matches!(
                e.sprite.as_str(),
                "sStandLeft" | "sDamselLeft" | "sTunnelLeft"
            ) {
                if e.pos.x >= 456. {
                    e.sprite =
                        selected_sprite(game, "sStandLeft", "sDamselLeft", "sTunnelLeft").into();
                    alarm(s, e.id, 0, 20);
                } else {
                    e.pos.x += 2.;
                }
            }
        }
        3 => {
            alarm(s, e.id, 3, 50);
            e.state = 4;
        }
        4 => {
            if let Some(x) = game
                .world
                .entities
                .iter()
                .find(|a| a.object == "oEndPlat")
                .map(|a| a.pos.x)
            {
                let offset = game.rng.range(81) as f64;
                spawn(game, s, "oBurn", Vec2::new(x + offset, 224.));
            }
        }
        _ => {}
    }
}
fn landing_step(game: &mut Game, s: &mut EndingState, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    match e.state {
        0 => {
            e.sprite = selected_sprite(game, "sDieLFall", "sDamselFallL", "sTunnelFallL").into();
            e.vel.y = 6.;
            if object_at(game, Vec2::new(e.pos.x, e.pos.y + 9.), "oDesert2") {
                e.vel.y = 0.;
                e.state = 1;
                e.sprite = selected_sprite(game, "sStunL", "sDamselStunL", "sTunnelStunL").into();
                game.sound("sndThud", e.pos);
                for dx in [-4., 4.] {
                    let id = spawn(game, s, "oPoof", Vec2::new(e.pos.x + dx, e.pos.y + 6.));
                    game.world.entity_mut(id).unwrap().vel.x = dx.signum() * 0.4;
                }
            }
        }
        1 => {
            alarm(s, e.id, 0, 70);
            alarm(s, e.id, 1, 50);
            e.state = 2;
        }
        2 => {
            if game
                .world
                .entities
                .iter()
                .find(|a| a.object == "oBigTreasure")
                .is_some_and(|a| a.vel.y == 0.)
            {
                e.vel.y = -4.;
                e.state = 3;
            }
        }
        3 => {
            e.sprite = selected_sprite(game, "sJumpLeft", "sDamselDieLR", "sTunnelDieLR").into();
            if e.vel.y < 4. {
                e.vel.y += 0.6;
            }
            if object_at(game, Vec2::new(e.pos.x, e.pos.y + 6.), "oDesert2") {
                e.vel.y = 0.;
                e.state = 4;
                alarm(s, e.id, 2, 50);
                e.sprite = selected_sprite(game, "sStandLeft", "sDamselLeft", "sTunnelLeft").into();
                e.facing = 1;
                e.data.insert("image_xscale".into(), -1.);
                set_menu(game, true);
            }
        }
        _ => {}
    }
    if object_at(game, Vec2::new(e.pos.x, e.pos.y + 6.), "oDesert2") {
        e.pos.y -= 2.;
    }
    if object_at(game, Vec2::new(e.pos.x, e.pos.y + 7.), "oDesert2") {
        e.pos.y -= 1.;
    }
}
fn treasure_step(game: &mut Game, s: &mut EndingState, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    if e.vel.y < 6. {
        e.vel.y += e.value("gravity");
    }
    let top = solid_edge(&game.world, Rect::new(e.pos.x, e.pos.y - 1., 32., 1.), e.id);
    let left = solid_edge(&game.world, Rect::new(e.pos.x - 1., e.pos.y, 1., 32.), e.id);
    let right = solid_edge(
        &game.world,
        Rect::new(e.pos.x + 32., e.pos.y, 1., 32.),
        e.id,
    );
    let bottom = solid_edge(
        &game.world,
        Rect::new(e.pos.x, e.pos.y + 32., 32., 1.),
        e.id,
    );
    if top && e.vel.y < 0. {
        e.vel.y = -e.vel.y * 0.8;
    }
    if left || right {
        e.vel.x = -e.vel.x * 0.5;
    }
    if bottom {
        if e.vel.y > 5. {
            for dx in [12., 20.] {
                let id = spawn(game, s, "oPoof", Vec2::new(e.pos.x + dx, e.pos.y + 30.));
                game.world.entity_mut(id).unwrap().vel.x = if dx == 12. { -0.4 } else { 0.4 };
            }
            game.sound("sndThump", e.pos);
        }
        e.vel.y = if e.vel.y > 1. { -e.vel.y * 0.5 } else { 0. };
        e.vel.x = if e.vel.x.abs() < 0.1 {
            0.
        } else {
            e.vel.x * 0.3
        };
    }
    if solid_edge(
        &game.world,
        Rect::new(e.pos.x, e.pos.y + 31., 32., 1.),
        e.id,
    ) && e.vel.y.abs() < 1.
    {
        e.pos.y -= 1.;
        e.vel.y = 0.;
    }
}

fn solid_edge(world: &World, probe: Rect, excluded: u64) -> bool {
    // oBigTreasure calls the shared isCollision* line helpers. Preserve their
    // RTE endpoint conversion, oSolid ancestry, active-instance filter, and
    // held-target visibility instead of treating this as a solid-flag Rect.
    world.solid_line_except(
        Vec2::new(probe.x.round_ties_even(), probe.y.round_ties_even()),
        Vec2::new(
            (probe.x + probe.w - 1.0).round_ties_even(),
            (probe.y + probe.h - 1.0).round_ties_even(),
        ),
        excluded,
    )
}

fn controller_step(game: &mut Game, s: &mut EndingState, input: &InputFrame) {
    match s.stage {
        EndingStage::Exit if s.kind == EndingKind::TitleCredits => {
            if s.fade_out {
                if s.fade_level < 1. {
                    s.fade_level += 0.1;
                } else {
                    enter(game, s, EndingStage::Credits, "rCredits1");
                }
            }
        }
        EndingStage::Chamber => {
            if action(input) && s.stage_tick > 50 {
                enter(game, s, EndingStage::Landing, "rEnd3");
                return;
            }
            game.camera.x = (game.camera.x + 1.).min(game.world.width as f64 * 16. - 320.);
            let shake = game.data.get("shake").copied().unwrap_or(0.);
            if shake > 0. {
                game.camera.y = if s.stage_tick % 2 == 0 {
                    1. + game.rng.range(8) as f64
                } else {
                    0.
                };
                game.data.insert("shake".into(), shake - 1.);
            } else {
                game.camera.y = 0.;
            }
        }
        EndingStage::Volcano => {
            if action(input) {
                enter(game, s, EndingStage::Landing, "rEnd3");
            }
        }
        EndingStage::Summary => {
            if action(input) {
                if s.draw_status == 7 {
                    if s.money_count < game.player.money {
                        s.money_count = game.player.money;
                    } else {
                        s.fade_out = true;
                    }
                } else if s.draw_status == 8 {
                    if s.kind == EndingKind::Custom {
                        finish(game, s);
                    } else {
                        enter(game, s, EndingStage::Credits, "rCredits2");
                    }
                    return;
                }
            }
            if s.draw_status > 2 {
                let diff = game.player.money.saturating_sub(s.money_count);
                s.money_count = s.money_count.saturating_add(if diff > 1000 {
                    1000
                } else if diff > 100 {
                    100
                } else {
                    diff
                });
                if s.draw_status == 3 && diff == 0 && !s.bonus_scheduled {
                    s.alarms[11] = 50;
                    s.bonus_scheduled = true;
                }
            }
            if s.fade_out {
                if s.fade_level < 1. {
                    s.fade_level += 0.1;
                } else {
                    s.draw_status = 8;
                }
            }
        }
        EndingStage::Credits => {
            let camel = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oCamel")
                .map(|e| e.pos.x);
            if (input.attack || input.credits_keyboard_skip_pressed) && camel.is_some() {
                if s.fade_in {
                    s.fade_level = 0.;
                } else {
                    s.fade_out = true;
                }
            }
            if camel.is_some_and(|x| x <= 160.) && !s.scroll_started {
                s.alarms[1] = 1;
                s.alarms[2] = 20;
                s.scroll_started = true;
                s.scrolling = true;
            }
            if s.fade_in {
                if s.fade_level > 0. {
                    s.fade_level -= 0.1;
                } else {
                    s.fade_in = false;
                    s.alarms[0] = 20;
                }
            } else if s.fade_out {
                if s.fade_level < 1. {
                    s.fade_level += 0.1;
                } else {
                    if game.world.name == "rCredits2" {
                        crate::music::stop_all(game);
                    }
                    finish(game, s);
                }
            }
        }
        EndingStage::Transition => {
            // The source writes this gameplay flag in Draw after revealing
            // the time. Apply it once per simulation step so display refresh
            // rates cannot change level-generation eligibility.
            if s.draw_loot > -2 {
                if s.level_time_ms / 1000 % 60 <= 20 {
                    game.flags.insert("no_dark_level".into());
                } else {
                    game.flags.remove("no_dark_level");
                }
            }
            let gate = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oTunnelMan")
                .is_none_or(|e| e.value("talk") >= 3.);
            if action(input) {
                if s.draw_loot == 2 && s.money_count == s.level_money && gate {
                    finish(game, s);
                    return;
                } else {
                    s.hurry_up = true;
                }
            }
            if s.draw_loot == 2 && s.money_count < s.level_money {
                s.money_count = if s.hurry_up {
                    s.level_money
                } else {
                    s.money_count.saturating_add(100).min(s.level_money)
                };
            }
        }
        _ => {}
    }
}
fn finish(game: &mut Game, s: &mut EndingState) {
    s.stage = EndingStage::Complete;
    s.stage_tick = 0;
    s.music.clear();
    match s.kind {
        EndingKind::Campaign => {
            s.record_ready = true;
            game.state = RunState::Won;
            game.flags.insert("won".into());
            game.events.push(crate::model::GameEvent::Won);
        }
        EndingKind::Custom => game.state = RunState::Ending,
        EndingKind::TitleCredits => {
            game.flags.insert("app_exit_requested".into());
            game.state = RunState::Ending;
        }
        EndingKind::Transition => {
            game.state = RunState::Transition;
            game.flags.insert("transition_cinematic_complete".into());
        }
        EndingKind::HubPortal => {
            if let Some(portal) = s.portal {
                crate::hub::complete_portal(game, portal);
            }
        }
    }
}

const LOOT: &[(&str, &str)] = &[
    ("gold", "sGoldChunk"),
    ("emeralds", "sEmerald"),
    ("sapphires", "sSapphire"),
    ("rubies", "sRuby"),
    ("nuggets", "sGoldNugget"),
    ("goldbar", "sGoldBarDraw"),
    ("goldbars", "sGoldBarsDraw"),
    ("bigemeralds", "sEmeraldBig"),
    ("bigsapphires", "sSapphireBig"),
    ("bigrubies", "sRubyBig"),
    ("diamonds", "sDiamond"),
    ("xdamsels", "sDamselLeft"),
    ("scarabs", "sScarabDisp"),
    ("idols", "sGoldIdolIco"),
    ("skulls", "sCrystalSkullIco"),
];
const KILLS: &[(&str, &str)] = &[
    ("bats", "sBatLeft"),
    ("snakes", "sSnakeLeft"),
    ("spiders", "sSpider"),
    ("deadfish", "sDeadFishLeftIco"),
    ("piranhas", "sPiranhaLeftIco"),
    ("skeletons", "sSkeletonLeft"),
    ("zombies", "sZombieLeft"),
    ("vampires", "sVampireLeft"),
    ("frogs", "sFrogLeft"),
    ("firefrogs", "sFireFrogLeft"),
    ("monkeys", "sMonkeyLeft"),
    ("mantraps", "sManTrapLeft"),
    ("yetis", "sYetiLeft"),
    ("ufos", "sUFO"),
    ("aliens", "sAlien"),
    ("alienbosses", "sAlienBossDisp"),
    ("cavemen", "sCavemanLeft"),
    ("hawkmen", "sHawkLeft"),
    ("giantspiders", "sGiantSpiderDisp"),
    ("megamouths", "sMegaMouth"),
    ("yetikings", "sYetiKingDisp"),
    ("tomblords", "sTombLordDisp"),
    ("damselsKilled", "sDamselLeftIco"),
    ("shopkeepers", "sShopLeftIco"),
];
fn consume_icon(game: &mut Game, categories: &[(&str, &str)]) -> Option<String> {
    for &(key, sprite) in categories {
        if let Some(count) = game.data.get_mut(key).filter(|n| **n > 0.) {
            *count -= 1.;
            return Some(
                if crate::characters::selected(game) == crate::characters::Character::Damsel {
                    match key {
                        "xdamsels" => "sStandLeft",
                        "damselsKilled" => "sStandLeftIco",
                        _ => sprite,
                    }
                } else {
                    sprite
                }
                .into(),
            );
        }
    }
    None
}
fn transition_icon(game: &mut Game, s: &mut EndingState) {
    if s.draw_loot == 0 {
        if s.draw_pos.x > 272. {
            s.draw_pos.x = 100.;
            s.draw_pos.y += 2.;
            if s.draw_pos.y > 87. {
                s.draw_pos.y = 83.;
            }
        }
    } else if s.draw_pos.x > 232. {
        s.draw_pos.x = 96.;
        s.draw_pos.y += 2.;
        if s.draw_pos.y > 95. {
            s.draw_pos.y = 91.;
        }
    }
    let mut icon = None;
    if s.draw_loot >= 0 {
        icon = consume_icon(game, LOOT);
        if icon.is_some() {
            s.is_loot = true;
        } else {
            if s.draw_loot == 0 {
                s.draw_loot = 1;
                s.draw_pos = Vec2::new(96., 91.);
            }
            icon = consume_icon(game, KILLS);
            if icon.is_some() {
                s.is_kills = true;
            } else {
                s.draw_loot = 2;
            }
        }
    }
    if let Some(sprite) = icon {
        let id = spawn(game, s, "oSprite", s.draw_pos);
        game.world.entity_mut(id).unwrap().sprite = sprite;
    }
    if s.draw_loot >= 0 {
        s.draw_pos.x += if s.draw_loot == 0 { 4. } else { 8. };
    }
    if s.draw_loot != 2 {
        s.alarms[0] = if s.hurry_up { 1 } else { 3 };
    }
}
fn setup_tunnel(game: &mut Game, s: &mut EndingState, room: &str) {
    if ["hub", "custom", "custom_level", "tutorial"]
        .iter()
        .any(|f| game.flags.contains(*f))
    {
        return;
    }
    let stage = match room {
        "rTransition1x" => 1,
        "rTransition2x" => 2,
        "rTransition3x" => 3,
        _ => return,
    };
    let unlocked = |destination| game.flags.contains(&format!("shortcut_{destination}"));
    let expected = if !unlocked(5) {
        1
    } else if !unlocked(9) {
        2
    } else if !unlocked(13) {
        3
    } else {
        return;
    };
    if expected != stage {
        return;
    }
    let key = format!("tunnel{stage}_remaining");
    let cost = stage as f64 * 100_000.;
    s.transition_tunnel = true;
    // Fresh source scores use max+1, consumed on the first encounter without
    // an offer. Missing persisted balance is the equivalent sentinel.
    let Some(remaining) = game.data.get(&key).copied() else {
        game.data.insert(key, cost);
        return;
    };
    if remaining > cost {
        game.data.insert(key, remaining - 1.);
        return;
    }
    if remaining <= 0. {
        return;
    }
    game.data.insert(
        if stage == 2 { "tunnel2" } else { "tunnel1" }.into(),
        remaining,
    );
    let id = spawn(game, s, "oTunnelMan", Vec2::new(104., 184.));
    let e = game.world.entity_mut(id).unwrap();
    e.data.insert("tunnel_stage".into(), stage as f64);
    e.data.insert("talk".into(), 0.);
    e.data.insert("donate".into(), 0.);
}
fn tunnel_step(game: &mut Game, s: &mut EndingState, e: &mut Entity, input: &InputFrame) {
    let stage = e.value("tunnel_stage") as u8;
    let remaining = game
        .data
        .get(&format!("tunnel{stage}_remaining"))
        .copied()
        .unwrap_or(stage as f64 * 100_000.);
    let mut donate = e.value("donate");
    if input.up_pressed {
        donate += 100.;
    } else if input.down_pressed {
        donate -= 100.;
    }
    donate = donate.max(0.).min(remaining).min(game.player.money as f64);
    let talk = e.value("talk") as u8;
    if input.attack_pressed {
        if talk == 1 {
            e.data.insert("talk".into(), 2.);
        } else if talk == 2 {
            let new_talk = if donate > 0. {
                if donate >= remaining { 5 } else { 3 }
            } else {
                4
            };
            e.data.insert("talk".into(), new_talk as f64);
            if donate > 0. {
                crate::progression::apply_donation(game, stage, donate as u32);
                game.data.insert(
                    if stage == 2 { "tunnel2" } else { "tunnel1" }.into(),
                    (remaining - donate).max(0.),
                );
            }
            let sprite = selected_sprite(game, "sRunLeft", "sDamselRunL", "sRunLeft").to_owned();
            if let Some(dummy) = game
                .world
                .entities
                .iter_mut()
                .find(|a| a.object == "oPDummy")
            {
                dummy.state = 0;
                dummy.sprite = sprite;
            }
        }
    }
    if e.value("talk") == 2. {
        let direction = if input.up {
            1.
        } else if input.down {
            -1.
        } else {
            0.
        };
        let counter_key = if direction > 0. {
            "up_counter"
        } else {
            "down_counter"
        };
        let hold_key = if direction > 0. {
            "up_held"
        } else {
            "down_held"
        };
        if direction != 0. {
            let held = e.value(hold_key) + 1.;
            let counter = e.value(counter_key);
            e.data.insert(hold_key.into(), held);
            if counter < 20. {
                e.data.insert(counter_key.into(), counter + 1.);
            } else {
                donate += direction * if held > 100. { 1000. } else { 100. };
            }
            e.data.insert(
                if direction > 0. {
                    "down_counter"
                } else {
                    "up_counter"
                }
                .into(),
                0.,
            );
            e.data.insert(
                if direction > 0. {
                    "down_held"
                } else {
                    "up_held"
                }
                .into(),
                0.,
            );
        } else {
            for key in ["up_counter", "down_counter", "up_held", "down_held"] {
                e.data.insert(key.into(), 0.);
            }
        }
    }
    e.data.insert(
        "donate".into(),
        donate.max(0.).min(remaining).min(game.player.money as f64),
    );
}

#[cfg(test)]
mod ball2_cached_distance_tests {
    use super::*;
    use crate::RulesPreset;

    fn fixture(rules: RulesPreset) -> (Game, EndingState, Entity) {
        let game = Game::shell(42, rules);
        let state = EndingState::new(&game, EndingKind::Transition, EndingStage::Transition);
        let mut ball = Entity::new(900, "oBall2", Vec2::new(80.0, 104.0));
        crate::items::init_entity(&mut ball);
        for (key, value) in [
            ("bboxLeft", -5.0),
            ("bboxTop", -5.0),
            ("bboxRight", 5.0),
            ("bboxBottom", 5.0),
            ("heavy", 1.0),
            ("gravity", 1.0),
        ] {
            ball.data.insert(key.into(), value);
        }
        assert_eq!(
            ball.collision_mask().point_bounds(),
            Rect::new(75.0, 100.0, 10.0, 10.0)
        );
        (game, state, ball)
    }

    fn dummy(game: &mut Game, x: f64, scale: f64, active: bool) -> u64 {
        let id = game.world.spawn("oPDummy", Vec2::new(x, 104.0));
        let target = game.world.entity_mut(id).unwrap();
        target.set_sprite("sRunLeft");
        target.animation = 2.0;
        target.data.insert("image_xscale".into(), scale);
        target.engine_active = active;
        id
    }

    #[test]
    fn configured_ball2_uses_aggregate_and_previous_draw_mirror_at_gap24() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (x, scale, expected) in [
                (115.0, 1.0, 91.0),
                (114.0, 1.0, 80.0),
                (115.0, -1.0, 80.0),
                (116.0, -1.0, 92.0),
            ] {
                let (mut game, mut state, mut ball) = fixture(rules);
                dummy(&mut game, x, scale, true);
                actor_step(&mut game, &mut state, &mut ball, &InputFrame::default());
                assert_eq!(
                    ball.pos.x, expected,
                    "BALL2_NATIVE_AGGREGATE_GAP24 x={x} scale={scale}"
                );
                assert_eq!(
                    ball.pos.y, 104.0,
                    "zero incoming velocity keeps first inherited displacement still"
                );
            }
        }
    }

    #[test]
    fn configured_ball2_skips_inactive_and_retired_dummy_targets() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for retired in [false, true] {
                let (mut game, mut state, mut ball) = fixture(rules);
                let id = dummy(&mut game, 160.0, 1.0, retired);
                if retired {
                    game.world.entity_mut(id).unwrap().alive = false;
                }
                actor_step(&mut game, &mut state, &mut ball, &InputFrame::default());
                assert_eq!(
                    ball.pos.x, 80.0,
                    "instance_exists must not admit this target"
                );
            }
        }
    }

    #[test]
    fn configured_ball2_family_distance_and_qualified_x_are_independent() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (second_x, expected) in [(110.0, 80.0), (115.0, 136.0)] {
                let (mut game, mut state, mut ball) = fixture(rules);
                // Pending births retain their creation-serial family order.
                // First far dummy supplies qualified x, but the second can
                // make the minimum distance smaller than24 and skip the write.
                dummy(&mut game, 160.0, 1.0, true);
                dummy(&mut game, second_x, 1.0, true);
                actor_step(&mut game, &mut state, &mut ball, &InputFrame::default());
                assert_eq!(ball.pos.x, expected);
            }
        }
    }

    #[test]
    fn configured_ball2_qualified_x_uses_registered_family_then_pending_serial_order() {
        use crate::dispatcher::{self, Instance};
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, mut state, mut ball) = fixture(rules);
            let c = dummy(&mut game, 320.0, 1.0, true);
            let b = dummy(&mut game, 192.0, 1.0, true);
            let a = dummy(&mut game, 160.0, 1.0, true);
            dispatcher::register_instances(&mut game);
            // Actual activation APIs establish C,A,B family order while
            // entity storage remains C,B,A. Retiring C leaves A first.
            game.world.entity_mut(a).unwrap().engine_active = false;
            dispatcher::activate_region(&mut game, Rect::new(316.0, 100.0, 24.0, 24.0));
            game.world.entity_mut(b).unwrap().engine_active = false;
            dispatcher::activate_region(&mut game, Rect::new(316.0, 100.0, 24.0, 24.0));
            dispatcher::activate_object(&mut game, "oPDummy");
            assert_eq!(
                dispatcher::descendant_instances(&game, "oPDummy"),
                vec![
                    Instance::Entity(c),
                    Instance::Entity(a),
                    Instance::Entity(b)
                ]
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oPDummy")
                    .map(|e| e.id)
                    .collect::<Vec<_>>(),
                vec![c, b, a]
            );
            game.world.entity_mut(c).unwrap().alive = false;
            let ledger = game.dispatcher.registration_order.clone();
            actor_step(&mut game, &mut state, &mut ball, &InputFrame::default());
            assert_eq!(
                ball.pos.x, 136.0,
                "first live family A supplies x, not storage B"
            );
            assert_eq!(
                game.dispatcher.registration_order, ledger,
                "query leaves the family ledger unchanged"
            );

            let (mut game, mut state, mut ball) = fixture(rules);
            let a = dummy(&mut game, 160.0, 1.0, true);
            let b = dummy(&mut game, 192.0, 1.0, true);
            game.world.entities.reverse();
            assert_eq!(
                game.world.entities.iter().map(|e| e.id).collect::<Vec<_>>(),
                vec![b, a]
            );
            assert!(dispatcher::descendant_instances(&game, "oPDummy").is_empty());
            let ledger = game.dispatcher.registration_order.clone();
            actor_step(&mut game, &mut state, &mut ball, &InputFrame::default());
            assert_eq!(
                ball.pos.x, 136.0,
                "unregistered pending births follow creation serial"
            );
            assert_eq!(
                game.dispatcher.registration_order, ledger,
                "query does not register pending births"
            );
        }
    }
}

// Exercise source donation edges independently of aggregate held controls.
#[cfg(test)]
mod explicit_transition_direction_edge_tests {
    use super::*;
    use crate::RulesPreset;
    #[test]
    fn transition_selection_keeps_short_and_second_device_direction_presses() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (up, held) in [(true, false), (true, true), (false, false), (false, true)] {
                let mut game = Game::shell(7, rules);
                game.player.money = 1000;
                game.data.insert("tunnel1_remaining".into(), 1000.);
                let mut state =
                    EndingState::new(&game, EndingKind::Transition, EndingStage::Transition);
                state.previous_up = up && held;
                state.previous_down = !up && held;
                let mut actor = Entity::new(900, "oTunnelMan", Vec2::new(104., 184.));
                actor.data.insert("tunnel_stage".into(), 1.);
                actor.data.insert("talk".into(), 1.);
                actor.data.insert("donate".into(), 300.);
                let input = InputFrame {
                    up: up && held,
                    down: !up && held,
                    up_pressed: up,
                    down_pressed: !up,
                    ..Default::default()
                };
                tunnel_step(&mut game, &mut state, &mut actor, &input);
                assert_eq!(
                    actor.value("donate"),
                    if up { 400. } else { 200. },
                    "explicit source edge up={up}, aggregate held={held}"
                );
            }
        }
    }
}

#[cfg(test)]
mod direct_transition_action_compatibility {
    use super::*;
    use crate::RulesPreset;
    fn configured_transition(rules: RulesPreset) -> (Game, u64) {
        let mut game = Game::shell(7, rules);
        game.player.money = 1000;
        game.data.insert("tunnel1_remaining".into(), 1000.);
        let mut state = EndingState::new(&game, EndingKind::Transition, EndingStage::Transition);
        // Initial source-room creation, not a mid-run actor/state rewrite.
        enter(
            &mut game,
            &mut state,
            EndingStage::Transition,
            "rTransition1x",
        );
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oTunnelMan")
            .unwrap()
            .id;
        assert_eq!(game.world.entity(id).unwrap().value("donate"), 0.);
        assert_eq!(game.world.entity(id).unwrap().value("talk"), 0.);
        game.state = RunState::Ending;
        game.ending = Some(state);
        (game, id)
    }
    #[test]
    fn direct_cinematic_and_game_step_keep_held_only_selection_compatibility() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for direct in [false, true] {
                let (mut game, id) = configured_transition(rules);
                let advance = |game: &mut Game, input: &InputFrame| {
                    if direct {
                        tick(game, input);
                    } else {
                        game.step(input);
                    }
                };
                for _ in 0..3 {
                    advance(
                        &mut game,
                        &InputFrame {
                            up: true,
                            ..Default::default()
                        },
                    );
                    assert_eq!(game.world.entity(id).unwrap().value("donate"), 100.);
                }
                advance(&mut game, &InputFrame::default());
                advance(
                    &mut game,
                    &InputFrame {
                        down: true,
                        ..Default::default()
                    },
                );
                assert_eq!(game.world.entity(id).unwrap().value("donate"), 0.);
                advance(
                    &mut game,
                    &InputFrame {
                        down: true,
                        ..Default::default()
                    },
                );
                assert_eq!(game.world.entity(id).unwrap().value("donate"), 0.);
                // A short explicit press survives aggregate-held=false.
                advance(
                    &mut game,
                    &InputFrame {
                        up_pressed: true,
                        ..Default::default()
                    },
                );
                assert_eq!(game.world.entity(id).unwrap().value("donate"), 100.);
            }
        }
    }
}
