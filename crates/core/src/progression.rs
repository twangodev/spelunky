//! Run progression translated from oPlayer1 Step, oGame Step, GoldDoor
//! collision, and TunnelMan Step. Filesystem loading belongs to the host.
use crate::model::{
    Area, Game, GameEvent, InputFrame, PlayerState, Rect, RunState, VIEW_HEIGHT, VIEW_WIDTH, Vec2,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameOverDestination {
    Editor,
    CustomLevels,
    Scores,
}

/// oGame's authoritative reveal and score counter. Rendering never advances it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameOverState {
    pub draw_status: u8,
    pub money_count: u32,
    pub acknowledged: bool,
    pub destination: Option<GameOverDestination>,
}
impl Default for GameOverState {
    fn default() -> Self {
        Self {
            draw_status: 0,
            money_count: 0,
            acknowledged: false,
            destination: None,
        }
    }
}

/// Alarm callback after the canonical dispatcher has decremented Game.alarms.
pub(crate) fn game_alarm(game: &mut Game, index: usize) {
    match index {
        0 => {
            if game.game_over.draw_status < 3 {
                game.game_over.draw_status = 2;
            }
            game.alarms[1] = 50;
        }
        1 if game.game_over.draw_status < 3 => game.game_over.draw_status = 3,
        2 => {
            let on = !game.flags.remove("udjat_blink");
            if on {
                game.flags.insert("udjat_blink".into());
            }
            if game.player.has("UdjatEye") {
                game.sound(if on { "sndBlink1" } else { "sndBlink2" }, game.player.pos);
            }
        }
        _ => {}
    }
}

/// oGame Step (resource318) observes the independent dead flag before Player319.
pub(crate) fn game_step(game: &mut Game, _input: &InputFrame) {
    let market = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oXMarket"))
        .map(|e| {
            // oGame executes this query inside with oPlayer1. The receiver
            // is Player's cached envelope, not Game's origin or mask.
            crate::world_events::mask_distance(
                game.player.collision_mask().point_bounds(),
                e.collision_mask().point_bounds(),
            )
            .min(1_000_000.0)
        })
        .min_by(f64::total_cmp);
    if let Some(distance) = market.filter(|_| game.player.engine_active) {
        // The canonical alarm setter converts its real value using x87's
        // default nearest-even integer mode, rather than truncating it.
        let distance = distance.max(4.).round_ties_even() as i32;
        if game.alarms[2] < 1 || distance < game.alarms[2] {
            game.alarms[2] = distance;
        }
    } else if market.is_none() {
        game.flags.remove("udjat_blink");
    }
    if game.state == RunState::Dead {
        let reveal = &mut game.game_over;
        if reveal.draw_status == 0 {
            reveal.draw_status = 1;
            game.alarms[0] = 50;
        }
        if reveal.draw_status > 2 {
            let difference = game.player.money.saturating_sub(reveal.money_count);
            reveal.money_count = reveal.money_count.saturating_add(if difference > 1000 {
                1000
            } else if difference > 100 {
                100
            } else {
                difference
            });
        }
    }
    if game.is_level() {
        ghost(game);
    }
}

/// Player Step's Game Over action. A reveal skip and acknowledgment are separate
/// input edges; lifetime level deaths are counted only by the latter.
pub fn acknowledge_game_over(game: &mut Game, input: &InputFrame) -> bool {
    if game.state != RunState::Dead
        || game.game_over.acknowledged
        || !(input.attack_pressed || input.start_pressed)
    {
        return false;
    }
    if game.game_over.draw_status < 3 || game.game_over.money_count < game.player.money {
        game.game_over.draw_status = 3;
        game.game_over.money_count = game.player.money;
        return false;
    }
    if crate::statistics::is_real_level(game) {
        let level = game
            .data
            .get("next_level")
            .copied()
            .unwrap_or(game.world.level as f64);
        if level >= 1. && level <= 16. {
            game.lifetime_stats.level_death(level as usize - 1);
        }
    }
    let destination = if game.flags.contains("editor_test") {
        GameOverDestination::Editor
    } else if game.flags.contains("custom") || game.flags.contains("custom_level") {
        GameOverDestination::CustomLevels
    } else {
        for (flag, start) in [("hub_sun", 1), ("hub_moon", 2), ("hub_stars", 3)] {
            if game.flags.contains(flag) {
                game.flags.insert(format!("scores_start:{start}"));
                break;
            }
        }
        game.flags.insert("reset_run_on_hub".into());
        GameOverDestination::Scores
    };
    game.flags
        .retain(|flag| !flag.starts_with("previous_custom_level:"));
    game.game_over.acknowledged = true;
    game.game_over.destination = Some(destination);
    game.state = RunState::Transition;
    true
}

fn count(game: &mut Game, key: &str) {
    *game.data.entry(key.into()).or_default() += 1.;
}

/// Source room Create work must run before inactive instances are partitioned.
/// The host clears this marker in reset_position for every new authored world.
pub(crate) fn initialize_level(game: &mut Game) {
    if game.flags.contains("hub") || game.data.contains_key("progression_initialized") {
        return;
    }
    game.data.insert("progression_initialized".into(), 1.);
    game.data
        .insert("progression_level".into(), game.world.level as f64);
    game.flags.remove("ghost_exists");
    game.flags.remove("ghost_warning");
    crate::music::room_create(game);
    game.flags.remove("udjat_blink");
    game.data.insert("udjat_blink_counter".into(), 0.);
    if game.world.dark {
        game.flags.insert("had_dark_level".into());
    }
    if game.world.tiles.iter().any(|t| t.object == "oMoai")
        || game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oMoai")
    {
        game.flags.insert("made_moai".into());
    }
    if game.world.name == "Black Market" {
        game.flags.insert("made_black_market".into());
    }
    if game.flags.contains("murderer") || game.data.get("thiefLevel").copied().unwrap_or(0.) > 0. {
        // scrInitLevel's with(oExit) visits every freshly created ordinary
        // exit, even when a keeper already occupies it. oXGold inherits the
        // parent's "Exit" type; oXMarket's Create changes it to "Market Exit".
        let mut exits: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| {
                e.alive
                    && crate::catalog::is_a(&e.object, "oExit")
                    && e.text
                        .get("type")
                        .map_or(e.object != "oXMarket", |kind| kind == "Exit")
            })
            .map(|e| (e.creation_serial, e.pos))
            .collect();
        // Initialization precedes activation/rebuilt instance lists. Preserve
        // that room's initial birth order independently from entity storage.
        exits.sort_by_key(|(serial, _)| *serial);
        for (_, pos) in exits {
            let id = game.spawn("oShopkeeper", pos);
            // Source producer writes only PATROL after synchronous Create.
            game.world.entity_mut(id).unwrap().state = 4;
        }
    }
}

/// oFinalBoss Step opens the authored right-hand door after lava kills Olmec.
pub(crate) fn final_boss_step(game: &mut Game) {
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

fn moai_revival_anchor(game: &Game) -> Option<Vec2> {
    // Implicit oMoai.x/y selects the first current active family member.
    // Raw World births remain visible to direct public adapters, appended
    // after registered entries by creation serial. No query runs Create or
    // mutates family registration; storage only breaks synthetic serial ties.
    let family = crate::dispatcher::descendant_instances(game, "oMoai");
    game.world
        .entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| {
            entity.alive && entity.engine_active && crate::catalog::is_a(&entity.object, "oMoai")
        })
        .min_by_key(|(storage, entity)| {
            family
                .iter()
                .position(|instance| *instance == crate::dispatcher::Instance::Entity(entity.id))
                .map_or((1_u8, entity.creation_serial, *storage), |rank| {
                    (0, rank as u64, *storage)
                })
        })
        .map(|(_, entity)| Vec2::new(entity.pos.x + 24., entity.pos.y + 24.))
}

fn revive(game: &mut Game) {
    if game.state == RunState::Dead
        || game.player.health > 0
        || !game.player.equipment.remove("Ankh")
    {
        return;
    }
    let moai = moai_revival_anchor(game);
    let destination = moai.unwrap_or_else(|| {
        if game.world.area == Area::Boss {
            Vec2::new(24., 552.)
        } else {
            // Activation can rebuild source descendant arrays after depth
            // ordering changes. Query the current array, not a room-Create
            // snapshot or the custom level's selected return entrance.
            crate::dispatcher::activate_object(game, "oEntrance");
            crate::dispatcher::descendant_instances(game, "oEntrance")
                .into_iter()
                .find(|instance| match instance {
                    crate::dispatcher::Instance::Entity(id) => game.world.entity(*id).is_some(),
                    crate::dispatcher::Instance::Terrain(index) => game
                        .world
                        .tiles
                        .get(*index)
                        .is_some_and(|tile| !tile.object.is_empty()),
                    crate::dispatcher::Instance::Player => false,
                })
                .map(|instance| {
                    let pos = instance.position(game);
                    Vec2::new(pos.x + 8., pos.y + 8.)
                })
                .unwrap_or(game.world.entrance)
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
    game.player.data.insert("blink".into(), 60.);
    game.player.data.insert("active".into(), 1.);
    game.player.data.remove("hidden");
    // Revival resets death, not the separate stun flag or its countdown.
    // A fatal explosion therefore retains its source 100-tick stun.
    if game.player.state == PlayerState::Dead {
        // The Rust health controller can select its corpse enum earlier in the
        // fatal frame, before Classic's separate dead flag is latched. An Ankh
        // cancels that pending death while retaining any independent stun.
        game.player.state = if game.player.stun > 0 {
            PlayerState::Stunned
        } else {
            PlayerState::Falling
        };
    }
    // oPlayer1's Ankh branch resets the accumulated fall before resuming.
    game.player.data.insert("fall_timer".into(), 0.);
    game.player.data.insert("fall_ticks".into(), 0.);
    for entity in &mut game.world.entities {
        if entity.alive && matches!(entity.object.as_str(), "oBall" | "oChain") {
            entity.pos = destination;
            entity.previous_pos = destination;
        }
    }
    game.camera = Vec2::new(
        (destination.x - VIEW_WIDTH / 2.).max(0.),
        (destination.y - VIEW_HEIGHT / 2.).max(0.),
    );
    game.previous_camera = game.camera;
    game.flags.insert("render_discontinuity".into());
    game.sound("sndTeleport", destination);
    crate::enemies::shop_dialogue::say(game, "THE ANKH SHATTERS!", "YOU HAVE BEEN REVIVED!", 150);
}

pub(crate) fn gold_door_collision(game: &mut Game) {
    // The Gold Door consumes a held sceptre only when the Hedjet is owned.
    if let Some(id) = game.player.held {
        let sceptre = game
            .world
            .entity(id)
            .filter(|e| e.object == "oSceptre")
            .map(|e| e.sprite_bounds());
        if let Some(bounds) = sceptre {
            if let Some(door) = game.world.entities.iter().position(|e| {
                e.alive && e.object == "oGoldDoor" && e.sprite_bounds().intersects(bounds)
            }) {
                if game.player.has("Crown") {
                    let door_pos = game.world.entities[door].pos;
                    game.world.entities[door].alive = false;
                    if let Some(item) = game.world.entity_mut(id) {
                        item.alive = false;
                        item.held = false;
                    }
                    game.player.held = None;
                    game.player.pickup_item_type.clear();
                    game.world.spawn("oXGold", door_pos);
                    game.sound("sndChestOpen", door_pos);
                } else {
                    crate::enemies::shop_dialogue::say(
                        game,
                        "THE SCEPTRE FITS...",
                        "BUT NOTHING IS HAPPENING!",
                        100,
                    );
                }
            }
        }
    }
}

/// oLevel553 Step16 precedes camera and activation work.
pub(crate) fn level_music_step(game: &mut Game) {
    let timer = game.data.get("music_fade_timer").copied().unwrap_or(0.0);
    if !game.flags.contains("music_fade") || timer >= 100.0 {
        return;
    }
    if game.is_level() {
        if let Some(music) = crate::music::level_track(game) {
            // Keep signed source requests, even beyond this port's positive-Hz backend domain.
            let hz = game.source_music.frequency(music) - 100;
            game.data.insert("music_frequency".into(), hz as f64);
            game.set_source_music_frequency(music, hz);
        }
    }
    game.data.insert("music_fade_timer".into(), timer + 1.0);
}

fn ghost(game: &mut Game) {
    // The level clock advances only while its player is alive. Dead world
    // ticks must not be reinterpreted as elapsed run time.
    let elapsed = game.data.get("xtime").copied().unwrap_or(0.);
    if game.world.level <= 1
        || game.world.area == Area::Boss
        || game.flags.contains("custom_level")
        || game.flags.contains("custom")
        || game.player.has("Crown")
        || matches!(game.player.sprite.as_str(), "sPExit" | "sDamselExit")
    {
        return;
    }
    if elapsed > 120000. && game.flags.insert("ghost_warning".into()) {
        game.flags.insert("music_fade".into());
        crate::enemies::shop_dialogue::say(
            game,
            "A CHILL RUNS UP YOUR SPINE...",
            "LET'S GET OUT OF HERE!",
            200,
        );
    }
    if elapsed > 150000. && !game.flags.contains("ghost_exists") {
        let view = game.simulation_view_rect();
        let x = if game.player.pos.x > game.world.width as f64 * 8. {
            view.x + view.w + 8.
        } else {
            view.x - 32.
        };
        game.world
            .spawn("oGhost", Vec2::new(x, view.y + (view.h / 2.).floor()));
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
        .clamp(0., f64::from(cost + 1)) as u32
}

/// Inspect the source transition's offer without advancing the ended level.
pub fn tunnel_offer(game: &Game) -> Option<TunnelOffer> {
    let transition_cinematic = game.state == RunState::Ending
        && game.ending.as_ref().is_some_and(|ending| {
            ending.kind == crate::ending::EndingKind::Transition
                && ending.stage == crate::ending::EndingStage::Transition
        });
    if (game.state != RunState::Transition && !transition_cinematic)
        || game.flags.contains("custom_level")
        || game.flags.contains("custom")
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

pub(crate) fn apply_donation(game: &mut Game, stage: u8, requested: u32) -> u32 {
    let remaining = tunnel_remaining(game, stage);
    // max+1 is the source's first-encounter marker, consumed by room Create
    // without spawning Tunnel Man. It is profile state, not a payable dollar.
    if remaining > u32::from(stage) * 100000 {
        return 0;
    }
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
    if ["hub", "custom", "custom_level", "tutorial"]
        .iter()
        .any(|flag| game.flags.contains(*flag))
    {
        return;
    }
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
    if game.world.level != u32::from(stage) * 4 {
        return;
    }
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

/// A damsel enters the door independently of the player. Each rescue produces
/// one transition kiss, including several rescues in the same room.
fn rescue_held_damsel(game: &mut Game, id: u64, door_center: Vec2) {
    let Some(damsel) = game.world.entity_mut(id) else {
        return;
    };
    if damsel.health <= 0 || damsel.state == 4 {
        return;
    }
    damsel.state = 4;
    damsel.pos = door_center;
    damsel.previous_pos = door_center;
    damsel.vel = Vec2::default();
    damsel.held = false;
    damsel.sprite = if game.flags.contains("character_damsel") {
        "sPExit".into()
    } else {
        "sDamselExit2".into()
    };
    damsel.data.insert("inactive".into(), 1.);
    damsel.data.insert("canPickUp".into(), 0.);
    game.player.held = None;
    count(game, "damsels");
    count(game, "xdamsels");
    count(game, "pending_damsel_kisses");
    if crate::statistics::is_real_level(game) {
        game.lifetime_stats
            .increment(crate::statistics::LifetimeCounter::DamselsSavedTotal);
    }
    game.flags.insert("damsel_rescued".into());
    game.sound("sndSteps", door_center);
}

fn redeem_held_idol(game: &mut Game, id: u64, immediate: bool) {
    let Some(idol) = game.world.entity(id) else {
        return;
    };
    let crystal = idol.object == "oCrystalSkull" || idol.sprite == "sCrystalSkull";
    let value = idol.value("value").max(0.) as u32;
    let multiplier = match game.world.area {
        Area::Mines => 1,
        Area::Jungle => 2,
        Area::Ice => 3,
        _ => 4,
    };
    let amount = value.saturating_mul(multiplier);
    if immediate {
        game.player.money = game.player.money.saturating_add(amount);
    } else {
        *game.data.entry("collect".into()).or_default() += f64::from(amount);
        let counter = game.data.entry("collectCounter".into()).or_default();
        *counter = (*counter + 20.).min(100.);
    }
    if crate::statistics::is_real_level(game) {
        count(game, "idolsConverted");
        game.lifetime_stats
            .increment(crate::statistics::LifetimeCounter::IdolsConverted);
    }
    count(game, if crystal { "skulls" } else { "idols" });
    // Both source exit branches birth the collection flash before Destroy,
    // at the Player origin even when the idol itself has another offset.
    game.spawn(
        "oBigCollect",
        Vec2::new(game.player.pos.x, game.player.pos.y - 8.),
    );
    if let Some(idol) = game.world.entity_mut(id) {
        idol.alive = false;
        idol.held = false;
    }
    game.player.held = None;
    game.sound("sndCoin", game.player.pos);
}

/// Source oPlayer1 Step also deposits idols and live damsels merely by touching
/// an exit. This does not require Up, standing on the floor, or leaving the room.
fn deposit_at_exit(game: &mut Game) {
    let Some(id) = game.player.held else {
        return;
    };
    let Some(center) = game
        .world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && crate::catalog::is_a(&e.object, "oExit")
                && e.sprite_bounds().contains(game.player.pos)
        })
        .map(|e| Vec2::new(e.pos.x + 8., e.pos.y + 8.))
    else {
        return;
    };
    let Some(object) = game.world.entity(id).map(|e| e.object.clone()) else {
        return;
    };
    if object == "oDamsel" {
        rescue_held_damsel(game, id, center);
    } else if matches!(object.as_str(), "oGoldIdol" | "oCrystalSkull") {
        redeem_held_idol(game, id, false);
    }
}

fn complete(game: &mut Game, door_id: u64) {
    let Some(door) = game.world.entity(door_id) else {
        return;
    };
    let object = door.object.clone();
    let ending = object == "oXEnd";
    let leads_to = door.text.get("leadsTo").cloned().unwrap_or_default();
    let door_center = Vec2::new(door.pos.x + 8., door.pos.y + 8.);
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
    let mut reserved_carry = false;
    let had_held = game.player.held.is_some();
    if let Some(id) = game.player.held {
        let carried = game.world.entity(id).map(|e| {
            (
                e.object.clone(),
                e.health,
                e.value("heavy") > 0.,
                e.value("armed") > 0.,
            )
        });
        if let Some((object, health, heavy, armed)) = carried {
            // oXEnd inherits oXStart, not oExit. The authored Olmec exit
            // follows the start-portal branch, which drops every heavy object
            // before the ordinary door's idol/damsel redemption rules.
            if ending && heavy {
                if let Some(item) = game.world.entity_mut(id) {
                    item.held = false;
                }
            } else if object == "oDamsel" && health > 0 {
                rescue_held_damsel(game, id, door_center);
            } else if matches!(object.as_str(), "oGoldIdol" | "oCrystalSkull") {
                redeem_held_idol(game, id, true);
            } else if object == "oDamsel" || heavy {
                if let Some(item) = game.world.entity_mut(id) {
                    // Ordinary exits put a dead Damsel in THROWN before
                    // release; its later own Step still runs that branch.
                    if object == "oDamsel" {
                        item.state = 2;
                    }
                    item.held = false;
                }
            } else if object == "oBomb" {
                reserved_carry = true;
                if !armed {
                    game.player.bombs += 1;
                    if let Some(item) = game.world.entity_mut(id) {
                        item.alive = false;
                        item.held = false;
                    }
                } else if let Some(item) = game.world.entity_mut(id) {
                    item.held = false;
                }
            } else if matches!(object.as_str(), "oRopeThrow" | "oRope") {
                reserved_carry = true;
                game.player.ropes += 1;
                if let Some(item) = game.world.entity_mut(id) {
                    item.alive = false;
                    item.held = false;
                }
            } else {
                // Source stores only pickupItem's type, suppressing fragments
                // and destroying the old instance before its exit animation.
                game.flags.retain(|f| !f.starts_with("carry:"));
                game.flags.insert(format!("carry:{object}"));
                if let Some(item) = game.world.entity_mut(id) {
                    item.data.insert("breakPieces".into(), 0.);
                    item.held = false;
                    item.alive = false;
                }
            }
        }
        game.player.held = None;
    }
    if reserved_carry {
        game.flags.retain(|flag| !flag.starts_with("carry:"));
        if let Some(object) = crate::items::explosives::hold_object(&game.player.pickup_item_type) {
            game.flags.insert(format!("carry:{object}"));
        }
    }
    if ending || had_held {
        game.player.pickup_item_type.clear();
    }
    game.player.pos = door_center;
    game.player.previous_pos = door_center;
    game.player.vel = Vec2::default();
    game.flags.insert("render_discontinuity".into());
    // Only the ordinary oExit branch advances the level and forgives theft.
    // xEND goes directly to the ending after the common collection payout.
    if !ending {
        // oPlayer1 Step pays its pending collection immediately at an ordinary
        // exit. The Animation End payout separately handles later collection.
        let collect = game.data.remove("collect").unwrap_or(0.).max(0.) as u32;
        game.player.money = game.player.money.saturating_add(collect);
        *game.data.entry("xmoney".into()).or_default() += f64::from(collect);
        if let Some(thief) = game.data.get_mut("thiefLevel") {
            *thief = (*thief - 1.).max(0.);
        }
        if game.data.get("thiefLevel").copied().unwrap_or(0.) == 0.
            && !game.flags.contains("murderer")
        {
            game.flags.remove("wanted");
        }
        game.data.insert(
            "next_level".into(),
            game.world.level.saturating_add(1) as f64,
        );
    }
    game.events.push(GameEvent::LevelComplete);
    crate::music::stop_all(game);
    game.sound("sndSteps", game.player.pos);
    let kind = if ending || (game.world.area == Area::Boss && object == "oExit") {
        crate::ending::EndingKind::Campaign
    } else {
        crate::ending::EndingKind::Transition
    };
    crate::ending::begin_exit(game, kind);
    if !ending {
        // Player Step932–943: ordinary exits knock off every active GRAB
        // monkey before its own later source Step can steal during the exit.
        // `with` follows the current descendant array and samples both rand
        // calls per affected instance, even when their difference is zero.
        let monkeys = crate::dispatcher::descendant_instances(game, "oMonkey");
        for instance in monkeys {
            let crate::dispatcher::Instance::Entity(id) = instance else {
                continue;
            };
            if !game
                .world
                .entity(id)
                .is_some_and(|monkey| monkey.engine_active && monkey.state == 7)
            {
                continue;
            }
            let x_vel = f64::from(game.rng.range(2)) - f64::from(game.rng.range(2));
            let monkey = game.world.entity_mut(id).unwrap();
            monkey.vel = Vec2::new(x_vel, -4.);
            monkey.state = 1;
            monkey.data.insert("vine_counter".into(), 20.);
            monkey.data.insert("grab_counter".into(), 60.);
        }
    }
}

/// Player Step775–984: portal actions and Game Over precede its hurt block.
pub(crate) fn player_actions(game: &mut Game, input: &InputFrame) {
    if game.state == RunState::Dead {
        acknowledge_game_over(game, input);
        return;
    }
    if game.state != RunState::Playing || !game.is_level() {
        return;
    }
    if !(input.up || input.exit_pressed)
        || !game.player.grounded
        || game.player.is_stunned()
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

/// Player Step1933: revival is after ordered hurt and before the dead latch.
pub(crate) fn player_revival(game: &mut Game) {
    if game.is_level()
        || ["hub_sun", "hub_moon", "hub_stars"]
            .iter()
            .any(|flag| game.flags.contains(*flag))
    {
        revive(game);
    }
}

/// Player Step2199: touching an ordinary exit deposits a remaining held item.
pub(crate) fn player_deposits(game: &mut Game) {
    if game.is_level() {
        deposit_at_exit(game);
    }
}

pub(crate) fn tunnel_step(game: &mut Game, input: &InputFrame) {
    donate(game, input);
}

/// Compatibility adapter for direct system callers. Game's authoritative
/// dispatcher calls the source event slices at their original boundaries.
pub fn tick(game: &mut Game, input: &InputFrame) {
    initialize_level(game);
    game_step(game, input);
    final_boss_step(game);
    player_actions(game, input);
    player_revival(game);
    player_deposits(game);
    gold_door_collision(game);
    tunnel_step(game, input);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;

    fn wanted_exit_room(exits: &[usize], wanted: &str) -> Game {
        let mut level = crate::level::Level::default();
        level.set_tile(3, 8, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 9, '1').unwrap();
        }
        for &x in exits {
            level.set_tile(x, 8, 'X').unwrap();
        }
        let mut game = Game::shell(1, RulesPreset::Classic);
        if wanted == "murderer" {
            game.flags.insert("murderer".into());
        } else if wanted == "thief" {
            game.data.insert("thiefLevel".into(), 1.);
        }
        game.load_custom(&level, "wanted-exits.lvl", "");
        game
    }

    fn guards_by_birth(game: &Game) -> Vec<&crate::Entity> {
        let mut guards: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oShopkeeper")
            .collect();
        guards.sort_by_key(|e| e.creation_serial);
        guards
    }

    #[test]
    fn wanted_exit_actual_step_guards_every_authored_exit_once_in_birth_order() {
        for wanted in ["murderer", "thief"] {
            let mut game = wanted_exit_room(&[10, 30], wanted);
            // Storage order must not replace the fresh source family's order.
            game.world.entities.reverse();
            game.step(&InputFrame::default());
            let guards = guards_by_birth(&game);
            assert_eq!(
                guards.iter().map(|e| e.pos).collect::<Vec<_>>(),
                [Vec2::new(176., 144.), Vec2::new(496., 144.)],
                "{wanted}"
            );
            for guard in &guards {
                assert_eq!(guard.value("enemy_context_created"), 1.);
                assert_eq!(guard.value("angry"), 0.);
                assert_eq!(guard.value("angered"), 0.);
            }
            let ids: Vec<_> = guards.iter().map(|e| e.id).collect();
            game.step(&InputFrame::default());
            assert_eq!(
                guards_by_birth(&game)
                    .iter()
                    .map(|e| e.id)
                    .collect::<Vec<_>>(),
                ids,
                "room initialization must not repeat"
            );
        }
    }

    #[test]
    fn wanted_exit_actual_step_keeps_preexisting_keeper_and_adds_source_guard() {
        let mut game = wanted_exit_room(&[30], "murderer");
        let existing = game.spawn("oShopkeeper", Vec2::new(496., 144.));
        game.step(&InputFrame::default());
        let guards = guards_by_birth(&game);
        assert_eq!(guards.len(), 2);
        assert!(guards.iter().any(|e| e.id == existing));
        assert!(guards.iter().all(|e| e.pos == Vec2::new(496., 144.)));
    }

    #[test]
    fn wanted_exit_actual_step_has_no_fallback_guard_without_an_exit() {
        let mut game = wanted_exit_room(&[], "murderer");
        game.step(&InputFrame::default());
        assert!(guards_by_birth(&game).is_empty());
    }

    #[test]
    fn wanted_exit_actual_step_does_not_guard_olmecs_entrance_only_room() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.start_from_hub(false, 16);
        game.flags.insert("murderer".into());
        assert_eq!(game.world.area, Area::Boss);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oEntrance")
                .count(),
            2
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| crate::catalog::is_a(&e.object, "oExit"))
        );
        game.step(&InputFrame::default());
        assert!(guards_by_birth(&game).is_empty());
    }

    #[test]
    fn wanted_exit_actual_step_leaves_nonwanted_custom_exits_unguarded() {
        let mut game = wanted_exit_room(&[10, 30], "");
        game.step(&InputFrame::default());
        assert!(guards_by_birth(&game).is_empty());
    }

    #[test]
    fn wanted_exit_actual_step_uses_inherited_create_type_and_runtime_overrides() {
        let mut game = wanted_exit_room(&[], "murderer");
        // oXMarket is produced before the guard pass by scrEntityGen. oXGold
        // normally appears later; its injected initial instance is a bounded
        // inherited-Create control, not a claim of a natural room-entry route.
        game.spawn("oXMarket", Vec2::new(176., 144.));
        game.spawn("oXGold", Vec2::new(240., 144.));
        let ordinary = game.spawn("oExit", Vec2::new(304., 144.));
        game.world
            .entity_mut(ordinary)
            .unwrap()
            .text
            .insert("type".into(), "Other Exit".into());
        let overridden_market = game.spawn("oXMarket", Vec2::new(368., 144.));
        game.world
            .entity_mut(overridden_market)
            .unwrap()
            .text
            .insert("type".into(), "Exit".into());
        game.step(&InputFrame::default());
        assert_eq!(
            guards_by_birth(&game)
                .iter()
                .map(|e| e.pos)
                .collect::<Vec<_>>(),
            [Vec2::new(240., 144.), Vec2::new(368., 144.)]
        );
    }

    fn game_over_blast_fixture(flags: &[&str]) -> Game {
        let mut game = Game::shell(123, RulesPreset::Classic);
        game.flags
            .extend(flags.iter().map(|flag| (*flag).to_owned()));
        for x in 0..game.world.width {
            game.world.set_tile(x, 3, "oBlock");
        }
        game.player.pos = Vec2::new(40., 40.);
        game.player.previous_pos = game.player.pos;
        game.player.money = 1000;
        game.world.spawn("oExplosion", game.player.pos);
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.lifetime_stats.level_deaths, [0; 16]);
        game
    }

    #[test]
    fn real_fatal_blast_game_over_waits_for_reveal_count_and_explicit_acknowledgment() {
        let mut game = game_over_blast_fixture(&[]);
        game.step(&InputFrame::default());
        assert_eq!(game.game_over.draw_status, 1);
        assert_eq!(game.alarms[0], 50);
        for _ in 0..50 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.game_over.draw_status, 2);
        assert_eq!(game.alarms[1], 49, "Alarm1 ticks after Alarm0 schedules it");
        for _ in 0..49 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.game_over.draw_status, 3);
        assert_eq!(
            game.game_over.money_count, 100,
            "exactly1000 uses the100 tier"
        );
        for _ in 0..120 {
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.state,
            RunState::Dead,
            "reveal readiness never routes by itself"
        );
        assert_eq!(game.game_over.money_count, 1000);
        assert_eq!(game.lifetime_stats.level_deaths, [0; 16]);
        game.step(&InputFrame {
            start_pressed: true,
            ..Default::default()
        });
        assert_eq!(
            game.game_over.destination,
            Some(GameOverDestination::Scores)
        );
        assert_eq!(game.lifetime_stats.level_deaths[0], 1);
        assert_eq!(game.state, RunState::Transition);
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.lifetime_stats.level_deaths[0], 1);
    }

    #[test]
    fn real_game_over_skip_and_ack_are_distinct_edges_with_source_routing_and_stat_gate() {
        for (flags, destination, counted) in [
            (&[][..], GameOverDestination::Scores, true),
            (&["tutorial"][..], GameOverDestination::Scores, false),
            (&["custom"][..], GameOverDestination::CustomLevels, false),
            (
                &["editor_test", "custom"][..],
                GameOverDestination::Editor,
                false,
            ),
        ] {
            let mut game = game_over_blast_fixture(flags);
            game.step(&InputFrame {
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Dead);
            assert_eq!(game.game_over.draw_status, 3);
            assert_eq!(game.game_over.money_count, game.player.money);
            assert!(game.game_over.destination.is_none());
            assert_eq!(game.lifetime_stats.level_deaths, [0; 16]);
            game.step(&InputFrame::default());
            game.step(&InputFrame {
                start_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.game_over.destination, Some(destination));
            assert_eq!(game.lifetime_stats.level_deaths[0], u64::from(counted));
        }
    }
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
        assert_eq!(g.state, RunState::Ending);
        assert_eq!(g.data["next_level"], 3.);
    }
    #[test]
    fn ankh_revives_once_inside_moai_and_resets_interpolation() {
        let mut g = game(10);
        g.player.health = 0;
        g.player.equipment.insert("Ankh".into());
        let p = Vec2::new(100., 100.);
        g.world.spawn("oMoai", Vec2::new(p.x - 16., p.y - 16.));
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
        g.player.equipment.insert("Ankh".into());
        g.player.pos = g.world.entrance;
        tick(&mut g, &InputFrame::default());
        assert_eq!(g.player.health, 4);
        assert_eq!(
            g.player.pos,
            Vec2::new(p.x + 8., p.y + 8.),
            "the Moai remains the revival anchor after its inner blocker is removed"
        );
    }
    #[test]
    fn ghost_uses_original_counter_and_spawns_once() {
        let mut g = game(2);
        g.level_tick = 4000;
        g.data.insert("xtime".into(), 120000.);
        tick(&mut g, &InputFrame::default());
        assert!(!g.flags.contains("ghost_warning"));
        g.level_tick = 4001;
        g.data.insert("xtime".into(), 120030.);
        tick(&mut g, &InputFrame::default());
        assert!(g.flags.contains("ghost_warning"));
        g.level_tick = 5001;
        g.data.insert("xtime".into(), 150030.);
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
        g.data.insert("xtime".into(), 300000.);
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
    fn boss_exit_begins_cinematic_and_custom_exit_keeps_link() {
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
        assert_eq!(g.state, RunState::Ending);
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
    fn generated_olmec_lava_death_opens_protected_exit_and_begins_ending() {
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
        assert_eq!(g.state, RunState::Ending);
        assert!(!g.events.contains(&GameEvent::Won));
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
    fn carried_exit(object: &str) -> (Game, u64) {
        let mut g = Game::shell(3, RulesPreset::Classic);
        g.world.spawn("oExit", Vec2::new(80., 48.));
        for x in 0..g.world.width {
            g.world.set_tile(x, 4, "oTemple");
        }
        g.player.pos = Vec2::new(85., 56.);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        let id = g.world.spawn(object, g.player.pos);
        g.world.entity_mut(id).unwrap().held = true;
        g.player.held = Some(id);
        (g, id)
    }
    #[test]
    fn actual_step_carries_light_items_but_drops_heavy_bodies_at_centered_exit() {
        for object in ["oRock", "oSkull", "oShotgun", "oSceptre"] {
            let (mut g, _) = carried_exit(object);
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(g.state, RunState::Ending);
            assert_eq!(g.player.pos, Vec2::new(88., 56.));
            assert!(g.flags.contains(&format!("carry:{object}")));
            g.next_level();
            let held = g.world.entity(g.player.held.unwrap()).unwrap();
            assert_eq!(held.object, object);
        }
        for object in ["oCrate", "oChest", "oLampItem"] {
            let (mut g, id) = carried_exit(object);
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(g.state, RunState::Ending);
            assert!(!g.flags.iter().any(|f| f.starts_with("carry:")));
            assert!(!g.world.entity(id).unwrap().held);
        }
    }
    #[test]
    fn actual_step_refunds_unarmed_bomb_and_rope_but_leaves_armed_bomb() {
        for (object, bombs, ropes) in [("oBomb", 5, 4), ("oRopeThrow", 4, 5)] {
            let (mut g, id) = carried_exit(object);
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(g.state, RunState::Ending);
            assert_eq!((g.player.bombs, g.player.ropes), (bombs, ropes));
            assert!(g.world.entity(id).is_none());
            assert!(!g.flags.iter().any(|f| f.starts_with("carry:")));
        }
        let (mut g, id) = carried_exit("oBomb");
        crate::items::explosives::arm_bomb(g.world.entity_mut(id).unwrap());
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(g.player.bombs, 4);
        assert!(!g.world.entity(id).unwrap().held);
        assert!(!g.flags.contains("carry:oBomb"));
    }
    #[test]
    fn actual_step_redeems_crystal_skull_and_rescues_held_damsel_once() {
        let (mut g, id) = carried_exit("oCrystalSkull");
        g.world.area = Area::Jungle;
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(
            g.player.money as f64 + g.data.get("collect").copied().unwrap_or(0.),
            30000.
        );
        assert!(g.world.entity(id).is_none());
        let (mut g, _) = carried_exit("oDamsel");
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(g.data["damsels"], 1.);
        assert_eq!(g.player.health, 4);
        g.next_level();
        assert_eq!(g.player.health, 5);
    }
    #[test]
    fn actual_step_inventory_bomb_exit_recreates_remembered_weapon_type() {
        let (mut g, id) = carried_exit("oShotgun");
        g.step(&InputFrame {
            cycle_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.pickup_item_type, "Shotgun");
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.player.bombs, 3);
        let bomb = g.player.held.unwrap();
        assert_eq!(g.world.entity(bomb).unwrap().object, "oBomb");
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(g.state, RunState::Ending);
        assert_eq!(g.player.bombs, 4);
        assert!(!g.player.data.contains_key("stash_id"));
        assert!(g.flags.contains("carry:oShotgun"));
        g.next_level();
        let held = g.world.entity(g.player.held.unwrap()).unwrap();
        assert_eq!(held.object, "oShotgun");
        assert_eq!(held.value("stashed"), 0.);
        assert!(held.pos.distance(g.player.pos) < 1.);
    }
    #[test]
    fn actual_step_udjat_alarm_holds_blink_and_alternates_source_sounds() {
        let (mut g, _) = carried_exit("oRock");
        g.player.pos = Vec2::new(88., 56.);
        g.player.previous_pos = g.player.pos;
        g.world.spawn("oXMarket", Vec2::new(92., 56.));
        g.player.equipment.insert("UdjatEye".into());
        g.step(&InputFrame::default());
        assert!(!g.flags.contains("udjat_blink"));
        for _ in 0..4 {
            g.step(&InputFrame::default());
        }
        assert!(g.flags.contains("udjat_blink"));
        assert!(
            g.events
                .iter()
                .any(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndBlink1"))
        );
        g.step(&InputFrame::default());
        assert!(g.flags.contains("udjat_blink"));
        for _ in 0..3 {
            g.step(&InputFrame::default());
        }
        assert!(!g.flags.contains("udjat_blink"));
        assert!(
            g.events
                .iter()
                .any(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndBlink2"))
        );
    }
    #[test]
    fn actual_step_gold_door_requires_held_sceptre_sprite_overlap() {
        let mut g = Game::shell(2, RulesPreset::Classic);
        for x in 0..g.world.width {
            g.world.set_tile(x, 4, "oTemple");
        }
        g.player.pos = Vec2::new(67., 56.);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.player.equipment.insert("Crown".into());
        let sceptre = g.world.spawn("oSceptre", g.player.pos);
        g.player.held = Some(sceptre);
        g.world.entity_mut(sceptre).unwrap().held = true;
        let door = g.world.spawn("oGoldDoor", Vec2::new(80., 48.));
        g.step(&InputFrame::default());
        assert!(g.world.entity(door).is_some());
        assert!(g.world.entity(sceptre).is_some());
        g.player.pos.x = 70.;
        g.player.previous_pos = g.player.pos;
        g.step(&InputFrame::default());
        assert!(g.world.entity(door).is_none());
        assert!(g.world.entity(sceptre).is_none());
        assert!(g.world.entities.iter().any(|e| e.object == "oXGold"));
    }
    #[test]
    fn actual_step_deposits_idol_and_damsel_without_exiting() {
        let (mut g, id) = carried_exit("oGoldIdol");
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert!(g.world.entity(id).is_none());
        assert!(g.player.held.is_none());
        assert_eq!(g.data["collect"], 5000.);
        assert_eq!(g.data["idols"], 1.);

        let (mut g, id) = carried_exit("oDamsel");
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert!(g.player.held.is_none());
        let rescued = g.world.entity(id).expect("damsel plays its exit animation");
        assert_eq!(rescued.state, 4);
        assert_eq!(rescued.pos, Vec2::new(88., 56.));
        assert_eq!(g.data["pending_damsel_kisses"], 1.);
        g.step(&InputFrame::default());
        assert_eq!(g.data["damsels"], 1., "rescue must be counted only once");
    }

    fn walk_carried_item_to_portal(object: &str, door: &str) -> (Game, u64) {
        let mut g = Game::shell(3, RulesPreset::Classic);
        g.world = crate::World::new(
            40,
            20,
            if door == "oXEnd" {
                Area::Boss
            } else {
                Area::Mines
            },
            if door == "oXEnd" { 16 } else { 1 },
        );
        for x in 0..g.world.width {
            g.world.set_tile(x, 4, "oTemple");
        }
        g.world.spawn(door, Vec2::new(128., 48.));
        g.player.pos = Vec2::new(88., 56.);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        let id = g.world.spawn(object, g.player.pos);
        g.step(&InputFrame::default());
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(id), "controller pickup: {object}");
        assert!(g.world.entity(id).unwrap().held);
        for _ in 0..60 {
            if g.player.pos.x >= 132. {
                break;
            }
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert!((132.0..144.0).contains(&g.player.pos.x));
        assert!(g.player.grounded);
        (g, id)
    }

    #[test]
    fn actual_pickup_and_walk_redeems_at_ordinary_exit_but_drops_at_ending() {
        for (object, counter, value) in [
            ("oGoldIdol", "idols", 5000.),
            ("oCrystalSkull", "skulls", 15000.),
            ("oDamsel", "damsels", 0.),
        ] {
            let (mut ordinary, id) = walk_carried_item_to_portal(object, "oExit");
            assert_eq!(ordinary.state, RunState::Playing);
            assert_eq!(ordinary.player.held, None);
            assert_eq!(ordinary.data[counter], 1., "ordinary exit: {object}");
            assert_eq!(
                ordinary.player.money as f64 + ordinary.data.get("collect").copied().unwrap_or(0.),
                value
            );
            if object == "oDamsel" {
                assert_eq!(ordinary.world.entity(id).unwrap().state, 4);
                assert_eq!(ordinary.data["pending_damsel_kisses"], 1.);
            } else {
                assert!(ordinary.world.entity(id).is_none());
            }
            ordinary.data.insert("thiefLevel".into(), 1.);
            ordinary.flags.insert("wanted".into());
            ordinary.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(ordinary.state, RunState::Ending);
            assert_eq!(ordinary.data["thiefLevel"], 0.);
            assert!(!ordinary.flags.contains("wanted"));
            assert_eq!(ordinary.data["next_level"], 2.);
            assert_eq!(
                ordinary.player.money as f64 + ordinary.data.get("collect").copied().unwrap_or(0.),
                value
            );

            let (mut ending, id) = walk_carried_item_to_portal(object, "oXEnd");
            // Merely touching the ending door is not an ordinary rescue or
            // treasure deposit, even though both portals share an exit sprite.
            assert_eq!(ending.state, RunState::Playing);
            assert_eq!(ending.player.held, Some(id));
            assert!(ending.world.entity(id).unwrap().held);
            assert_eq!(ending.data.get(counter).copied().unwrap_or(0.), 0.);
            assert_eq!(ending.data.get("collect").copied().unwrap_or(0.), 0.);
            assert_eq!(ending.player.money, 0);

            ending.data.insert("thiefLevel".into(), 1.);
            ending.flags.insert("wanted".into());
            ending.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(ending.state, RunState::Ending);
            assert_eq!(ending.player.held, None);
            let dropped = ending.world.entity(id).expect("heavy item is dropped");
            assert!(!dropped.held);
            if object == "oDamsel" {
                assert_ne!(dropped.state, 4);
                assert!(!ending.data.contains_key("pending_damsel_kisses"));
            }
            assert_eq!(ending.data.get(counter).copied().unwrap_or(0.), 0.);
            assert_eq!(ending.data.get("collect").copied().unwrap_or(0.), 0.);
            assert_eq!(ending.player.money, 0);
            assert!(!ending.flags.iter().any(|f| f.starts_with("carry:")));
            assert_eq!(ending.data["thiefLevel"], 1.);
            assert!(ending.flags.contains("wanted"));
            assert!(!ending.data.contains_key("next_level"));
        }
    }

    #[test]
    fn natural_monkey_grab_is_released_by_ordinary_exit_but_not_ending_portal() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for portal in ["oExit", "oXEnd"] {
                let mut level = crate::level::Level::default();
                level.starting_bombs = 0;
                level.starting_ropes = 0;
                level.set_tile(6, 7, '@').unwrap();
                for x in 0..crate::level::LEVEL_WIDTH {
                    level.set_tile(x, 8, '1').unwrap();
                }
                let mut game = Game::shell(31, rules);
                game.load_custom(&level, "monkey-exit", "");
                game.spawn(portal, Vec2::new(128., 128.));
                for _ in 0..8 {
                    game.step(&InputFrame::default());
                }
                assert!(game.player.grounded);
                game.spawn(
                    "oGoldBar",
                    Vec2::new(game.player.pos.x, game.player.pos.y + 4.),
                );
                for _ in 0..60 {
                    if game.player.money == 500 {
                        break;
                    }
                    game.step(&InputFrame::default());
                }
                assert_eq!(game.player.money, 500, "real treasure pickup/count-up");
                let monkey = game.spawn(
                    "oMonkey",
                    Vec2::new(game.player.pos.x - 8., game.player.pos.y - 8.),
                );
                game.step(&InputFrame::default());
                assert_eq!(
                    game.world.entity(monkey).unwrap().state,
                    7,
                    "actual automatic collision must acquire GRAB"
                );
                // Wait using real source counters, without forcing a state or
                // timer, so the old GRAB would steal during the live exit.
                for _ in 0..80 {
                    if game.world.entity(monkey).unwrap().timer <= 24 {
                        break;
                    }
                    game.step(&InputFrame::default());
                }
                assert_eq!(game.world.entity(monkey).unwrap().state, 7);
                let mut expected_rng = game.rng.clone();
                let mut entered = false;
                for _ in 0..20 {
                    expected_rng = game.rng.clone();
                    if portal == "oExit" {
                        expected_rng.range(2);
                        expected_rng.range(2);
                    }
                    game.step(&InputFrame {
                        right: true,
                        up: true,
                        ..Default::default()
                    });
                    if game.ending.is_some() {
                        entered = true;
                        break;
                    }
                }
                assert!(entered, "actual Right+Up must enter {portal}");
                assert_eq!(
                    serde_json::to_value(&game.rng).unwrap(),
                    serde_json::to_value(expected_rng).unwrap(),
                    "ordinary exit consumes exactly two source release draws"
                );
                let actor = game.world.entity(monkey).unwrap();
                if portal == "oExit" {
                    assert_ne!(actor.state, 7);
                    assert_eq!(actor.value("vine_counter"), 19.);
                    assert_eq!(actor.value("grab_counter"), 59.);
                    let mut entered_transition = false;
                    for _ in 0..40 {
                        if game.ending.as_ref().unwrap().stage
                            == crate::ending::EndingStage::Transition
                        {
                            entered_transition = true;
                            break;
                        }
                        game.step(&InputFrame::default());
                        assert_eq!(
                            game.player.money, 500,
                            "released monkey must not steal during the live exit"
                        );
                    }
                    assert!(entered_transition);
                } else {
                    assert_eq!(
                        actor.state, 7,
                        "source XEnd start-portal branch does not release monkeys"
                    );
                    assert_eq!(actor.value("grab_counter"), 0.);
                }
            }
        }
    }

    #[test]
    fn natural_exit_destroys_carried_instance_and_recreates_only_its_source_type() {
        for object in ["oRock", "oBow", "oShotgun"] {
            let (mut g, old_id) = walk_carried_item_to_portal(object, "oExit");
            g.data.insert("arrows".into(), 9.);
            let old = g.world.entity_mut(old_id).unwrap();
            old.data.insert("local_state_probe".into(), 31.);
            old.data.insert("cost".into(), 6000.);
            old.data.insert("new".into(), 1.);
            old.timer = 23;
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(g.state, RunState::Ending);
            assert_eq!(g.player.held, None);
            assert!(
                g.world.entity(old_id).is_none(),
                "old carried {object} is destroyed on Up"
            );
            assert!(g.flags.contains(&format!("carry:{object}")));
            // The public room continuation adapter implements scrHoldItem's
            // creation boundary; its source globals survive the room change.
            let mut without_carry = g.clone();
            without_carry
                .flags
                .retain(|flag| !flag.starts_with("carry:"));
            without_carry.next_level();
            g.next_level();
            assert_eq!(
                serde_json::to_value(&g.rng).unwrap(),
                serde_json::to_value(&without_carry.rng).unwrap(),
                "source {object} recreation has no Create randomness"
            );
            let item = g.world.entity(g.player.held.unwrap()).unwrap();
            assert_eq!(item.object, object);
            assert!(item.held);
            assert!(item.alive);
            assert!(!item.data.contains_key("local_state_probe"));
            assert_eq!(item.timer, 0);
            assert_eq!(item.value("cost"), 0.);
            assert_eq!(item.value("new"), 0.);
            assert_eq!(item.value("items_initialized"), 1.);
            assert_eq!(g.data["arrows"], 9., "Bow ammunition is a source global");
        }
    }

    #[test]
    fn actual_step_multiple_rescues_create_one_transition_kiss() {
        let (mut g, _) = carried_exit("oDamsel");
        g.step(&InputFrame::default());
        let id = g.world.spawn("oDamsel", g.player.pos);
        g.world.entity_mut(id).unwrap().held = true;
        g.player.held = Some(id);
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(g.state, RunState::Ending);
        assert_eq!(g.data["damsels"], 2.);
        assert_eq!(g.data["pending_damsel_kisses"], 2.);
        g.next_level();
        assert_eq!(g.player.health, 5);
        assert!(!g.data.contains_key("pending_damsel_kisses"));
    }

    #[test]
    fn actual_step_ankh_relocates_curse_and_clears_fall_history() {
        let mut g = Game::shell(8, RulesPreset::Classic);
        g.world.entrance = Vec2::new(88., 56.);
        for x in 0..g.world.width {
            g.world.set_tile(x, 4, "oTemple");
        }
        g.player.pos = Vec2::new(240., 56.);
        g.player.previous_pos = g.player.pos;
        g.player.health = 0;
        g.player.equipment.insert("Ankh".into());
        g.player.data.insert("fall_timer".into(), 80.);
        g.player.data.insert("fall_ticks".into(), 80.);
        let ball = g.world.spawn("oBall", Vec2::new(240., 40.));
        let chain = g.world.spawn("oChain", Vec2::new(248., 48.));
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.pos, g.world.entrance);
        assert_eq!(g.player.data["blink"], 59.);
        assert_eq!(g.player.data["fall_timer"], 0.);
        assert_eq!(g.player.data["fall_ticks"], 0.);
        for id in [ball, chain] {
            let curse = g.world.entity(id).unwrap();
            assert_eq!(curse.pos, g.player.pos);
            assert_eq!(curse.previous_pos, curse.pos);
        }
        for _ in 0..8 {
            g.step(&InputFrame::default());
        }
        assert_eq!(
            g.player.health, 4,
            "old fall must not hurt the revived player"
        );
        assert!(
            (g.player.pos.x - 88.).abs() < 8.,
            "old ball must not pull player away"
        );
    }
    #[test]
    fn ankh_uses_current_entrance_array_after_real_membership_rebuild() {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.world = crate::World::new(80, 40, Area::Mines, 1);
        g.world.name = "fixture".into();
        g.player.pos = Vec2::new(40., 72.);
        g.player.previous_pos = g.player.pos;
        for x in 0..80 {
            g.world.set_tile(x, 5, "oBrick");
        }
        let first = g.spawn("oEntrance", Vec2::new(24., 48.));
        let current = g.spawn("oEntrance", Vec2::new(240., 48.));
        g.world
            .entity_mut(first)
            .unwrap()
            .data
            .insert("depth".into(), 0.);
        g.world
            .entity_mut(current)
            .unwrap()
            .data
            .insert("depth".into(), 100.);
        g.spawn("oPoof", Vec2::new(600., 64.));
        g.step(&InputFrame::default());
        crate::dispatcher::activate_object(&mut g, "oPoof");
        assert_eq!(
            crate::dispatcher::descendant_instances(&g, "oEntrance")[0],
            crate::dispatcher::Instance::Entity(current)
        );
        // Controlled fatal state isolates the source's current-array lookup;
        // the preceding depth sort and activation are actual engine phases.
        g.player.health = 0;
        g.player.equipment.insert("Ankh".into());
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.pos, Vec2::new(248., 56.));
        assert!(!g.player.has("Ankh"));
    }

    #[test]
    fn actual_step_fatal_explosion_ankh_preserves_stun_and_sets_blink() {
        let mut g = Game::shell(5, RulesPreset::Classic);
        g.world.entrance = Vec2::new(88., 56.);
        g.player.pos = Vec2::new(240., 120.);
        g.player.previous_pos = g.player.pos;
        g.player.health = 1;
        g.player.equipment.insert("Ankh".into());
        g.world.spawn("oExplosion", g.player.pos);
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.pos, g.world.entrance);
        assert_eq!(g.player.invincible, 59);
        assert_eq!(g.player.data["blink"], 59.);
        assert_eq!(
            g.player.stun, 100,
            "Ankh must preserve the explosion's independent stun"
        );
        assert!(g.player.is_stunned());
        assert_eq!(
            g.player.state,
            PlayerState::Falling,
            "revival preserves the pre-existing movement controller rather than overwriting it"
        );
    }

    fn pickup_free_ankh() -> Game {
        let mut g = Game::new(42, RulesPreset::Classic);
        let ankh = g
            .world
            .spawn("oAnkh", Vec2::new(g.player.pos.x, g.player.pos.y + 4.));
        crate::items::init_entity(g.world.entity_mut(ankh).unwrap());
        let item = g.world.entity_mut(ankh).unwrap();
        item.data.insert("cost".into(), 0.);
        item.data.insert("forSale".into(), 0.);
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.player.has("Ankh"));
        g
    }

    #[test]
    fn controlled_intro_explosion_ankh_restores_active_and_stun_recovery() {
        let mut g = pickup_free_ankh();
        while g.world.level < 16 {
            g.next_level();
        }
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.data["active"], 0.);
        // Controlled hazard during the real generated intro, not a claim that
        // an explosion naturally occurs on arrival. The final blast frame
        // avoids repeated damage after revival and retains the real hurt path.
        let blast = g.world.spawn("oExplosion", g.player.pos);
        crate::items::init_entity(g.world.entity_mut(blast).unwrap());
        // Explosion lifetime is its source Animation End, not timer.
        g.world.entity_mut(blast).unwrap().animation = 9.6;
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.player.health, 4);
        assert!(!g.player.has("Ankh"));
        assert_eq!(g.player.data["active"], 1.);
        assert_eq!(g.player.stun, 100);
        assert_eq!(g.player.data["blink"], 59.);
        for _ in 0..200 {
            g.step(&InputFrame::default());
        }
        assert!(
            g.level_tick < 439,
            "intro alarm must not release the player"
        );
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.data["active"], 1.);
        assert_eq!(g.player.stun, 0);
        assert!(!g.player.is_stunned());
    }

    #[test]
    fn natural_ankh_pickup_then_exit_lava_revives_without_cancelling_pending_exit() {
        for character in [
            crate::characters::Character::Explorer,
            crate::characters::Character::Damsel,
            crate::characters::Character::TunnelMan,
        ] {
            let mut level = crate::level::Level::default();
            level.set_tile(6, 7, '@').unwrap();
            level.set_tile(7, 7, 'X').unwrap();
            for x in 0..40 {
                level.set_tile(x, 8, '1').unwrap();
            }
            let mut g = Game::shell(11, RulesPreset::Classic);
            g.load_custom(&level, "exit-ankh", "");
            crate::characters::apply(&mut g, character);
            for _ in 0..8 {
                g.step(&InputFrame::default());
            }
            let ankh = g
                .world
                .spawn("oAnkh", Vec2::new(g.player.pos.x, g.player.pos.y + 4.));
            crate::items::init_entity(g.world.entity_mut(ankh).unwrap());
            let item = g.world.entity_mut(ankh).unwrap();
            item.data.insert("cost".into(), 0.);
            item.data.insert("forSale".into(), 0.);
            g.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert!(g.player.has("Ankh"));
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
            assert_eq!(g.state, RunState::Ending);
            assert!(g.player.is_exiting());
            let sprite = g.player.sprite.clone();
            let phase = g.player.animation;
            // A controlled arriving lava tile tests the real hazard branch;
            // the pickup, walk and exit interaction use normal controllers.
            let lava = g
                .world
                .spawn("oLava", Vec2::new(g.player.pos.x - 8., g.player.pos.y - 8.));
            let age = g.world.entity(lava).unwrap().age;
            g.step(&InputFrame::default());
            assert_eq!(g.state, RunState::Ending, "{character:?}");
            assert_eq!(g.player.health, 4);
            assert!(!g.player.has("Ankh"));
            assert_eq!(g.player.data["active"], 1.);
            assert_eq!(g.player.data["blink"], 59.);
            assert_eq!(g.player.invincible, 59);
            assert_eq!(g.player.pos, g.world.entrance);
            assert_eq!(g.player.sprite, sprite);
            assert_eq!(g.player.animation, phase + 0.5);
            assert_eq!(
                g.ending.as_ref().unwrap().stage,
                crate::ending::EndingStage::Exit
            );
            assert!(
                g.events.contains(&GameEvent::Damage { amount: 99 }),
                "lava remains harmful"
            );
            assert!(!g.events.contains(&GameEvent::Died));
            assert_eq!(g.world.entity(lava).unwrap().age, age + 1);
            for _ in 0..40 {
                g.step(&InputFrame {
                    right: true,
                    jump: true,
                    jump_pressed: true,
                    ..Default::default()
                });
                if g.ending.as_ref().unwrap().stage != crate::ending::EndingStage::Exit {
                    break;
                }
                assert_eq!(
                    g.player.pos, g.world.entrance,
                    "source exit sprite still excludes character movement"
                );
                assert_eq!(g.player.sprite, sprite);
                assert_eq!(g.player.health, 4);
            }
            assert_eq!(
                g.ending.as_ref().unwrap().stage,
                crate::ending::EndingStage::Transition
            );
            assert_eq!(g.player.health, 4);
            assert!(!g.player.has("Ankh"));
        }
    }

    #[test]
    fn controlled_challenge_entry_ankh_revives_only_in_source_challenge_rooms() {
        use crate::hub::HubKind;
        for kind in [
            HubKind::Sun,
            HubKind::Moon,
            HubKind::Stars,
            HubKind::Title,
            HubKind::Scores,
        ] {
            let mut g = pickup_free_ankh();
            // The public room-loading API supplies a controlled carried-Ankh
            // entry. This does not claim that normal menu navigation carries
            // campaign equipment into a challenge in the original game.
            g.load_hub(kind);
            assert!(g.player.has("Ankh"));
            let room = g.world.name.clone();
            let time = g.data.get("time").copied();
            let blast = g.world.spawn("oExplosion", g.player.pos);
            crate::items::init_entity(g.world.entity_mut(blast).unwrap());
            // Explosion lifetime is its source Animation End, not timer.
            g.world.entity_mut(blast).unwrap().animation = 9.6;
            g.step(&InputFrame::default());
            if kind.challenge() {
                assert_eq!(g.state, RunState::Playing, "{kind:?}");
                assert_eq!(g.player.health, 4);
                assert!(!g.player.has("Ankh"));
                assert_eq!(g.player.pos, g.world.entrance);
                assert_eq!(g.player.invincible, 59);
                assert_eq!(g.player.data["blink"], 59.);
                assert_eq!(
                    g.events
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndTeleport"))
                        .count(),
                    1
                );
                for frame in 1..=12 {
                    g.step(&InputFrame::default());
                    if kind == HubKind::Sun {
                        // Sun's interior entrance floor is destructible. The
                        // unchanged late blast removes it; revival preserves
                        // stun, so the same neutral tape reaches contained Lava
                        // on frame 9. Source Lava damage ignores invincibility.
                        if frame < 9 {
                            assert_eq!(g.state, RunState::Playing);
                            assert_eq!(g.player.health, 4);
                        } else {
                            assert_eq!(g.state, RunState::Dead);
                        }
                        if frame == 9 {
                            assert!(g.player.invincible > 0);
                            assert!(g.events.contains(&GameEvent::Damage { amount: 99 }));
                            assert!(g.events.contains(&GameEvent::Died));
                            assert_eq!(g.player.health, 0);
                        }
                    }
                }
                if kind == HubKind::Sun {
                    assert_eq!(g.state, RunState::Dead);
                } else {
                    assert_eq!(g.state, RunState::Playing);
                    assert_eq!(g.player.health, 4);
                }
                assert!(!g.player.has("Ankh"));
            } else {
                assert_eq!(g.state, RunState::Playing, "{kind:?}");
                assert!(g.player.has("Ankh"));
                g.step(&InputFrame::default());
                assert_eq!(g.player.health, 4);
                assert!(g.player.has("Ankh"));
            }
            assert_eq!(g.world.name, room);
            assert_eq!(g.world.level, 0);
            assert_eq!(g.data.get("time").copied(), time);
            assert!(!g.data.contains_key("next_level"));
            assert!(!g.data.contains_key("progression_initialized"));
        }
    }

    #[test]
    fn actual_step_ankh_uses_first_entrance_even_when_linked_spawn_and_activation_differ() {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.world = crate::model::World::new(80, 20, Area::Mines, 1);
        g.world.spawn("oEntrance", Vec2::new(24., 48.));
        g.world.spawn("oEntrance", Vec2::new(700., 48.));
        g.world.entrance = Vec2::new(708., 56.);
        g.flags.insert("custom_level".into());
        g.player.pos = g.world.entrance;
        g.player.previous_pos = g.player.pos;
        g.player.health = 0;
        g.player.equipment.insert("Ankh".into());
        g.update_camera();
        assert!(g.camera.x > 400.);
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.player.pos, Vec2::new(32., 56.));
    }

    #[test]
    fn healed_latched_dead_player_cannot_activate_level_exit() {
        let (mut g, _) = carried_exit("oRock");
        g.player.health = 1;
        g.player.state = PlayerState::Dead;
        g.state = RunState::Dead;
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(g.state, RunState::Dead);
        assert!(!g.events.contains(&GameEvent::LevelComplete));
    }
    #[test]
    fn ankh_moai_getter_keeps_current_family_order_and_pending_birth_visibility() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut g = Game::shell(7, preset);
            let a = g.world.spawn("oMoai", Vec2::new(64., 64.));
            let b = g.world.spawn("oMoai", Vec2::new(192., 64.));
            let c = g.world.spawn("oMoai", Vec2::new(320., 64.));
            crate::dispatcher::register_instances(&mut g);
            // A bounded current-family getter fixture: C,A,B in native array,
            // C,B,A in independent storage, then retirement of C.
            g.dispatcher.registration_order = vec![
                crate::dispatcher::Instance::Entity(c),
                crate::dispatcher::Instance::Entity(a),
                crate::dispatcher::Instance::Entity(b),
            ];
            g.world.entities.sort_by_key(|e| {
                if e.id == c {
                    0
                } else if e.id == b {
                    1
                } else {
                    2
                }
            });
            g.world.entity_mut(c).unwrap().alive = false;
            let before = g.dispatcher.registration_order.clone();
            assert_eq!(moai_revival_anchor(&g), Some(Vec2::new(88., 88.)));
            g.world.entity_mut(a).unwrap().engine_active = false;
            assert_eq!(moai_revival_anchor(&g), Some(Vec2::new(216., 88.)));
            g.world.entity_mut(b).unwrap().engine_active = false;
            assert_eq!(moai_revival_anchor(&g), None);
            let pending = g.world.spawn("oMoai", Vec2::new(448., 64.));
            assert!(!before.contains(&crate::dispatcher::Instance::Entity(pending)));
            assert_eq!(moai_revival_anchor(&g), Some(Vec2::new(472., 88.)));
            assert_eq!(g.dispatcher.registration_order, before);
            g.player.health = 0;
            g.player.equipment.insert("Ankh".into());
            revive(&mut g);
            assert_eq!(g.player.health, 4);
            assert_eq!(g.player.pos, Vec2::new(472., 88.));
            assert!(!g.player.has("Ankh"));
            assert_eq!(g.dispatcher.registration_order, before);
        }
    }

    #[test]
    fn idol_exit_redemption_births_source_collection_effect_before_retirement() {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            for immediate in [false, true] {
                for object in ["oGoldIdol", "oCrystalSkull"] {
                    let mut g = Game::shell(3, preset);
                    g.world.area = Area::Jungle;
                    g.world.level = 5;
                    g.player.pos = Vec2::new(100.25, 120.5);
                    let id = g.spawn(object, Vec2::new(110.25, 108.5));
                    let idol = g.world.entity_mut(id).unwrap();
                    idol.data.insert("value".into(), 5000.);
                    idol.held = true;
                    g.player.held = Some(id);
                    let first = g
                        .world
                        .entities
                        .iter()
                        .map(|e| e.creation_serial)
                        .max()
                        .unwrap();
                    redeem_held_idol(&mut g, id, immediate);
                    let effects: Vec<_> = g
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oBigCollect")
                        .collect();
                    assert_eq!(effects.len(), 1);
                    assert_eq!(effects[0].pos, Vec2::new(100.25, 112.5));
                    assert!(effects[0].creation_serial > first);
                    assert_eq!(effects[0].alarms[0], 30);
                    assert!(g.world.entity(id).is_none());
                    assert!(g.player.held.is_none());
                    assert_eq!(g.data["idolsConverted"], 1.);
                    if immediate {
                        assert_eq!(g.player.money, 10000);
                    } else {
                        assert_eq!(g.data["collect"], 10000.);
                        assert_eq!(g.data["collectCounter"], 20.);
                    }
                    assert_eq!(
                        g.data[if object == "oCrystalSkull" {
                            "skulls"
                        } else {
                            "idols"
                        }],
                        1.
                    );
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "udjat_cached_distance_tests.rs"]
mod udjat_cached_distance_tests;
