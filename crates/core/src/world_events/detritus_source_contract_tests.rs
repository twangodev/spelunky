use super::*;
use crate::{Area, RulesPreset, World};

fn room(seed: u32) -> Game {
    let mut g = Game::shell(seed, RulesPreset::Classic);
    g.world = World::new(32, 24, Area::Mines, 1);
    g.player.pos = Vec2::new(40.0, 104.0);
    g.world.set_tile(2, 7, "oBlock");
    g.player.previous_pos = g.player.pos;
    for x in 0..32 {
        g.world.set_tile(x, 14, "oBlock");
    }
    g
}
fn stored(g: &Game, id: u64) -> &Entity {
    g.world.entities.iter().find(|e| e.id == id).unwrap()
}
fn smokes(g: &Game) -> Vec<Vec2> {
    g.world
        .entities
        .iter()
        .filter(|e| e.object == "oSmokePuff")
        .map(|e| e.pos)
        .collect()
}
fn child_step(g: &mut Game, id: u64, object: &str) {
    if object == "oMagma" {
        crate::enemies::step_instance(g, id);
    } else {
        step_instance(g, id);
    }
}
fn next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    *state
}
fn child_create_oracle(state: &mut u32) -> (Vec2, f64) {
    let a = f64::from(next(state)) / 4294967296.0;
    let b = f64::from(next(state)) / 4294967296.0;
    let c = f64::from(next(state)) / 4294967296.0;
    let grav = (1 + ((u64::from(next(state)) * 6) >> 32)) as f64 * 0.1;
    (Vec2::new(a * 4.0 - b * 4.0, -1.0 - c * 2.0), grav)
}

#[test]
fn source_parent_expiry_continues_all_four_child_actions() {
    // Controlled callback receiver, not an asserted natural lifetime history.
    for object in ["oFlame", "oBlood", "oBone", "oMagma"] {
        let mut g = room(1);
        for x in 0..32 {
            g.world.set_tile(x, 10, "oBlock");
        }
        let extent = if object == "oMagma" { 8.0 } else { 4.0 };
        let id = g.spawn(object, Vec2::new(88.0, 160.0 - extent));
        if object == "oBlood" {
            alarm(&mut g, id, 1);
        }
        let e = g.world.entity_mut(id).unwrap();
        e.data.insert("life".into(), 0.0);
        e.data.insert("gravity".into(), 0.4);
        e.vel = Vec2::new(2.0, 2.0);
        child_step(&mut g, id, object);
        let e = stored(&g, id);
        assert!(!e.alive, "{object}");
        assert_eq!(
            e.value("life"),
            0.0,
            "expired life is not decremented: {object}"
        );
        assert_eq!(
            e.pos,
            Vec2::new(90.0, 160.0 - extent),
            "moveTo still runs: {object}"
        );
        if object == "oMagma" {
            assert_eq!(e.vel, Vec2::default());
            assert_eq!(e.sprite, "sMagmaManCreate");
            assert_eq!(e.value("dying"), 1.0);
        } else {
            assert_eq!(
                e.vel,
                Vec2::new(2.0, -1.2),
                "parent bounce still runs: {object}"
            );
            if object == "oBone" {
                assert_eq!(e.sprite, "sSmokePuff");
                assert_eq!(e.value("dying"), 1.0);
            }
        }
        if object == "oFlame" {
            assert_eq!(smokes(&g), vec![Vec2::new(88.0, 156.0)]);
        }
    }
}

#[test]
fn source_viewport_destroy_continues_life_movement_and_child_tail() {
    let mut g = room(1);
    let id = g.spawn("oFlame", Vec2::new(-5.0, 72.0));
    let e = g.world.entity_mut(id).unwrap();
    e.vel = Vec2::new(2.0, 1.0);
    e.data.insert("gravity".into(), 0.4);
    step_instance(&mut g, id);
    let e = stored(&g, id);
    assert!(!e.alive);
    assert_eq!(e.pos, Vec2::new(-3.0, 73.0));
    assert_eq!(e.value("life"), 59.0);
    assert_eq!(e.vel, Vec2::new(2.0, 1.4));
    assert_eq!(smokes(&g), vec![Vec2::new(-5.0, 72.0)]);
}

#[test]
fn source_lava_destroy_continues_bounce_and_magma_bottom_action() {
    for object in ["oFlame", "oBlood", "oBone", "oMagma"] {
        let mut g = room(1);
        g.world.set_tile(5, 5, "oLava");
        for x in 0..32 {
            g.world.set_tile(x, 6, "oBlock");
        }
        let extent = if object == "oMagma" { 8.0 } else { 4.0 };
        let id = g.spawn(object, Vec2::new(88.0, 96.0 - extent));
        if object == "oBlood" {
            alarm(&mut g, id, 1);
        }
        let e = g.world.entity_mut(id).unwrap();
        e.vel = Vec2::new(2.0, 2.0);
        e.data.insert("gravity".into(), 0.4);
        child_step(&mut g, id, object);
        let e = stored(&g, id);
        assert!(!e.alive, "{object}");
        assert_eq!(e.pos, Vec2::new(90.0, 96.0 - extent));
        if object == "oMagma" {
            assert_eq!(e.vel, Vec2::default());
            assert_eq!(e.sprite, "sMagmaManCreate");
            assert_eq!(e.value("dying"), 1.0);
        } else {
            assert_eq!(e.vel, Vec2::new(2.0, -1.2), "{object}");
            if object == "oBone" {
                assert_eq!(e.sprite, "sSmokePuff");
            }
        }
    }
}

#[test]
fn source_each_reached_flame_destroy_runs_at_its_call_position() {
    // Explicit callback control reaches viewport, life, lava, then own yVel
    // destruction in one Code chain. Native proof certifies repeated dispatch.
    let mut g = room(1);
    g.world.spawn("oLava", Vec2::new(-16.0, 64.0));
    let id = g.spawn("oFlame", Vec2::new(-5.0, 72.0));
    let e = g.world.entity_mut(id).unwrap();
    e.data.insert("life".into(), 0.0);
    e.vel = Vec2::new(2.0, 7.0);
    step_instance(&mut g, id);
    let e = stored(&g, id);
    assert!(!e.alive);
    assert_eq!(e.pos, Vec2::new(-3.0, 79.0));
    assert_eq!(e.value("life"), 0.0);
    assert_eq!(
        smokes(&g),
        vec![
            Vec2::new(-5.0, 72.0),
            Vec2::new(-5.0, 72.0),
            Vec2::new(-3.0, 79.0),
            Vec2::new(-3.0, 79.0)
        ]
    );
}

#[test]
fn natural_flame_expiry_smoke_starts_before_final_move() {
    let mut g = room(40);
    let mut oracle = 40;
    let (velocity, gravity) = child_create_oracle(&mut oracle);
    let id = g.spawn("oFlame", Vec2::new(160.0, 96.0));
    assert_eq!(g.world.entity(id).unwrap().vel, velocity);
    assert_eq!(g.world.entity(id).unwrap().value("gravity"), gravity);
    assert_eq!(g.rng.state_bits(), u64::from(oracle));
    for _ in 0..60 {
        g.step(&InputFrame::default());
    }
    let before = g
        .world
        .entity(id)
        .expect("natural particle reaches life zero")
        .clone();
    assert_eq!(before.value("life"), 0.0);
    assert!(smokes(&g).is_empty());
    g.step(&InputFrame::default());
    assert!(g.world.entity(id).is_none());
    // Smoke490 follows Flame456 in this same Normal Step, then moves up .1.
    assert_eq!(
        smokes(&g),
        vec![Vec2::new(before.pos.x, before.pos.y - 0.1)]
    );
}

#[test]
fn natural_flame_in_lava_smoke_starts_after_move() {
    let mut g = room(1);
    g.world.set_tile(5, 5, "oLava");
    let id = g.spawn("oFlame", Vec2::new(88.0, 88.0));
    let before = g.world.entity(id).unwrap().clone();
    let clock = g.engine_time() + 1;
    let pos = Vec2::new(
        before.pos.x + f64::from(displacement(before.vel.x, clock)),
        before.pos.y + f64::from(displacement(before.vel.y, clock)),
    );
    assert!(point_family(&g, Vec2::new(pos.x, pos.y - 4.0), "oLava"));
    g.step(&InputFrame::default());
    assert!(g.world.entity(id).is_none());
    assert_eq!(smokes(&g), vec![Vec2::new(pos.x, pos.y - 0.1)]);
}

#[test]
fn natural_runtime_cap_runs_flame_destroy_then_all_child_create_draws() {
    let mut g = room(1);
    for _ in 0..32 {
        g.spawn("oBone", Vec2::new(160.0, 96.0));
    }
    let mut state = u32::try_from(g.rng.state_bits()).unwrap();
    let (velocity, gravity) = child_create_oracle(&mut state);
    let id = g.spawn("oFlame", Vec2::new(160.0, 96.0));
    let flame = stored(&g, id);
    assert!(!flame.alive);
    assert_eq!(smokes(&g), vec![Vec2::new(160.0, 96.0)]);
    assert_eq!(flame.vel, velocity);
    assert_eq!(flame.value("gravity"), gravity);
    assert_eq!(flame.image_speed(), 0.3);
    assert_eq!(flame.alarms[0], 2);
    assert_eq!(flame.alarms[1], 50);
    assert_eq!(g.rng.state_bits(), u64::from(state));
}

#[test]
fn pending_authored_cap_sees_allocated_instances_before_their_create() {
    let mut g = room(1);
    let ids: Vec<_> = (0..33)
        .map(|_| g.world.spawn("oBone", Vec2::new(160.0, 96.0)))
        .collect();
    // This is the raw authored allocation visibility control, deliberately
    // distinct from runtime Game.spawn's sequential synchronous Creates.
    crate::dispatcher::create(&mut g, ids[0]);
    assert!(!stored(&g, ids[0]).alive);
    assert!(ids[1..].iter().all(|id| g.world.entity(*id).is_some()));
}

#[test]
fn source_bare_detritus_create_has_parent_defaults_and_no_child_random_draws() {
    let mut g = room(19);
    let state = g.rng.state_bits();
    let id = g.spawn("oDetritus", Vec2::new(88.0, 72.0));
    assert_eq!(
        g.rng.state_bits(),
        state,
        "parent Create contains no random call"
    );
    let e = stored(&g, id);
    assert!(e.alive);
    assert!(e.engine_active);
    assert_eq!(e.pos, Vec2::new(88.0, 72.0));
    assert_eq!(e.vel, Vec2::default());
    assert_eq!(e.image_speed(), 1.0);
    assert_eq!(e.alarms, [-1; 12]);
    for (key, expected) in [
        ("collision_left", -4.0),
        ("collision_top", -4.0),
        ("collision_right", 4.0),
        ("collision_bottom", 4.0),
        ("life", 60.0),
        ("gravity", 0.6),
        ("bounce", 1.0),
        ("dying", 0.0),
        ("invincible", 0.0),
    ] {
        assert_eq!(e.value(key), expected, "parent field {key}");
    }
}

#[test]
fn source_bare_detritus_cap_has_no_child_draw_or_flame_destroy_effect() {
    let mut g = room(19);
    let ids: Vec<_> = (0..32)
        .map(|_| g.spawn("oBone", Vec2::new(88.0, 72.0)))
        .collect();
    let state = g.rng.state_bits();
    let id = g.spawn("oDetritus", Vec2::new(88.0, 72.0));
    assert_eq!(
        g.rng.state_bits(),
        state,
        "capped parent still owns no random call"
    );
    let e = stored(&g, id);
    assert!(!e.alive);
    assert_eq!(e.value("life"), 60.0);
    assert_eq!(e.value("gravity"), 0.6);
    assert_eq!(e.image_speed(), 1.0);
    assert_eq!(e.vel, Vec2::default());
    assert!(smokes(&g).is_empty(), "only Flame owns Smoke on Destroy");
    assert!(ids.iter().all(|id| stored(&g, *id).alive));
}

#[test]
fn direct_detritus_parent_birth_runs_its_source_step() {
    let mut g = room(23);
    let id = g.spawn("oDetritus", Vec2::new(88.0, 72.0));
    let e = g.world.entity_mut(id).unwrap();
    e.vel = Vec2::new(2.0, 1.0);

    g.step(&InputFrame::default());

    let e = stored(&g, id);
    assert_eq!(e.value("life"), 59.0);
    assert_eq!(e.pos, Vec2::new(90.0, 73.0));
    assert_eq!(e.vel, Vec2::new(2.0, 1.6));
    assert_eq!(e.value("bounce"), 1.0);
}
