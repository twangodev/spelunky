//! Run progression translated from oPlayer1 Step, oGame Step, GoldDoor
//! collision, and TunnelMan Step. Filesystem loading belongs to the host.
use crate::model::{
    Area, Game, GameEvent, InputFrame, PlayerState, Rect, RunState, VIEW_HEIGHT, VIEW_WIDTH, Vec2,
};

fn sprite_gap(a: Rect, b: Rect) -> f64 {
    let x = (a.x - b.x - b.w + 1.).max(b.x - a.x - a.w + 1.).max(0.);
    let y = (a.y - b.y - b.h + 1.).max(b.y - a.y - a.h + 1.).max(0.);
    x.hypot(y)
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
    // The Ankh branch activates oEntrance before selecting its first instance.
    // Cache that room-Create anchor while every entrance is still accessible;
    // ordinary activation need not expose distant doors during other queries.
    if let Some(entrance) = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oEntrance")
    {
        game.data
            .insert("progression_entrance_x".into(), entrance.pos.x + 8.);
        game.data
            .insert("progression_entrance_y".into(), entrance.pos.y + 8.);
    }
    game.flags.remove("ghost_exists");
    game.flags.remove("ghost_warning");
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
    if (game.flags.contains("murderer") || game.data.get("thiefLevel").copied().unwrap_or(0.) > 0.)
        && !game.world.entities.iter().any(|e| {
            e.alive
                && e.object == "oShopkeeper"
                && Vec2::new(e.pos.x + 8., e.pos.y + 8.).distance(game.world.exit) < 8.
        })
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
    if game.state == RunState::Dead
        || game.player.health > 0
        || !game.player.equipment.remove("Ankh")
    {
        return;
    }
    let moai = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oMoai")
        .map(|e| Vec2::new(e.pos.x + 24., e.pos.y + 24.));
    let destination = moai.unwrap_or_else(|| {
        if game.world.area == Area::Boss {
            Vec2::new(24., 552.)
        } else {
            game.data
                .get("progression_entrance_x")
                .zip(game.data.get("progression_entrance_y"))
                .map(|(&x, &y)| Vec2::new(x, y))
                .or_else(|| {
                    game.world
                        .entities
                        .iter()
                        .find(|e| e.alive && e.object == "oEntrance")
                        .map(|e| Vec2::new(e.pos.x + 8., e.pos.y + 8.))
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
            .map(|e| sprite_gap(e.sprite_bounds(), game.player.sprite_bounds()))
            .min_by(f64::total_cmp);
        if let Some(distance) = distance {
            let counter = game.data.entry("udjat_blink_counter".into()).or_default();
            if *counter <= 0. {
                // First Step schedules alarm 2; shortening an existing alarm
                // does not fire it early. Its blink state lasts until next fire.
                *counter = distance.max(4.);
            } else {
                *counter -= 1.;
                if *counter <= 0. {
                    *counter = distance.max(4.);
                    let on = !game.flags.remove("udjat_blink");
                    if on {
                        game.flags.insert("udjat_blink".into());
                    }
                    game.sound(if on { "sndBlink1" } else { "sndBlink2" }, game.player.pos);
                } else if *counter > distance.max(4.) {
                    *counter = distance.max(4.);
                }
            }
        } else {
            game.flags.remove("udjat_blink");
        }
    }
}

fn ghost(game: &mut Game) {
    // The level clock advances only while its player is alive. Dead world
    // ticks must not be reinterpreted as elapsed run time.
    let elapsed = game.data.get("xtime").copied().unwrap_or(0.);
    if game.world.level <= 1
        || game.world.area == Area::Boss
        || game.flags.contains("custom_level")
        || game.player.has("Crown")
        || game.player.health <= 0
    {
        return;
    }
    if elapsed > 120000. && game.flags.insert("ghost_warning".into()) {
        game.events.push(GameEvent::Message(
            "A CHILL RUNS UP YOUR SPINE... LET'S GET OUT OF HERE!".into(),
        ));
    }
    if elapsed > 150000. && !game.flags.contains("ghost_exists") {
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
    if !game.flags.contains("custom_level") && !game.flags.contains("tutorial") {
        count(game, "idolsConverted");
    }
    count(game, if crystal { "skulls" } else { "idols" });
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
                    item.held = false;
                }
            } else if object == "oBomb" {
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
    // Inventory selection temporarily hides the prior light item. Classic
    // carries pickupItemType through a bomb/rope exit, rather than its instance id.
    if let Some(stash) = game.player.data.remove("stash_id") {
        let stash = stash as u64;
        if let Some(item) = game.world.entity_mut(stash) {
            item.data.remove("stashed");
            item.held = false;
            item.data.insert("breakPieces".into(), 0.);
            item.alive = false;
            game.flags.retain(|flag| !flag.starts_with("carry:"));
            game.flags.insert(format!("carry:{}", item.object));
        }
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
    game.sound("sndSteps", game.player.pos);
    let kind = if ending || (game.world.area == Area::Boss && object == "oExit") {
        crate::ending::EndingKind::Campaign
    } else {
        crate::ending::EndingKind::Transition
    };
    crate::ending::begin_exit(game, kind);
}

pub fn tick(game: &mut Game, input: &InputFrame) {
    if game.state == RunState::Ending
        && game.ending.as_ref().is_some_and(|ending| {
            ending.stage == crate::ending::EndingStage::Exit
                && ending.kind != crate::ending::EndingKind::TitleCredits
        })
    {
        // Source lava precedes the exit-sprite hurt exclusion. Its Ankh branch
        // still runs afterward and restores life without clearing pExit/art.
        revive(game);
        return;
    }
    if !matches!(game.state, RunState::Playing | RunState::Dead) {
        return;
    }
    if game.flags.contains("hub") {
        // Player Step explicitly includes these challenge rooms in its Ankh
        // branch, while Title and Highscores remain outside that branch.
        if ["hub_sun", "hub_moon", "hub_stars"]
            .iter()
            .any(|flag| game.flags.contains(*flag))
        {
            revive(game);
        }
        return;
    }
    initialize_level(game);
    final_boss(game);
    if game.state == RunState::Dead {
        return;
    }
    revive(game);
    if game.player.health <= 0 || game.player.state == PlayerState::Dead {
        return;
    }
    secrets(game);
    ghost(game);
    donate(game, input);
    if !(input.up || input.exit_pressed)
        || !game.player.grounded
        || game.player.is_stunned()
        || game.player.attack_timer > 0
    {
        deposit_at_exit(game);
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
    } else {
        deposit_at_exit(game);
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
    fn actual_step_inventory_bomb_exit_restores_stashed_weapon_without_stale_id() {
        let (mut g, id) = carried_exit("oShotgun");
        g.step(&InputFrame {
            cycle_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.data["stash_id"], id as f64);
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
        assert_eq!(g.player.data["blink"], 60.);
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
        assert_eq!(g.player.invincible, 60);
        assert_eq!(g.player.data["blink"], 60.);
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
        g.world.entity_mut(blast).unwrap().timer = 1;
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.player.health, 4);
        assert!(!g.player.has("Ankh"));
        assert_eq!(g.player.data["active"], 1.);
        assert_eq!(g.player.stun, 100);
        assert_eq!(g.player.data["blink"], 60.);
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
            assert_eq!(g.player.data["blink"], 60.);
            assert_eq!(g.player.invincible, 60);
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
            g.world.entity_mut(blast).unwrap().timer = 1;
            g.step(&InputFrame::default());
            if kind.challenge() {
                assert_eq!(g.state, RunState::Playing, "{kind:?}");
                assert_eq!(g.player.health, 4);
                assert!(!g.player.has("Ankh"));
                assert_eq!(g.player.pos, g.world.entrance);
                assert_eq!(g.player.invincible, 60);
                assert_eq!(g.player.data["blink"], 60.);
                assert_eq!(
                    g.events
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndTeleport"))
                        .count(),
                    1
                );
                for _ in 0..12 {
                    g.step(&InputFrame::default());
                }
                assert_eq!(g.state, RunState::Playing);
                assert_eq!(g.player.health, 4);
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
}
