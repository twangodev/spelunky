//! Original narrated introduction and first title entrance, at source 30 Hz.
use crate::{Game, InputFrame, PlayerState, RunState, Vec2, hub, items, world_events};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningKind {
    Intro,
    Title,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningState {
    pub kind: OpeningKind,
    pub stage: u8,
    pub draw_status: i8,
    pub fade_level: f64,
    #[serde(default = "opaque")]
    pub previous_fade_level: f64,
    pub fade_in: bool,
    pub fade_out: bool,
    pub darkness: f64,
    #[serde(default = "opaque")]
    pub previous_darkness: f64,
    pub can_pause: bool,
    pub player_present: bool,
    pub lines: [String; 3],
    alarms: [i32; 12],
    dummy: Option<u64>,
    dummy_status: u8,
    dummy_alarms: [i32; 3],
    climb_sound_toggle: bool,
    need_dark: bool,
}
fn opaque() -> f64 {
    1.
}
impl OpeningState {
    fn new(kind: OpeningKind) -> Self {
        Self {
            kind,
            stage: 0,
            draw_status: 0,
            fade_level: 1.,
            previous_fade_level: 1.,
            fade_in: false,
            fade_out: false,
            darkness: 1.,
            previous_darkness: 1.,
            can_pause: false,
            player_present: false,
            lines: Default::default(),
            alarms: [-1; 12],
            dummy: None,
            dummy_status: 0,
            dummy_alarms: [-1; 3],
            climb_sound_toggle: false,
            need_dark: true,
        }
    }
}
const LINES: [[&str; 8]; 3] = [
    [
        "AS THE MOON BURNED BRIGHT ABOVE,",
        "WITH THE DESERT STRETCHING BEHIND ME,",
        "AFTER I DOUBLE-CHECKED MY MAP,",
        "MY LIPS CRACKED AND COVERED IN SAND,",
        "WITH FATE GUIDING MY EVERY MOVE,",
        "PUTTING THE FADED PHOTO IN MY POCKET,",
        "AS I RECALLED MY FATHER'S LAST WORDS,",
        "MY MEMORY SLIPPING AWAY FROM ME,",
    ],
    [
        "I STRODE VALIANTLY TOWARD MY DESTINY,",
        "I SQUEEZED THE WHIP AT MY SIDE,",
        "I DRAINED THE REST OF MY CANTEEN,",
        "I SPOTTED THE CAVE'S ENTRANCE,",
        "I FURROWED MY BROW,",
        "I PAID MY BEDOUIN GUIDE,",
        "I DISMOUNTED MY CAMEL,",
        "I SQUINTED INTO THE DARKNESS,",
    ],
    [
        "AND THOUGHT OF HER ONE LAST TIME.",
        "AND HELD MY HAT AGAINST THE WIND.",
        "AND A COLD CHILL TOOK HOLD OF ME.",
        "AND WONDERED HOW LONG I'D BE BELOW.",
        "AND VOWED TO RETURN VICTORIOUS.",
        "AND SWORE I HEARD VOICES UP AHEAD.",
        "AND FELT THE GODS SMILING UPON ME.",
        "AND THAT'S WHEN IT ALL STARTED.",
    ],
];
fn hide_player(game: &mut Game) {
    game.player.data.insert("hidden".into(), 1.);
    game.player.data.insert("active".into(), 0.);
    // There is no source player instance. Keep the mandatory Rust record away
    // from inherited item/contact queries until the real source Create point.
    game.player.pos = Vec2::new(-10000., -10000.);
    game.player.previous_pos = game.player.pos;
    game.player.vel = Vec2::default();
}

fn setup_intro_walls(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            e.object == "oTerrainMetadata"
                && game
                    .world
                    .tile((e.pos.x / 16.) as i32, (e.pos.y / 16.) as i32)
                    .is_some_and(|t| t.object == "oDark")
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        let pos = game.world.entity(id).unwrap().pos;
        let x = (pos.x / 16.) as i32;
        let y = (pos.y / 16.) as i32;
        let dark = |dy| {
            game.world
                .tile(x, y + dy)
                .is_some_and(|t| t.object == "oDark")
        };
        let up = y == 0 || dark(-1);
        let down = y * 16 >= 224 || dark(1);
        let sprite = match (up, down) {
            (false, false) => "sDarkUp2",
            (false, true) => "sDarkUp",
            (true, false) => "sDarkDown",
            _ => "sDark",
        };
        game.world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), sprite.into());
        if !up && !game.flags.contains("low_graphics") {
            let crop = if game.rng.range(3) < 2 { 0. } else { 16. };
            let id = game.spawn(
                "oGenerationDecoration",
                Vec2::new(f64::from(x * 16), f64::from(y * 16 - 16)),
            );
            let e = game.world.entity_mut(id).unwrap();
            e.set_sprite("bgCaveTop3");
            for (key, value) in [
                ("decoration", 1.),
                ("source_x", crop),
                ("source_y", 0.),
                ("source_w", 16.),
                ("source_h", 16.),
                ("depth", 3.),
            ] {
                e.data.insert(key.into(), value);
            }
        }
    }
    // oBricks is a one-shot room setup controller, not a drawn live actor.
    for e in &mut game.world.entities {
        if e.object == "oBricks" {
            e.alive = false;
        }
    }
}

pub fn begin_intro(game: &mut Game) {
    game.draw_view = None;
    game.world = hub::authored_world("rIntro");
    game.state = RunState::Playing;
    game.ending = None;
    game.dispatcher = Default::default();
    game.game_over = Default::default();
    game.alarms = crate::dispatcher::unset_alarms();
    game.level_tick = 0;
    game.flags
        .retain(|f| f != "hub" && !f.starts_with("hub_") && f != "tutorial" && f != "custom");
    game.camera = Vec2::default();
    game.previous_camera = game.camera;
    game.events.clear();
    let mut state = OpeningState::new(OpeningKind::Intro);
    state.alarms[11] = 20;
    // Source room Create visits authored instances in order. oDark consumes
    // its ordinary Create roll even though rIntro suppresses its gold variants.
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../assets/hub.json")).unwrap();
    for instance in data["rooms"]["rIntro"]["instances"].as_array().unwrap() {
        match instance["object"].as_str().unwrap() {
            "oDark" => {
                game.rng.range(100);
            }
            "oIntro" => {
                for (i, choices) in LINES.iter().enumerate() {
                    state.lines[i] = choices[game.rng.range(8) as usize].into();
                }
            }
            "oBricks" => setup_intro_walls(game),
            _ => {}
        }
    }
    for e in &mut game.world.entities {
        let sprite = match e.object.as_str() {
            "oDesert" => Some("sDesertNight"),
            "oDesert2" => Some("sDesertNight2"),
            "oDesertTop" => Some("sDesertTopNight"),
            _ => None,
        };
        if let Some(sprite) = sprite {
            e.set_sprite(sprite);
        }
        items::init_entity(e);
        e.previous_pos = e.pos;
    }
    hide_player(game);
    game.opening = Some(state);
}

/// Start zero is the source alarm-driven entrance. Completed/return starts
/// already have their source player and Flare created by the hub controller.
pub fn begin_title(game: &mut Game, start: u8) {
    game.draw_view = None;
    if start != 0 {
        game.opening = None;
        return;
    }
    let mut state = OpeningState::new(OpeningKind::Title);
    state.alarms[0] = 50;
    hide_player(game);
    game.camera = Vec2::new(320., 0.);
    game.previous_camera = game.camera;
    game.opening = Some(state);
}
fn due(alarms: &mut [i32]) -> Vec<usize> {
    let mut result = Vec::new();
    for (i, alarm) in alarms.iter_mut().enumerate() {
        if *alarm > 0 {
            *alarm -= 1;
            if *alarm == 0 {
                *alarm = -1;
                result.push(i);
            }
        }
    }
    result
}
fn dummy(game: &mut Game, state: &mut OpeningState, object: &str, pos: Vec2) {
    let id = game.spawn(object, pos);
    let e = game.world.entity_mut(id).unwrap();
    e.set_sprite("sRunLeft");
    e.data.insert("animation_speed".into(), 0.6);
    e.facing = if object == "oPDummy3" { 1 } else { -1 };
    state.dummy = Some(id);
}
fn climb_sound(game: &mut Game, state: &mut OpeningState) {
    let pos = state
        .dummy
        .and_then(|id| game.world.entity(id))
        .map(|e| e.pos)
        .unwrap_or_default();
    game.sound(
        if state.climb_sound_toggle {
            "sndClimb1"
        } else {
            "sndClimb2"
        },
        pos,
    );
    state.climb_sound_toggle = !state.climb_sound_toggle;
}
fn alarms(game: &mut Game, state: &mut OpeningState) {
    let actor_due = due(&mut state.dummy_alarms);
    for alarm in due(&mut state.alarms) {
        match (state.kind, alarm) {
            (OpeningKind::Intro, 11 | 10 | 9) if !state.fade_in => {
                if state.draw_status >= 0 {
                    state.draw_status = (12 - alarm) as i8;
                }
                state.alarms[alarm - 1] = 80;
            }
            (OpeningKind::Intro, 8) if !state.fade_in => {
                state.draw_status = -1;
                state.fade_in = true;
            }
            (OpeningKind::Title, 0) => {
                state.stage = 1;
                state.alarms[1] = 100;
            }
            (OpeningKind::Title, 1) => {
                state.stage = 2;
                state.alarms[2] = 70;
            }
            (OpeningKind::Title, 2) => {
                state.stage = 3;
                game.spawn("oFlare", Vec2::new(600., -8.));
                game.sound("sndIgnite", Vec2::new(600., -8.));
                state.alarms[3] = 50;
            }
            (OpeningKind::Title, 3) => {
                dummy(game, state, "oPDummy4", Vec2::new(600., -32.));
                state.can_pause = true;
            }
            _ => {}
        }
    }
    for alarm in actor_due {
        if alarm == 2 {
            climb_sound(game, state);
        } else if alarm == 0 && state.kind == OpeningKind::Intro {
            if let Some(pos) = state
                .dummy
                .and_then(|id| game.world.entity(id))
                .map(|e| e.pos)
            {
                let id = game.spawn("oRopeThrow", Vec2::new(pos.x + 16., pos.y));
                let e = game.world.entity_mut(id).unwrap();
                items::init_entity(e);
                e.data.insert("falling".into(), 1.);
                e.data.insert("armed".into(), 1.);
                state.dummy_alarms[1] = 50;
                state.dummy_status = 3;
                game.sound("sndThrow", pos);
            }
        }
    }
}
fn actor_step(game: &mut Game, state: &mut OpeningState) {
    let Some(id) = state.dummy else {
        return;
    };
    let Some(index) = game
        .world
        .entities
        .iter()
        .position(|e| e.id == id && e.alive)
    else {
        return;
    };
    let mut e = game.world.entities[index].clone();
    e.pos.y += e.vel.y;
    if state.kind == OpeningKind::Title {
        e.pos.x += e.vel.x;
        if e.pos.y >= 160. {
            e.set_sprite("sFallLeft");
            e.vel.y = 4.2;
        } else {
            e.set_sprite("sClimbUp3");
            e.vel.y = 2.;
            if state.dummy_alarms[2] < 1 {
                state.dummy_alarms[2] = 8;
            }
        }
        if e.pos.y >= 184. {
            e.pos.y = 184.;
            e.alive = false;
            state.player_present = true;
            game.player.pos = e.pos;
            game.player.previous_pos = e.pos;
            game.player.vel = Vec2::default();
            game.player.facing = -1;
            game.player.state = PlayerState::Falling;
            game.player.data.remove("hidden");
            game.player.data.insert("active".into(), 1.);
            game.player.animation = 0.;
            game.player
                .set_sprite(match crate::characters::selected(game) {
                    crate::characters::Character::Explorer => "sStandLeft",
                    crate::characters::Character::Damsel => "sDamselLeft",
                    crate::characters::Character::TunnelMan => "sTunnelLeft",
                });
            game.player.data.insert("depth".into(), 50.);
        }
    } else {
        match state.dummy_status {
            0 => {
                if e.pos.x >= 904. {
                    e.set_sprite("sDuckLeft");
                    state.dummy_status = 1;
                } else {
                    e.pos.x += 2.;
                }
            }
            1 => {
                state.dummy_alarms[0] = 20;
                state.dummy_status = 2;
            }
            3 => {
                e.set_sprite("sRunLeft");
                if e.pos.x >= 920. {
                    e.set_sprite("sClimbUp3");
                    state.dummy_status = 4;
                } else {
                    e.pos.x += 2.;
                }
            }
            4 => {
                if e.pos.y >= 256. {
                    state.fade_out = true;
                } else {
                    e.pos.y += 2.;
                }
                if state.dummy_alarms[2] < 1 {
                    state.dummy_alarms[2] = 8;
                }
            }
            _ => {}
        }
    }
    game.world.entities[index] = e;
}
fn intro_props(game: &mut Game) {
    let camera = game.camera.x;
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive)
        .map(|e| e.id)
        .collect();
    for id in ids {
        let Some(mut e) = game.world.entity(id).cloned() else {
            continue;
        };
        match e.object.as_str() {
            "oIntroBG" | "oMoon" => {
                let speed = if e.object == "oMoon" { 0.01 } else { 0.02 };
                let offset = e.value("xOff") - if camera != 0. { speed } else { 0. };
                e.data.insert("xOff".into(), offset);
                e.pos.x = camera + offset + if e.object == "oMoon" { 208. } else { 0. };
            }
            "oBatIntro" => {
                e.pos.x += e.vel.x;
                e.pos.y += e.vel.y;
                e.data.insert("animation_speed".into(), 1.);
                if e.state == 0 && e.pos.x < camera + 336. {
                    e.state = 1;
                    e.vel.x = -game.rng.unit() * 3. - 2.;
                    e.vel.y = -game.rng.unit();
                    e.data.insert("yAcc".into(), -game.rng.unit() * 0.2);
                    game.sound("sndBat", e.pos);
                }
            }
            _ => continue,
        }
        *game.world.entity_mut(id).unwrap() = e;
    }
}

fn passive_initialize(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.value("opening_created") == 0.)
        .map(|e| e.id)
        .collect();
    for id in ids {
        let object = game.world.entity(id).unwrap().object.clone();
        if items::handles(&object) {
            items::create_instance(game, id);
        } else if world_events::handles(&object) {
            world_events::init_entity(game.world.entity_mut(id).unwrap());
        }
        if let Some(e) = game.world.entity_mut(id) {
            e.data.insert("opening_created".into(), 1.);
        }
    }
}
fn passive_alarms(game: &mut Game) {
    for index in 0..12 {
        let ids: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive)
            .map(|e| e.id)
            .collect();
        for id in ids {
            let Some(e) = game.world.entity_mut(id) else {
                continue;
            };
            if e.alarms[index] <= 0 {
                continue;
            }
            e.alarms[index] -= 1;
            if e.alarms[index] != 0 {
                continue;
            }
            e.alarms[index] = -1;
            let object = e.object.clone();
            if items::handles(&object) {
                items::alarm(game, id, index as u8);
            } else if world_events::handles(&object) {
                world_events::alarm(game, id, index as u8);
            }
        }
    }
}
fn passive_steps(game: &mut Game) {
    passive_initialize(game);
    world_events::begin_step(game);
    let mut ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive)
        .map(|e| {
            (
                crate::catalog::object(&e.object).map_or(i32::MAX, |o| o.id),
                e.id,
            )
        })
        .collect();
    ids.sort_unstable();
    for (_, id) in &ids {
        let Some(object) = game.world.entity(*id).map(|e| e.object.clone()) else {
            continue;
        };
        if items::handles(&object) {
            items::step_instance(game, *id);
        } else if world_events::handles(&object) {
            world_events::step_instance(game, *id);
        }
    }
    for (_, id) in ids {
        if game
            .world
            .entity(id)
            .is_some_and(|e| items::handles(&e.object))
        {
            items::builtin_motion(game, id);
            items::end_instance(game, id);
        }
    }
}

/// Exclusive authored-room controller while there is no source player instance.
pub fn tick(game: &mut Game, input: &InputFrame) {
    let Some(mut state) = game.opening.take() else {
        return;
    };
    state.previous_fade_level = state.fade_level;
    state.previous_darkness = state.darkness;
    game.events.clear();
    game.previous_camera = game.camera;
    game.player.previous_pos = game.player.pos;
    for e in &mut game.world.entities {
        e.previous_pos = e.pos;
        e.age = e.age.saturating_add(1);
        items::init_entity(e);
    }
    game.tick += 1;
    game.level_tick += 1;
    passive_initialize(game);
    passive_alarms(game);
    alarms(game, &mut state);
    let skip = input.attack_pressed || input.start_pressed;
    if state.kind == OpeningKind::Title && skip && !state.can_pause {
        game.flags.insert("title_start:2".into());
        game.load_hub(hub::HubKind::Title);
        return;
    }
    if state.kind == OpeningKind::Intro {
        if skip {
            if state.dummy.is_none() {
                state.fade_in = true;
            } else {
                state.fade_in = false;
                state.fade_out = true;
            }
        }
        if state.fade_in {
            state.draw_status = -1;
            if state.fade_level > 0. {
                state.fade_level -= 0.1;
            } else {
                state.fade_in = false;
                if state.dummy.is_none() {
                    dummy(game, &mut state, "oPDummy3", Vec2::new(-32., 184.));
                }
            }
        } else if state.fade_out {
            if state.fade_level < 1. {
                state.fade_level += 0.1;
            } else {
                game.flags.insert("title_start:0".into());
                game.load_hub(hub::HubKind::Title);
                return;
            }
        }
        intro_props(game);
    }
    // Passive objects precede the later source dummy resource. The absent
    // player's Step and contacts are never dispatched.
    passive_steps(game);
    actor_step(game, &mut state);
    if state.kind == OpeningKind::Title && state.need_dark {
        if let Some(flare) = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oFlare")
        {
            state.darkness = (160. - flare.pos.y).max(0.) / 160.;
            if state.darkness == 0. {
                state.need_dark = false;
            }
        }
    }
    if let Some(id) = state.dummy {
        if let Some(e) = game.world.entity_mut(id) {
            if state.kind == OpeningKind::Intro {
                game.camera = Vec2::new((e.pos.x - 160.).clamp(0., 640.), 0.);
            }
        }
    }
    game.world.entities.retain(|e| e.alive);
    let player_present = state.player_present;
    // Source Draw observes the completed Step and view calculations, before
    // automatic image advancement or Animation End callbacks.
    game.opening = Some(state);
    game.capture_draw_view();
    crate::dispatcher::animate_entities(game);
    if player_present {
        game.opening = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;
    fn title() -> Game {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.flags.insert("first_time".into());
        game.flags.insert("title_start:0".into());
        game.load_hub(hub::HubKind::Title);
        game
    }
    #[test]
    fn input_skipped_intro_captures_draw_before_animation_and_clears_old_room_view() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.capture_draw_view();
        begin_intro(&mut game);
        assert!(game.draw_view.is_none());
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..20 {
            if game.world.entities.iter().any(|e| e.object == "oPDummy3") {
                break;
            }
            game.step(&InputFrame::default());
        }
        let live = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oPDummy3")
            .unwrap();
        let drawn = game.presentation().world.entity(live.id).unwrap();
        assert_eq!(drawn.pos, live.pos);
        assert_eq!(drawn.animation, 0.);
        assert_eq!(live.animation, 0.6);
        assert_eq!(game.presentation().camera, game.camera);
        assert_eq!(
            game.presentation().opening.as_ref().unwrap().fade_level,
            game.opening.as_ref().unwrap().fade_level
        );
        assert!(game.presentation().draw_view.is_none());
        begin_title(&mut game, 0);
        assert!(game.draw_view.is_none());
        game.capture_draw_view();
        begin_title(&mut game, 2);
        assert!(game.draw_view.is_none());
    }
    #[test]
    fn title_source_alarms_precede_live_player_and_ignore_movement() {
        let mut game = title();
        assert_eq!(game.opening.as_ref().unwrap().stage, 0);
        assert_eq!(
            game.opening.as_ref().unwrap().previous_darkness,
            game.opening.as_ref().unwrap().darkness
        );
        assert_eq!(
            game.opening.as_ref().unwrap().previous_fade_level,
            game.opening.as_ref().unwrap().fade_level
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oFlare")
                .count(),
            0
        );
        let mut ignites = 0;
        for tick in 1..=270 {
            let previous_darkness = game.opening.as_ref().unwrap().darkness;
            game.step(&InputFrame {
                left: true,
                jump: true,
                up: true,
                ..Default::default()
            });
            let state = game
                .opening
                .as_ref()
                .expect("source player has not arrived");
            assert_eq!(state.previous_darkness, previous_darkness);
            assert!(!state.player_present);
            assert_eq!(
                state.stage,
                if tick < 50 {
                    0
                } else if tick < 150 {
                    1
                } else if tick < 220 {
                    2
                } else {
                    3
                }
            );
            assert_eq!(state.can_pause, tick >= 270);
            assert_eq!(game.camera, Vec2::new(320., 0.));
            assert_eq!(game.player.data.get("hidden"), Some(&1.));
            ignites += game
                .events
                .iter()
                .filter(
                    |event| matches!(event,crate::GameEvent::Sound{name,..} if name=="sndIgnite"),
                )
                .count();
        }
        assert_eq!(ignites, 1);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oPDummy4")
                .count(),
            1
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .find(|e| e.object == "oPDummy4")
                .unwrap()
                .pos,
            Vec2::new(600., -32.)
        );
        for _ in 0..150 {
            if game.opening.is_none() {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert!(game.opening.is_none());
        assert_eq!(game.player.pos, Vec2::new(600., 184.));
        assert_eq!(game.player.facing, -1);
        assert_eq!(game.player.data.get("active"), Some(&1.));
        assert!(game.flags.contains("first_time"));
        assert!(!game.world.entities.iter().any(|e| e.object == "oHintHand"));
        for _ in 0..4 {
            game.step(&InputFrame {
                left: true,
                ..Default::default()
            });
        }
        assert!(game.player.pos.x < 600.);
    }
    #[test]
    fn each_title_skip_phase_restarts_completed_entry_without_action_leak() {
        for wait in [0, 60, 160, 230] {
            let mut game = title();
            for _ in 0..wait {
                game.step(&InputFrame::default());
            }
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert!(game.opening.is_none());
            assert_eq!(game.player.pos, Vec2::new(600., 184.));
            assert_eq!(game.player.attack_timer, 0);
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oFlare")
                    .count(),
                1
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oHintHand")
                    .count(),
                1
            );
            assert!(!game.world.entities.iter().any(|e| e.object == "oPDummy4"));
            assert!(game.flags.contains("first_time"));
        }
    }
    #[test]
    fn title_skip_stops_when_alarm_three_enables_pause() {
        let mut game = title();
        for _ in 0..270 {
            game.step(&InputFrame::default());
        }
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oPDummy4")
            .unwrap()
            .id;
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.opening.is_some());
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(600., -30.));
        assert_eq!(game.player.attack_timer, 0);
    }
    #[test]
    fn intro_narrative_reveals_then_walks_and_climbs_into_title_without_teleport() {
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.flags.insert("first_time".into());
        begin_intro(&mut game);
        let lines = game.opening.as_ref().unwrap().lines.clone();
        assert_eq!(game.opening.as_ref().unwrap().previous_fade_level, 1.);
        assert_eq!(game.opening.as_ref().unwrap().previous_darkness, 1.);
        assert!(lines.iter().all(|line| !line.is_empty()));
        let mut bats = 0;
        let mut throws = 0;
        let mut dummy_seen = false;
        let mut walk_start = false;
        let mut climbed = false;
        for tick in 1..=1100 {
            if game.world.name == "rTitle" {
                break;
            }
            game.step(&InputFrame::default());
            if game.world.name == "rIntro" {
                let state = game.opening.as_ref().unwrap();
                assert_eq!(state.lines, lines);
                if tick == 260 {
                    assert_eq!(state.previous_fade_level, 1.);
                    assert_eq!(state.fade_level, 0.9);
                }
                if tick < 260 {
                    assert_eq!(
                        state.draw_status,
                        if tick < 20 {
                            0
                        } else if tick < 100 {
                            1
                        } else if tick < 180 {
                            2
                        } else {
                            3
                        }
                    );
                }
                if let Some(actor) = game.world.entities.iter().find(|e| e.object == "oPDummy3") {
                    dummy_seen = true;
                    walk_start |= actor.pos == Vec2::new(-30., 184.);
                    climbed |= actor.pos.y > 184.;
                }
            }
            bats += game
                .events
                .iter()
                .filter(|event| matches!(event,crate::GameEvent::Sound{name,..} if name=="sndBat"))
                .count();
            throws += game
                .events
                .iter()
                .filter(
                    |event| matches!(event,crate::GameEvent::Sound{name,..} if name=="sndThrow"),
                )
                .count();
        }
        assert_eq!(game.world.name, "rTitle");
        assert_eq!(game.opening.as_ref().unwrap().kind, OpeningKind::Title);
        assert_eq!(
            game.opening.as_ref().unwrap().previous_darkness,
            game.opening.as_ref().unwrap().darkness
        );
        assert_eq!(
            game.opening.as_ref().unwrap().previous_fade_level,
            game.opening.as_ref().unwrap().fade_level
        );
        assert!(dummy_seen && walk_start && climbed);
        assert_eq!(bats, 4);
        assert_eq!(throws, 1);
        assert!(game.flags.contains("first_time"));
    }
    #[test]
    fn intro_skip_first_reveals_scene_then_second_edge_fades_to_title() {
        let mut game = Game::shell(12, RulesPreset::Classic);
        begin_intro(&mut game);
        game.step(&InputFrame {
            start_pressed: true,
            ..Default::default()
        });
        assert!(game.opening.as_ref().unwrap().fade_in);
        for _ in 0..20 {
            if game.world.entities.iter().any(|e| e.object == "oPDummy3") {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert!(game.world.entities.iter().any(|e| e.object == "oPDummy3"));
        game.step(&InputFrame {
            start_pressed: true,
            ..Default::default()
        });
        assert!(game.opening.as_ref().unwrap().fade_out);
        for _ in 0..20 {
            if game.world.name == "rTitle" {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.name, "rTitle");
        assert_eq!(game.opening.as_ref().unwrap().kind, OpeningKind::Title);
    }
}
