//! Run progression translated from oPlayer1 Step, oGame Step, GoldDoor
//! collision, and TunnelMan Step. Filesystem loading belongs to the host.
use crate::model::{
    Area, Game, GameEvent, InputFrame, PlayerState, RunState, VIEW_HEIGHT, VIEW_WIDTH, Vec2,
};

fn count(game: &mut Game, key: &str) {
    *game.data.entry(key.into()).or_default() += 1.;
}

fn initialize_level(game: &mut Game) {
    let level = game.world.level as f64;
    // level_tick also distinguishes restarting an editor level at the same index.
    if game.data.get("progression_level").copied() == Some(level) && game.level_tick > 1 {
        return;
    }
    game.data.insert("progression_level".into(), level);
    game.flags.remove("ghost_exists");
    game.flags.remove("ghost_warning");
    game.data.insert("udjat_blink_counter".into(), 0.);
    if game.world.dark {
        game.flags.insert("had_dark_level".into());
    }
    if game.world.tiles.iter().any(|t| t.object == "oMoai") {
        game.flags.insert("made_moai".into());
    }
    if game.world.name == "Black Market" {
        game.flags.insert("made_black_market".into());
    }
    if (game.flags.contains("murderer") || game.data.get("thiefLevel").copied().unwrap_or(0.) > 0.)
        && !game
            .world
            .entities
            .iter()
            .any(|e| e.object == "oShopkeeper" && e.pos.distance(game.world.exit) < 8.)
    {
        let pos = Vec2::new(game.world.exit.x - 8., game.world.exit.y - 8.);
        let id = game.world.spawn("oShopkeeper", pos);
        let keeper = game.world.entity_mut(id).unwrap();
        keeper.state = 4;
        keeper.data.insert("angry".into(), 1.);
        keeper.data.insert("angered".into(), 1.);
    }
}

/// oFinalBoss Step opens the authored right-hand door after lava kills Olmec.
fn final_boss(game: &mut Game) {
    if game.world.area != Area::Boss
        || !game.flags.contains("olmec_dead")
        || game.flags.contains("boss_door_open")
    {
        return;
    }
    let Some(index) =
        game.world.entities.iter().position(|door| {
            door.alive && door.object == "oEntrance" && door.value("boss_exit") > 0.
        })
    else {
        return;
    };
    let position = game.world.entities[index].pos;
    game.world.entities[index].alive = false;
    game.world.spawn("oXEnd", position);
    let (x, y) = ((position.x / 16.) as usize, (position.y / 16.) as usize + 1);
    if !game
        .world
        .tile(x as i32, y as i32)
        .is_some_and(|tile| tile.is_solid())
    {
        game.world.set_tile(x, y, "oTemple");
    }
    let center = Vec2::new(x as f64 * 16. + 8., y as f64 * 16. + 8.);
    let metadata = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == center)
        .map(|e| e.id)
        .unwrap_or_else(|| game.world.spawn("oTerrainMetadata", center));
    game.world
        .entity_mut(metadata)
        .unwrap()
        .data
        .insert("invincible".into(), 1.);
    game.flags.insert("boss_door_open".into());
    game.sound("sndThump", position);
}

fn revive(game: &mut Game) {
    if game.player.health > 0 || !game.player.equipment.remove("Ankh") {
        return;
    }
    let moai = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oMoaiInside")
        .map(|e| Vec2::new(e.pos.x + 8., e.pos.y + 8.));
    let destination = moai.unwrap_or_else(|| {
        if game.world.area == Area::Boss {
            Vec2::new(24., 552.)
        } else {
            game.world.entrance
        }
    });
    if moai.is_some() {
        for e in &mut game.world.entities {
            if e.object == "oMoaiInside" {
                e.alive = false;
                e.solid = false;
            }
        }
        game.flags.insert("moai_open".into());
    }
    game.player.pos = destination;
    game.player.previous_pos = destination;
    game.player.vel = Vec2::default();
    game.player.health = 4;
    game.player.invincible = 60;
    game.player.stun = 0;
    game.player.state = PlayerState::Falling;
    game.camera = Vec2::new(
        (destination.x - VIEW_WIDTH / 2.).max(0.),
        (destination.y - VIEW_HEIGHT / 2.).max(0.),
    );
    game.previous_camera = game.camera;
    game.flags.insert("render_discontinuity".into());
    game.sound("sndTeleport", destination);
    game.events.push(GameEvent::Message(
        "THE ANKH SHATTERS! YOU HAVE BEEN REVIVED!".into(),
    ));
}

fn secrets(game: &mut Game) {
    // The Gold Door consumes a held sceptre only when the Hedjet is owned.
    if let Some(id) = game.player.held {
        let sceptre = game
            .world
            .entity(id)
            .filter(|e| e.object == "oSceptre")
            .map(|e| e.pos);
        if let Some(pos) = sceptre {
            if let Some(door) = game
                .world
                .entities
                .iter()
                .position(|e| e.alive && e.object == "oGoldDoor" && e.pos.distance(pos) < 16.)
            {
                if game.player.has("Crown") {
                    let door_pos = game.world.entities[door].pos;
                    game.world.entities[door].alive = false;
                    if let Some(item) = game.world.entity_mut(id) {
                        item.alive = false;
                        item.held = false;
                    }
                    game.player.held = None;
                    game.world.spawn("oXGold", door_pos);
                    game.sound("sndChestOpen", door_pos);
                } else if !game.flags.contains("gold_door_hint") {
                    game.flags.insert("gold_door_hint".into());
                    game.events.push(GameEvent::Message(
                        "THE SCEPTRE FITS... BUT NOTHING IS HAPPENING!".into(),
                    ));
                }
            }
        }
    }
    // Source eye alarm depends on distance, with a four-tick minimum.
    if game.player.has("UdjatEye") {
        let distance = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oXMarket")
            .map(|e| e.pos.distance(game.player.pos))
            .min_by(f64::total_cmp);
        if let Some(distance) = distance {
            let counter = game.data.entry("udjat_blink_counter".into()).or_default();
            *counter -= 1.;
            if *counter <= 0. || *counter > distance.max(4.) {
                *counter = distance.max(4.);
                game.flags.insert("udjat_blink".into());
                game.sound("sndBlink", game.player.pos);
            } else {
                game.flags.remove("udjat_blink");
            }
        } else {
            game.flags.remove("udjat_blink");
        }
    }
}

fn ghost(game: &mut Game) {
    // The original increments xtime by room_speed (30), not delta milliseconds.
    // Keep that source counter exactly; do not reinterpret its thresholds.
    let elapsed = game.level_tick.saturating_mul(30);
    game.data.insert("xtime".into(), elapsed as f64);
    if game.world.level <= 1
        || game.world.area == Area::Boss
        || game.flags.contains("custom_level")
        || game.player.has("Crown")
        || game.player.health <= 0
    {
        return;
    }
    if elapsed > 120000 && game.flags.insert("ghost_warning".into()) {
        game.events.push(GameEvent::Message(
            "A CHILL RUNS UP YOUR SPINE... LET'S GET OUT OF HERE!".into(),
        ));
    }
    if elapsed > 150000 && !game.flags.contains("ghost_exists") {
        let x = if game.player.pos.x > game.world.width as f64 * 8. {
            game.camera.x + VIEW_WIDTH + 8.
        } else {
            game.camera.x - 32.
        };
        game.world
            .spawn("oGhost", Vec2::new(x, game.camera.y + VIEW_HEIGHT / 2.));
        game.flags.insert("ghost_exists".into());
    }
}

/// A source-cost shortcut donation offered after an area boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunnelOffer {
    pub stage: u8,
    pub remaining: u32,
    pub destination: u32,
}

fn tunnel_stage(game: &Game) -> Option<u8> {
    if !game.flags.contains("shortcut_5") {
        Some(1)
    } else if !game.flags.contains("shortcut_9") {
        Some(2)
    } else if !game.flags.contains("shortcut_13") {
        Some(3)
    } else {
        None
    }
}

fn tunnel_remaining(game: &Game, stage: u8) -> u32 {
    let cost = u32::from(stage) * 100000;
    game.data
        .get(&format!("tunnel{stage}_remaining"))
        .copied()
        .unwrap_or(f64::from(cost))
        .clamp(0., f64::from(cost)) as u32
}

/// Host transition UI can inspect an offer without replacing the ended world.
/// Classic's introductory max+1 sentinel delay is intentionally omitted.
pub fn tunnel_offer(game: &Game) -> Option<TunnelOffer> {
    if game.state != RunState::Transition
        || game.flags.contains("custom_level")
        || game.flags.contains("tutorial")
        || game.flags.contains("hub")
    {
        return None;
    }
    let stage = tunnel_stage(game)?;
    if game.world.level != u32::from(stage) * 4 {
        return None;
    }
    Some(TunnelOffer {
        stage,
        remaining: tunnel_remaining(game, stage),
        destination: u32::from(stage) * 4 + 1,
    })
}

fn apply_donation(game: &mut Game, stage: u8, requested: u32) -> u32 {
    let remaining = tunnel_remaining(game, stage);
    let amount = requested.min(remaining).min(game.player.money);
    if amount == 0 {
        return 0;
    }
    game.player.money -= amount;
    game.data.insert(
        format!("tunnel{stage}_remaining"),
        f64::from(remaining - amount),
    );
    if amount == remaining {
        game.flags
            .insert(format!("shortcut_{}", u32::from(stage) * 4 + 1));
        game.events.push(GameEvent::Message(
            "THANKS! THE SHORTCUT IS COMPLETE!".into(),
        ));
    } else {
        game.events
            .push(GameEvent::Message("THANKS FOR THE DONATION!".into()));
    }
    amount
}

/// Pay only an eligible transition's offer, clamped to money and remaining cost.
/// Returns the amount actually paid; simulation, world and carried items stay intact.
pub fn donate_to_tunnel(game: &mut Game, amount: u32) -> u32 {
    let Some(offer) = tunnel_offer(game) else {
        return 0;
    };
    apply_donation(game, offer.stage, amount)
}

fn donate(game: &mut Game, input: &InputFrame) {
    let tunnel = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oTunnelMan" && e.pos.distance(game.player.pos) < 32.)
        .map(|e| e.id);
    let Some(id) = tunnel else {
        return;
    };
    let Some(stage) = tunnel_stage(game) else {
        return;
    };
    let remaining = f64::from(tunnel_remaining(game, stage));
    let entity = game.world.entity_mut(id).unwrap();
    let mut amount = entity.value("donate");
    // Preserve the 20-tick initial delay and 100-tick accelerated hold.
    let held = if input.up || input.down {
        entity.value("donation_hold") + 1.
    } else {
        0.
    };
    entity.data.insert("donation_hold".into(), held);
    if held > 20. {
        let delta = if held > 100. { 1000. } else { 100. };
        amount += if input.up { delta } else { -delta };
    }
    amount = amount.max(0.).min(remaining).min(game.player.money as f64);
    entity.data.insert("donate".into(), amount);
    if input.pay_pressed && amount > 0. {
        apply_donation(game, stage, amount as u32);
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("donate".into(), 0.);
    }
}

fn complete(game: &mut Game, door_id: u64) {
    let Some(door) = game.world.entity(door_id) else {
        return;
    };
    let object = door.object.clone();
    let leads_to = door.text.get("leadsTo").cloned().unwrap_or_default();
    if object == "oXMarket" {
        game.flags.insert("genBlackMarket".into());
    }
    if object == "oXGold" {
        game.flags.insert("cityOfGold".into());
    }
    if !leads_to.is_empty() {
        game.flags.retain(|f| !f.starts_with("custom_level:"));
        game.flags.insert(format!("custom_level:{leads_to}"));
    }
    if let Some(id) = game.player.held {
        let carried = game
            .world
            .entity(id)
            .map(|e| (e.object.clone(), e.health, e.value("value")));
        if let Some((object, health, value)) = carried {
            if object == "oDamsel" && health > 0 {
                count(game, "damsels");
                game.flags.insert("damsel_rescued".into());
                if let Some(e) = game.world.entity_mut(id) {
                    e.alive = false;
                }
            } else if object == "oGoldIdol" {
                let multiplier = match game.world.area {
                    Area::Mines => 1,
                    Area::Jungle => 2,
                    Area::Ice => 3,
                    _ => 4,
                };
                game.player.money = game
                    .player
                    .money
                    .saturating_add((value.max(5000.) as u32).saturating_mul(multiplier));
                count(game, "idolsConverted");
                if let Some(e) = game.world.entity_mut(id) {
                    e.alive = false;
                }
            } else if !matches!(
                object.as_str(),
                "oRock" | "oPot" | "oSkull" | "oCrate" | "oChest"
            ) {
                // Keep a carried weapon for root's level transition adapter.
                game.flags.retain(|f| !f.starts_with("carry:"));
                game.flags.insert(format!("carry:{object}"));
            }
        }
        game.player.held = None;
    }
    let collect = game.data.remove("collect").unwrap_or(0.).max(0.) as u32;
    game.player.money = game.player.money.saturating_add(collect);
    if let Some(thief) = game.data.get_mut("thiefLevel") {
        *thief = (*thief - 1.).max(0.);
    }
    if game.data.get("thiefLevel").copied().unwrap_or(0.) == 0. && !game.flags.contains("murderer")
    {
        game.flags.remove("wanted");
    }
    game.data.insert(
        "next_level".into(),
        game.world.level.saturating_add(1) as f64,
    );
    game.events.push(GameEvent::LevelComplete);
    game.sound("sndSteps", game.player.pos);
    if object == "oXEnd" || (game.world.area == Area::Boss && object == "oExit") {
        game.state = RunState::Won;
        game.flags.insert("won".into());
        game.events.push(GameEvent::Won);
    } else {
        game.state = RunState::Transition;
    }
}

pub fn tick(game: &mut Game, input: &InputFrame) {
    if game.state != RunState::Playing || game.flags.contains("hub") {
        return;
    }
    initialize_level(game);
    final_boss(game);
    revive(game);
    if game.player.health <= 0 {
        return;
    }
    *game.data.entry("time".into()).or_default() += 30.;
    secrets(game);
    ghost(game);
    donate(game, input);
    if !(input.up || input.exit_pressed)
        || !game.player.grounded
        || game.player.stun > 0
        || game.player.attack_timer > 0
    {
        return;
    }
    let door = game
        .world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && matches!(e.object.as_str(), "oExit" | "oXMarket" | "oXGold" | "oXEnd")
                && e.bounds().contains(game.player.pos)
                && !game.world.solid_at(e.pos.x, e.pos.y)
        })
        .map(|e| e.id);
    if let Some(id) = door {
        complete(game, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;
    fn game(level: u32) -> Game {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world.level = level;
        g.player.grounded = true;
        g.level_tick = 2;
        g
    }
    #[test]
    fn ordinary_exit_requires_ground_and_input() {
        let mut g = game(2);
        g.world.spawn("oExit", g.player.pos);
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        tick(
            &mut g,
            &InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(g.state, RunState::Transition);
        assert_eq!(g.data["next_level"], 3.);
    }
    #[test]
    fn ankh_revives_once_inside_moai_and_resets_interpolation() {
        let mut g = game(10);
        g.player.health = 0;
        g.player.equipment.insert("Ankh".into());
        let p = Vec2::new(100., 100.);
        let id = g.world.spawn("oMoaiInside", p);
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.pos, Vec2::new(p.x + 8., p.y + 8.));
        assert_eq!(g.player.previous_pos, g.player.pos);
        assert!(!g.player.has("Ankh"));
        assert!(g.world.entity(id).is_none());
        g.player.health = 0;
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.player.health, 0);
    }
    #[test]
    fn ghost_uses_original_counter_and_spawns_once() {
        let mut g = game(2);
        g.level_tick = 4000;
        tick(&mut g, &InputFrame::default());
        assert!(!g.flags.contains("ghost_warning"));
        g.level_tick = 4001;
        tick(&mut g, &InputFrame::default());
        assert!(g.flags.contains("ghost_warning"));
        g.level_tick = 5001;
        tick(&mut g, &InputFrame::default());
        tick(&mut g, &InputFrame::default());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oGhost")
                .count(),
            1
        );
    }
    #[test]
    fn hedjet_suppresses_ghost() {
        let mut g = game(14);
        g.level_tick = 10000;
        g.player.equipment.insert("Crown".into());
        tick(&mut g, &InputFrame::default());
        assert!(!g.flags.contains("ghost_exists"));
    }
    #[test]
    fn golden_door_consumes_sceptre_only_with_crown() {
        let mut g = game(14);
        let id = g.world.spawn("oSceptre", g.player.pos);
        g.player.held = Some(id);
        g.world.spawn("oGoldDoor", g.player.pos);
        tick(&mut g, &InputFrame::default());
        assert!(g.world.entity(id).is_some());
        g.player.equipment.insert("Crown".into());
        tick(&mut g, &InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert!(g.world.entities.iter().any(|e| e.object == "oXGold"));
    }
    #[test]
    fn donation_unlocks_first_shortcut() {
        let mut g = game(4);
        g.player.money = 100000;
        let id = g.world.spawn("oTunnelMan", g.player.pos);
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("donate".into(), 100000.);
        tick(
            &mut g,
            &InputFrame {
                pay_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.money, 0);
        assert!(g.flags.contains("shortcut_5"));
    }
    #[test]
    fn boss_exit_emits_win_and_custom_exit_keeps_link() {
        let mut g = game(16);
        g.world.area = Area::Boss;
        g.world.spawn("oExit", g.player.pos);
        tick(
            &mut g,
            &InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(g.state, RunState::Won);
        let mut g = game(1);
        let id = g.world.spawn("oExit", g.player.pos);
        g.world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("leadsTo".into(), "next.lvl".into());
        tick(
            &mut g,
            &InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert!(g.flags.contains("custom_level:next.lvl"));
    }
    #[test]
    fn generated_olmec_lava_death_opens_protected_exit_and_wins() {
        let mut g = Game::shell(17, RulesPreset::Classic);
        g.world = crate::generation::generate(&mut g.rng, 16, &g.flags);
        g.player.pos = g.world.exit;
        g.player.previous_pos = g.player.pos;
        g.update_camera();
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.object == "oEntrance" && e.value("boss_exit") > 0.)
        );
        assert!(!g.world.entities.iter().any(|e| e.object == "oXEnd"));
        // Standing at the closed, generated door cannot finish the campaign.
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(g.state, RunState::Playing);
        let lava = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oLava" && e.pos.x == 304. && e.pos.y == 768.)
            .unwrap()
            .pos;
        assert_eq!(g.world.tile(19, 48).unwrap().object, "");
        // Reproduce the excavated shaft a player makes through the generated
        // underground rooms; keep the generated lava entities untouched.
        for y in 44..48 {
            for x in 19..23 {
                g.world.set_tile(x, y, "");
            }
        }
        let boss_id = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oOlmec")
            .unwrap()
            .id;
        let boss = g.world.entity_mut(boss_id).unwrap();
        // Start at the actual generated lava surface: feet enter first, then
        // authoritative ticks must sink the boss until its head enters lava.
        boss.pos = Vec2::new(lava.x, lava.y - 64.);
        boss.previous_pos = boss.pos;
        boss.state = 6;
        boss.vel = Vec2::default();
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(boss_id).unwrap().state, 4);
        assert!(!g.flags.contains("olmec_dead"));
        for _ in 0..700 {
            g.step(&InputFrame::default());
            if g.flags.contains("olmec_dead") {
                break;
            }
        }
        assert!(
            g.flags.contains("olmec_dead"),
            "Olmec must finish drowning while the player waits at the exit: state={:?}, boss={:?}",
            g.state,
            g.world.entity(boss_id).map(|e| (e.pos, e.vel, e.state))
        );
        assert!(g.world.entity(boss_id).is_none());
        let exit = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oXEnd")
            .unwrap()
            .pos;
        assert_eq!(exit, Vec2::new(640., 544.));
        assert!(g.world.tile(40, 35).unwrap().is_solid());
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.object == "oTerrainMetadata"
                    && e.pos == Vec2::new(648., 568.)
                    && e.value("invincible") > 0.)
        );
        assert_eq!(g.state, RunState::Playing);
        g.player.pos = Vec2::new(exit.x + 8., exit.y + 8.);
        g.player.previous_pos = g.player.pos;
        g.player.vel = Vec2::default();
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert!(g.player.grounded);
        assert_eq!(g.state, RunState::Won);
        assert!(g.events.contains(&GameEvent::Won));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oXEnd")
                .count(),
            1
        );
    }
    #[test]
    fn tunnel_offers_follow_source_stages_and_area_boundaries() {
        let mut g = game(4);
        g.state = RunState::Transition;
        assert_eq!(
            tunnel_offer(&g),
            Some(TunnelOffer {
                stage: 1,
                remaining: 100000,
                destination: 5
            })
        );
        g.world.level = 8;
        assert_eq!(tunnel_offer(&g), None);
        g.flags.insert("shortcut_5".into());
        assert_eq!(
            tunnel_offer(&g),
            Some(TunnelOffer {
                stage: 2,
                remaining: 200000,
                destination: 9
            })
        );
        g.world.level = 12;
        assert_eq!(tunnel_offer(&g), None);
        g.flags.insert("shortcut_9".into());
        assert_eq!(
            tunnel_offer(&g),
            Some(TunnelOffer {
                stage: 3,
                remaining: 300000,
                destination: 13
            })
        );
        g.flags.insert("shortcut_13".into());
        assert_eq!(tunnel_offer(&g), None);
    }
    #[test]
    fn partial_transition_donation_preserves_world_and_carry_and_survives_runs() {
        let mut g = game(4);
        g.state = RunState::Transition;
        g.player.money = 30000;
        let id = g.world.spawn("oShotgun", g.player.pos);
        g.world.entity_mut(id).unwrap().held = true;
        g.player.held = Some(id);
        g.flags.insert("carry:oShotgun".into());
        let world = serde_json::to_string(&g.world).unwrap();
        let tick = g.tick;
        assert_eq!(donate_to_tunnel(&mut g, 50000), 30000);
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["tunnel1_remaining"], 70000.);
        assert_eq!(g.state, RunState::Transition);
        assert_eq!(g.tick, tick);
        assert_eq!(g.player.held, Some(id));
        assert!(g.flags.contains("carry:oShotgun"));
        assert_eq!(serde_json::to_string(&g.world).unwrap(), world);
        let mut next = game(4);
        next.state = RunState::Transition;
        next.flags = g.flags.clone();
        next.data = g.data.clone();
        next.player.money = 80000;
        assert_eq!(tunnel_offer(&next).unwrap().remaining, 70000);
        assert_eq!(donate_to_tunnel(&mut next, u32::MAX), 70000);
        assert_eq!(next.player.money, 10000);
        assert!(next.flags.contains("shortcut_5"));
        assert_eq!(donate_to_tunnel(&mut next, 1000), 0);
    }
    #[test]
    fn donation_rejects_playing_custom_tutorial_and_wrong_boundary() {
        let mut g = game(4);
        g.player.money = 100000;
        assert_eq!(donate_to_tunnel(&mut g, 100), 0);
        g.state = RunState::Transition;
        g.flags.insert("custom_level".into());
        assert_eq!(donate_to_tunnel(&mut g, 100), 0);
        g.flags.clear();
        g.flags.insert("tutorial".into());
        assert_eq!(donate_to_tunnel(&mut g, 100), 0);
        g.flags.clear();
        g.world.level = 3;
        assert_eq!(donate_to_tunnel(&mut g, 100), 0);
        assert_eq!(g.player.money, 100000);
    }
}
