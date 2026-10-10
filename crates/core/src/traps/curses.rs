//! Kali's iron ball and presentation-room chain variants. Source: oBall/2,
//! oChain/2 and the inherited oItem Step. Player tether limits run in player.rs.
use crate::model::{Entity, Game, Rect, Vec2};

pub(super) fn handles(object: &str) -> bool {
    matches!(object, "oBall" | "oBall2" | "oChain" | "oChain2")
}

/// oLevel Create restores a campaign punishment before the room starts.
/// This is a room initializer, never a ball's Step or a mid-room repair.
pub(super) fn initialize_level(game: &mut Game) {
    if !game.flags.contains("hub") && game.data.get("kali_punish").copied().unwrap_or(0.0) >= 2.0 {
        spawn_chain(game, game.player.pos, false);
    }
}

pub(super) fn init_entity(entity: &mut Entity) {
    if matches!(entity.object.as_str(), "oBall" | "oBall2") {
        init(entity);
    } else if matches!(entity.object.as_str(), "oChain" | "oChain2") {
        entity.data.entry("linkVal".into()).or_insert(2.0);
    }
}

pub(super) fn create_instance(game: &mut Game, instance: crate::dispatcher::Instance) {
    let crate::dispatcher::Instance::Entity(id) = instance else {
        return;
    };
    let Some(entity) = game.world.entity_mut(id) else {
        return;
    };
    if matches!(entity.object.as_str(), "oBall" | "oBall2") {
        crate::items::init_entity(entity);
    }
    init_entity(entity);
}

pub(super) fn step_instance(game: &mut Game, instance: crate::dispatcher::Instance) {
    let crate::dispatcher::Instance::Entity(id) = instance else {
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
    let mut entity = game.world.entities[index].clone();
    match entity.object.as_str() {
        "oBall" | "oBall2" => {
            init(&mut entity);
            crate::items::tick_inherited_ball_body(game, &mut entity);
            if entity.alive {
                // Child follow still runs when the inherited parent is gated.
                if entity.object == "oBall" {
                    follow_player(game, &mut entity);
                } else if let Some(dummy) = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oPDummy")
                {
                    if bounds_distance(
                        sprite_edges(&entity.sprite, entity.pos, entity.bounds()),
                        sprite_edges(&dummy.sprite, dummy.pos, dummy.bounds()),
                    ) >= 24.0
                    {
                        entity.pos.x = dummy.pos.x - 24.0;
                    }
                }
            }
        }
        "oChain" | "oChain2" => chain_step(game, &mut entity),
        _ => return,
    }
    if let Some(slot) = game.world.entities.iter_mut().find(|e| e.id == id) {
        *slot = entity;
    }
}

#[cfg(test)]
pub(super) fn tick(game: &mut Game) {
    if game.level_tick == 1 {
        initialize_level(game);
    }
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && handles(&e.object))
        .map(|e| e.id)
        .collect();
    for id in ids {
        step_instance(game, crate::dispatcher::Instance::Entity(id));
    }
}

fn init(ball: &mut Entity) {
    if ball.value("curse_initialized") != 0.0 {
        return;
    }
    ball.data.insert("curse_initialized".into(), 1.0);
    ball.data.insert("heavy".into(), 1.0);
    ball.data.insert("canPickUp".into(), 1.0);
    ball.data.insert("gravity".into(), 1.0);
    ball.data.insert("bounceFactor".into(), 0.5);
    ball.data.insert("frictionFactor".into(), 0.3);
    for (key, value) in [
        ("bboxLeft", -5.0),
        ("bboxTop", -5.0),
        ("bboxRight", 5.0),
        ("bboxBottom", 5.0),
        ("collision_left", -5.0),
        ("collision_top", -5.0),
        ("collision_right", 5.0),
        ("collision_bottom", 5.0),
    ] {
        ball.data.insert(key.into(), value);
    }
}

fn spawn_chain(game: &mut Game, pos: Vec2, presentation: bool) {
    game.world
        .spawn(if presentation { "oBall2" } else { "oBall" }, pos);
    for link in 1..=4 {
        let id = game
            .world
            .spawn(if presentation { "oChain2" } else { "oChain" }, pos);
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("linkVal".into(), f64::from(link));
    }
}

pub(super) fn bounds_distance(a: Rect, b: Rect) -> f64 {
    let dx = (a.x - (b.x + b.w)).max(b.x - (a.x + a.w)).max(0.0);
    let dy = (a.y - (b.y + b.h)).max(b.y - (a.y + a.h)).max(0.0);
    dx.hypot(dy)
}
fn sprite_edges(sprite: &str, pos: Vec2, fallback: Rect) -> Rect {
    let Some(s) = crate::catalog::sprite_def(sprite) else {
        return fallback;
    };
    Rect::new(
        pos.x + f64::from(s.bounds[0] - s.origin[0]),
        pos.y + f64::from(s.bounds[1] - s.origin[1]),
        f64::from(s.bounds[2] - s.bounds[0]),
        f64::from(s.bounds[3] - s.bounds[1]),
    )
}
fn follow_player(game: &Game, ball: &mut Entity) {
    if bounds_distance(
        sprite_edges(&ball.sprite, ball.pos, ball.bounds()),
        sprite_edges(&game.player.sprite, game.player.pos, game.player.bounds()),
    ) >= 24.0
    {
        let dx = game.player.pos.x - ball.pos.x;
        if dx.abs() >= 24.0 || ball.value("colBot") == 0.0 {
            if dx.abs() < 1.0 {
                ball.pos.x = game.player.pos.x;
                ball.vel.x = 0.0;
            }
            if game.player.pos.x > ball.pos.x {
                if game.player.vel.x > 0.0 && ball.pos.y >= game.player.pos.y {
                    ball.vel.x = game.player.vel.x;
                } else if ball.vel.x < 0.0 {
                    ball.vel.x *= -0.5;
                } else if ball.vel.x == 0.0 {
                    ball.vel.x = 2.0;
                }
            } else if game.player.pos.x < ball.pos.x {
                if game.player.vel.x < 0.0 && ball.pos.y >= game.player.pos.y {
                    ball.vel.x = game.player.vel.x;
                } else if ball.vel.x > 0.0 {
                    ball.vel.x *= -0.5;
                } else if ball.vel.x == 0.0 {
                    ball.vel.x = -2.0;
                }
            }
        } else {
            ball.vel.x *= 0.5;
            if ball.vel.x.abs() < 0.5 {
                ball.vel.x = 0.0;
            }
        }
        if (game.player.pos.y - ball.pos.y).abs() >= 24.0 && game.player.pos.y < ball.pos.y {
            ball.vel.y = 0.0;
        }
    } else if ball.value("colBot") != 0.0 {
        ball.vel.x = 0.0;
    }
}
fn chain_step(game: &Game, chain: &mut Entity) {
    let presentation = chain.object == "oChain2";
    let Some(ball) = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == if presentation { "oBall2" } else { "oBall" })
    else {
        // Only oChain explicitly destroys itself when its ball is absent.
        if !presentation {
            chain.alive = false;
        }
        return;
    };
    let endpoint = if presentation {
        game.world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oPDummy")
            .map(|e| e.pos)
            .unwrap_or(Vec2::new(280.0, 184.0))
    } else {
        game.player.pos
    };
    let link = chain
        .data
        .get("link_val")
        .or_else(|| chain.data.get("linkVal"))
        .copied()
        .unwrap_or(2.0);
    chain.pos = ball.pos.lerp(endpoint, link / 4.0);
}

#[cfg(test)]
fn chains(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oChain" | "oChain2"))
        .map(|e| e.id)
        .collect();
    for id in ids {
        step_instance(game, crate::dispatcher::Instance::Entity(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;
    fn room() -> Game {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.player.pos = Vec2::new(120.0, 80.0);
        g
    }
    #[test]
    fn instance_step_advances_only_its_curse() {
        let mut g = room();
        let first = g.world.spawn("oBall2", Vec2::new(72., 72.));
        let second = g.world.spawn("oBall2", Vec2::new(96., 72.));
        g.world.entity_mut(first).unwrap().vel.x = 3.;
        g.world.entity_mut(second).unwrap().vel.x = 3.;
        let second_before = serde_json::to_value(g.world.entity(second).unwrap()).unwrap();
        step_instance(&mut g, crate::dispatcher::Instance::Entity(first));
        assert_eq!(g.world.entity(first).unwrap().pos.x, 75.);
        assert_eq!(g.world.entity(second).unwrap().pos.x, 96.);
        assert_eq!(
            serde_json::to_value(g.world.entity(second).unwrap()).unwrap(),
            second_before
        );
    }

    #[test]
    fn offscreen_parent_does_not_hit_but_child_follow_still_runs() {
        let mut g = room();
        let enemy = g.world.spawn("oBat", Vec2::new(492., 64.));
        let health = g.world.entity(enemy).unwrap().health;
        let ball = g.world.spawn("oBall", Vec2::new(500., 72.));
        g.world.entity_mut(ball).unwrap().vel.x = 4.;
        step_instance(&mut g, crate::dispatcher::Instance::Entity(ball));
        assert_eq!(g.world.entity(enemy).unwrap().health, health);
        assert_eq!(g.world.entity(ball).unwrap().pos, Vec2::new(500., 72.));
        assert_eq!(
            g.world.entity(ball).unwrap().vel.x,
            -2.,
            "child follows despite inactive parent body"
        );
    }

    #[test]
    fn full_step_ball_precedes_earlier_created_chain_and_moves_once() {
        let mut g = room();
        // Object resource order, not creation order: Ball2 926 precedes Chain2 927.
        let chain = g.world.spawn("oChain2", Vec2::new(8., 8.));
        g.world
            .entity_mut(chain)
            .unwrap()
            .data
            .insert("linkVal".into(), 1.);
        let ball = g.world.spawn("oBall2", Vec2::new(72., 72.));
        g.world.entity_mut(ball).unwrap().vel.x = 3.;
        g.step(&crate::InputFrame::default());
        let ball = g.world.entity(ball).unwrap();
        assert_eq!(ball.pos.x, 75., "inherited item motion runs exactly once");
        assert_eq!(
            g.world.entity(chain).unwrap().pos,
            ball.pos.lerp(Vec2::new(280., 184.), 0.25)
        );
    }

    #[test]
    fn actual_ball_pickup_uses_inherited_heavy_attachment() {
        let mut level = crate::level::Level::default();
        level.set_tile(6, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut g = Game::shell(11, RulesPreset::Classic);
        g.load_custom(&level, "curse-held-route", "");
        for _ in 0..8 {
            g.step(&crate::InputFrame::default());
        }
        let ball = g
            .world
            .spawn("oBall", Vec2::new(g.player.pos.x, g.player.pos.y + 4.));
        g.step(&crate::InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(ball));
        assert!(g.world.entity(ball).unwrap().held);
        assert_eq!(g.world.entity(ball).unwrap().pos.y, g.player.pos.y - 2.);
        g.step(&crate::InputFrame {
            right: true,
            ..Default::default()
        });
        let ball = g.world.entity(ball).unwrap();
        assert_eq!(
            ball.pos,
            Vec2::new(
                g.player.pos.x + f64::from(g.player.facing) * 4.,
                g.player.pos.y - 4.
            )
        );
        assert_eq!(ball.vel, Vec2::default());
    }

    #[test]
    fn actual_altar_destroy_punishment_restores_once_in_next_room() {
        let mut level = crate::level::Level::default();
        level.set_tile(6, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut g = Game::shell(11, RulesPreset::Classic);
        g.load_custom(&level, "curse-altar-route", "");
        for _ in 0..8 {
            g.step(&crate::InputFrame::default());
        }
        for offense in 1..=2 {
            // Controlled unsupported altar; its ordinary Step and actual
            // Destroy event must cause punishment, without editing Kali data.
            g.world.spawn("oSacAltarLeft", Vec2::new(240., 64.));
            g.world.spawn("oSacAltarRight", Vec2::new(256., 64.));
            g.step(&crate::InputFrame::default());
            assert_eq!(g.data.get("kali_punish").copied(), Some(f64::from(offense)));
        }
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBall")
                .count(),
            1
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oChain")
                .count(),
            4
        );
        g.next_level();
        g.step(&crate::InputFrame::default());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBall")
                .count(),
            1
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oChain")
                .count(),
            4
        );
        for _ in 0..3 {
            g.step(&crate::InputFrame::default());
        }
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBall")
                .count(),
            1
        );
    }

    #[test]
    fn actual_ankh_pickup_relocates_before_ball_step_and_chain_sampling() {
        let mut level = crate::level::Level::default();
        level.set_tile(6, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut g = Game::shell(11, RulesPreset::Classic);
        g.load_custom(&level, "curse-ankh-order", "");
        for _ in 0..8 {
            g.step(&crate::InputFrame::default());
        }
        let ankh = g
            .world
            .spawn("oAnkh", Vec2::new(g.player.pos.x, g.player.pos.y + 4.));
        crate::items::init_entity(g.world.entity_mut(ankh).unwrap());
        g.world
            .entity_mut(ankh)
            .unwrap()
            .data
            .insert("cost".into(), 0.);
        g.world
            .entity_mut(ankh)
            .unwrap()
            .data
            .insert("forSale".into(), 0.);
        g.step(&crate::InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.player.has("Ankh"));
        let ball = g.world.spawn("oBall", Vec2::new(240., 40.));
        g.world.entity_mut(ball).unwrap().vel.x = 2.;
        let chain = g.world.spawn("oChain", Vec2::new(248., 48.));
        g.world
            .entity_mut(chain)
            .unwrap()
            .data
            .insert("linkVal".into(), 1.);
        // Controlled fatal producer after a real pickup; no injected player
        // health, velocity, stun, or revival state.
        g.world.spawn("oLava", g.player.pos);
        g.step(&crate::InputFrame::default());
        assert!(!g.player.has("Ankh"));
        assert_eq!(g.player.health, 4);
        assert_eq!(g.state, crate::RunState::Playing);
        let ball = g.world.entity(ball).unwrap();
        assert_eq!(
            ball.pos.x,
            g.player.pos.x + 2.,
            "preserved ball velocity runs after oPlayer1 revival"
        );
        assert_eq!(
            g.world.entity(chain).unwrap().pos,
            ball.pos.lerp(g.player.pos, 0.25)
        );
    }

    #[test]
    fn moving_curse_ball_hits_damsel_in_game_step() {
        let mut game = room();
        let damsel = game.world.spawn("oDamsel", Vec2::new(72.0, 80.0));
        let ball = game.world.spawn("oBall", Vec2::new(72.0, 72.0));
        game.world.entity_mut(ball).unwrap().vel.y = 4.0;
        game.step(&crate::InputFrame::default());
        let damsel = game.world.entity(damsel).unwrap();
        assert_eq!(damsel.health, 3);
        assert_eq!(damsel.state, 2);
        assert_eq!(damsel.timer, 120);
        assert_eq!(damsel.value("hit_cooldown"), 10.0);
        assert_eq!(damsel.vel.y, -6.0);
        let damsel_id = damsel.id;
        for _ in 0..10 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(
            game.world.entity(damsel_id).unwrap().value("hit_cooldown"),
            0.,
            "Damsel child Step must expire the inherited real-impact cooldown"
        );
    }

    #[test]
    fn hub_room_does_not_restore_campaign_punishment_ball() {
        let mut game = room();
        game.load_hub(crate::hub::HubKind::Title);
        game.data.insert("kali_punish".into(), 2.0);
        game.step(&crate::InputFrame::default());
        assert!(!game.world.entities.iter().any(|e| e.object == "oBall"));
    }
    #[test]
    fn links_interpolate_all_four_quarters_and_vanish_with_ball() {
        let mut g = room();
        spawn_chain(&mut g, Vec2::new(40.0, 80.0), false);
        chains(&mut g);
        let links: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oChain")
            .map(|e| e.pos.x)
            .collect();
        assert_eq!(links, vec![60.0, 80.0, 100.0, 120.0]);
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oBall")
            .unwrap()
            .alive = false;
        chains(&mut g);
        assert!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oChain")
                .all(|e| !e.alive)
        );
    }
    #[test]
    fn ball_uses_bounds_gap_and_follows_player_velocity() {
        let g = room();
        let mut e = Entity::new(1, "oBall", Vec2::new(70.0, 80.0));
        init(&mut e);
        let mut g = g;
        g.player.vel.x = 3.0;
        follow_player(&g, &mut e);
        assert_eq!(e.vel.x, 3.0);
        e.pos.x = 95.0;
        e.vel.x = 2.0;
        e.data.insert("colBot".into(), 1.0);
        follow_player(&g, &mut e);
        assert_eq!(e.vel.x, 0.0);
    }
    #[test]
    fn submerged_ball_is_destroyed_and_links_release() {
        let mut g = room();
        g.world.set_tile(4, 4, "oLava");
        spawn_chain(&mut g, Vec2::new(72.0, 73.0), false);
        tick(&mut g);
        assert!(
            g.world
                .entities
                .iter()
                .filter(|e| matches!(e.object.as_str(), "oBall" | "oChain"))
                .all(|e| !e.alive)
        );
    }
    #[test]
    fn water_does_not_apply_character_drag_to_iron_ball() {
        let mut g = room();
        g.world.set_tile(4, 4, "oWaterSwim");
        let mut e = Entity::new(1, "oBall", Vec2::new(72.0, 72.0));
        init(&mut e);
        e.vel.y = 2.0;
        crate::items::tick_inherited_ball_body(&mut g, &mut e);
        assert_eq!(e.pos.y, 74.0);
        assert_eq!(e.vel.y, 3.0);
        assert_eq!(e.value("gravity"), 0.6);
    }
    #[test]
    fn falling_ball_stuns_body_enemy_once_without_hurting_magma() {
        let mut g = room();
        let enemy = g.world.spawn("oCaveman", Vec2::new(64., 64.));
        let ball = g.world.spawn("oBall", Vec2::new(72., 68.));
        g.world.entity_mut(ball).unwrap().vel.y = 4.;
        step_instance(&mut g, crate::dispatcher::Instance::Entity(ball));
        assert_eq!(g.world.entity(enemy).unwrap().health, 2);
        assert_eq!(g.world.entity(enemy).unwrap().state, 98);
        step_instance(&mut g, crate::dispatcher::Instance::Entity(ball));
        assert_eq!(g.world.entity(enemy).unwrap().health, 2);

        let mut g = room();
        let magma = g.world.spawn("oMagmaMan", Vec2::new(64., 64.));
        let health = g.world.entity(magma).unwrap().health;
        let ball = g.world.spawn("oBall", Vec2::new(72., 68.));
        g.world.entity_mut(ball).unwrap().vel.y = 4.;
        step_instance(&mut g, crate::dispatcher::Instance::Entity(ball));
        assert_eq!(g.world.entity(magma).unwrap().health, health);
    }
    #[test]
    fn transition_variant_tracks_dummy_and_fallback_endpoint() {
        let mut g = room();
        spawn_chain(&mut g, Vec2::new(200.0, 184.0), true);
        chains(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oChain2" && e.value("linkVal") == 4.0)
                .unwrap()
                .pos,
            Vec2::new(280.0, 184.0)
        );
        let dummy = g.world.spawn("oPDummy", Vec2::new(300.0, 184.0));
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oBall2")
                .unwrap()
                .pos
                .x,
            g.world.entity(dummy).unwrap().pos.x - 24.0
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oChain2" && e.value("linkVal") == 4.0)
                .unwrap()
                .pos,
            g.world.entity(dummy).unwrap().pos
        );
    }
    #[test]
    fn room_entry_restores_curse_once() {
        let mut g = room();
        g.data.insert("kali_punish".into(), 2.0);
        g.level_tick = 1;
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBall")
                .count(),
            1
        );
        g.level_tick = 2;
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oBall")
            .unwrap()
            .alive = false;
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBall")
                .count(),
            1
        );
    }
}
