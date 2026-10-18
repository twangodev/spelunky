//! Default enemies, ordinary whip/pickup/throw, and a deliberate visible fringe
//! aim. All scene writes precede the first input. Geometry failures are fixture
//! admission failures, not evidence of a damage omission.
use spelunky_core::{
    Area, Entity, Game, GameEvent, InputFrame, Rect, RngAlgorithm, RulesPreset, RunState, Vec2,
    World, catalog,
};

fn healthy(g: &Game) {
    assert_eq!(g.state, RunState::Playing);
    assert_eq!(g.player.health, 4.0);
    assert!(!g.player.is_stunned());
}

fn held_body(g: &Game, id: u64) {
    healthy(g);
    assert_eq!(g.player.held, Some(id));
    let e = g
        .world
        .entity(id)
        .expect("real stunned Caveman remains live");
    assert!(e.held && e.engine_active);
    assert_eq!(e.health, 2);
    assert_eq!(e.state, 98);
    assert!(
        e.timer > 10,
        "no recovery may invalidate the throw producer"
    );
}

fn prepare(rules: RulesPreset) -> (Game, u64, u64) {
    // Seed0 is fixed by the initial idle direction draws, not a hit search.
    let mut g = Game::shell(0, rules);
    g.world = World::new(30, 16, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..30 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    g.player.pos = Vec2::new(80.0, 152.0);
    g.player.previous_pos = g.player.pos;
    // One tile farther leaves the source whip windup outside the living
    // Caveman's Character contact gate; do not chase during that windup.
    let body = g.spawn("oCaveman", Vec2::new(120.0, 144.0));
    let snake = g.spawn("oSnake", Vec2::new(224.0, 144.0));
    assert_eq!(g.world.entity(body).unwrap().health, 3);
    assert_eq!(g.world.entity(body).unwrap().state, 0);
    assert_eq!(g.world.entity(snake).unwrap().health, 1);
    assert_eq!(g.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    for _ in 0..4 {
        g.step(&InputFrame::default());
        healthy(&g);
    }
    assert!(g.player.grounded);
    assert_eq!(g.player.facing, 1);
    g.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    for _ in 0..12 {
        healthy(&g);
        if g.world.entity(body).unwrap().state == 98 {
            break;
        }
        g.step(&InputFrame::default());
    }
    let actor = g.world.entity(body).unwrap();
    assert_eq!(actor.state, 98, "FIXTURE_REAL_WHIP_STUN");
    assert_eq!(actor.health, 2, "exactly one ordinary whip damage");
    g.step(&InputFrame {
        right_released: true,
        attack_released: true,
        ..Default::default()
    });
    let mut settled = false;
    for _ in 0..60 {
        healthy(&g);
        let actor = g.world.entity(body).unwrap();
        assert_eq!(actor.health, 2);
        assert_eq!(actor.state, 98);
        if actor.vel == Vec2::default()
            && actor.value("bounced") != 0.0
            && g.player.attack_timer == 0
        {
            settled = true;
            break;
        }
        g.step(&InputFrame::default());
    }
    assert!(settled, "FIXTURE_REAL_STUN_LANDING_AND_ATTACK_END");
    for _ in 0..48 {
        let x = g.world.entity(body).unwrap().pos.x + 8.0;
        let delta = x - g.player.pos.x;
        if delta.abs() <= 4.0 {
            break;
        }
        g.step(&InputFrame {
            right: delta > 0.0,
            left: delta < 0.0,
            ..Default::default()
        });
        healthy(&g);
    }
    // Source Down braking and fresh press; no direct pose/velocity edits.
    for _ in 0..2 {
        g.step(&InputFrame {
            down: true,
            left_released: true,
            right_released: true,
            ..Default::default()
        });
        healthy(&g);
    }
    g.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    held_body(&g, body);
    assert_eq!(g.player.pickup_item_type, "Caveman");
    g.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    for _ in 0..3 {
        g.step(&InputFrame::default());
        held_body(&g, body);
    }
    assert!(g.player.grounded);
    (g, body, snake)
}

fn probes(body: &Entity, snake: &Entity) -> (bool, bool) {
    assert_eq!(snake.sprite, "sSnakeWalkL");
    assert!(snake.engine_active);
    let scale = Vec2::new(
        snake.data.get("image_xscale").copied().unwrap_or(1.0),
        snake.data.get("image_yscale").copied().unwrap_or(1.0),
    );
    assert_eq!(scale, Vec2::new(1.0, 1.0));
    assert_eq!(snake.value("image_angle"), 0.0);
    let mask = catalog::instance_mask(
        &snake.object,
        &snake.sprite,
        snake.pos,
        snake.animation,
        scale,
        0.0,
    );
    (
        mask.intersects_rectangle_bbox(body.pos, Vec2::new(body.pos.x + 16.0, body.pos.y + 16.0)),
        Rect::new(body.pos.x, body.pos.y, 17.0, 17.0).intersects(snake.sprite_bounds()),
    )
}

fn quiet(g: &Game) -> bool {
    !g.world
        .entities
        .iter()
        .any(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
}

fn displacement(v: f64, time: u64) -> f64 {
    let fraction = v.abs().fract();
    let extra =
        fraction > 0.0 && time.is_multiple_of((1.0 / fraction).round_ties_even().max(1.0) as u64);
    f64::from(v.abs().floor() as i32 + i32::from(extra)) * v.signum()
}

struct Literal(u32);
impl Literal {
    fn raw(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x08088405).wrapping_add(1);
        self.0
    }
    fn unit(&mut self) -> f64 {
        f64::from(self.raw()) / 4_294_967_296.0
    }
    fn range(&mut self, n: u32) -> u32 {
        ((u64::from(self.raw()) * u64::from(n)) >> 32) as u32
    }
    fn blood(&mut self) -> (Vec2, f64) {
        let vx = 4.0 * self.unit() - 4.0 * self.unit();
        let vy = -1.0 - 2.0 * self.unit();
        (Vec2::new(vx, vy), f64::from(1 + self.range(6)) * 0.1)
    }
}

#[test]
fn ordinary_whip_pickup_throw_hits_default_snake_at_cached_bbox_fringe() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, body, snake) = prepare(rules);
        let mut ready = false;
        // Ordinary steering/timing aims the carried body at a visible bbox
        // fringe. It never mutates an actor to manufacture the discriminator.
        for _ in 0..140 {
            held_body(&g, body);
            let actor = g.world.entity(body).unwrap();
            let target = g.world.entity(snake).unwrap();
            assert_eq!(target.health, 1);
            assert_eq!(target.value("invincible"), 0.0);
            let (source, old) = probes(actor, target);
            if source && !old && target.state == 1 && quiet(&g) && g.player.vel == Vec2::default() {
                ready = true;
                break;
            }
            let gap = target.pos.x - (actor.pos.x + 16.0);
            // Chase the actual fringe, retaining RIGHT rather than flipping
            // the held origin by eight pixels. One Down brake at gap <= -2
            // gives vx3 -> .48; releasing Duck sets vx/ax to zero. A walking
            // Snake advances two pixels while the player advances at most
            // one, leaving the source-only gap 0/-1 at a stationary throw.
            let ducking = matches!(g.player.state, spelunky_core::PlayerState::Ducking);
            let down = !ducking && g.player.vel.x > 0.0 && gap <= -2.0;
            let right = !ducking && !down && (gap > 0.0 || (g.player.vel.x > 0.0 && gap > -2.0));
            g.step(&InputFrame {
                right,
                down,
                ..Default::default()
            });
        }
        assert!(ready, "FIXTURE_DEFAULT_TARGET_SOURCE_FRINGE_REQUIRED");
        let origin = g.world.entity(body).unwrap().pos;
        let target_origin = g.world.entity(snake).unwrap().pos;
        let facing = f64::from(g.player.facing);
        assert_eq!(g.player.vel, Vec2::default());
        assert_eq!(g.player.attack_timer, 0);
        assert_eq!(g.world.entity(body).unwrap().value("bounced"), 1.0);
        let mut oracle = Literal(u32::try_from(g.rng.state_bits()).unwrap());
        let drops: Vec<_> = (0..4).map(|_| oracle.blood()).collect();
        // Current Snake remains WALK, including after synchronous death. Its
        // final pause roll follows all four Blood Creates and may draw counter.
        if oracle.range(100) == 0 {
            oracle.range(31);
        }
        assert_eq!(catalog::object("oPlayer1").unwrap().id, 319);
        assert_eq!(catalog::object("oCaveman").unwrap().id, 325);
        assert_eq!(catalog::object("oBlood").unwrap().id, 370);
        assert_eq!(catalog::object("oSnake").unwrap().id, 552);
        let next_time = g.engine_time().wrapping_add(1) % 100_000_001;
        let first_birth = g.world.next_id;
        g.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        healthy(&g);
        assert_eq!(g.player.held, None, "REAL_ORDINARY_BODY_THROW");
        let thrown = g.world.entity(body).unwrap();
        assert!(!thrown.held && thrown.engine_active);
        assert_eq!(thrown.state, 98);
        assert_eq!(thrown.health, 2);
        assert_eq!(
            thrown.pos,
            Vec2::new(origin.x + facing * 4.0, origin.y - 2.0)
        );
        assert!((thrown.vel.x - facing * 3.9).abs() < 1e-9);
        assert!((thrown.vel.y + 1.4).abs() < 1e-9);
        assert!(
            g.world.entity(snake).is_none(),
            "SOURCE_BODY_CACHED_RECTANGLE_DAMAGE"
        );
        let blood: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.id >= first_birth && e.object == "oBlood")
            .collect();
        assert_eq!(blood.len(), 4, "body hit1 before Snake death3");
        for (i, e) in blood.iter().enumerate() {
            let x = target_origin.x + if i == 0 { 8.0 } else { facing * 4.0 + 8.0 };
            let birth = Vec2::new(x, target_origin.y + 8.0);
            // Caveman325's impact Blood runs later at370; Snake552's death
            // drops wait. All four use the same literal Create velocity.
            let expected = if i == 0 {
                Vec2::new(
                    birth.x + displacement(drops[i].0.x, next_time),
                    birth.y + displacement(drops[i].0.y, next_time),
                )
            } else {
                birth
            };
            assert_eq!(e.pos, expected);
            assert_eq!(e.value("life"), if i == 0 { 59.0 } else { 60.0 });
            assert_eq!(
                e.value("bounce"),
                0.0,
                "first Blood Step retains Create velocity"
            );
            assert!((e.vel.x - drops[i].0.x).abs() < 1e-9);
            assert!((e.vel.y - drops[i].0.y).abs() < 1e-9);
            assert!((e.value("gravity") - drops[i].1).abs() < 1e-9);
            assert_eq!(e.alarms[..3], [1, 1, 5]);
        }
        assert_eq!(
            g.rng.state_bits(),
            u64::from(oracle.0),
            "literal quiet-frame birth/tail RNG"
        );
        assert_eq!(g.data.get("snakes").copied().unwrap_or(0.0), 1.0);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 1.0);
        assert_eq!(g.lifetime_stats.enemy_kills[1], 1);
        assert_eq!(
            g.events
                .iter()
                .filter(|e| matches!(e,
            GameEvent::Sound {name,..} if name == "sndHit"))
                .count(),
            1
        );
    }
}

#[test]
fn ordinary_stunned_body_throw_away_keeps_default_snake_healthy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, body, snake) = prepare(rules);
        for _ in 0..2 {
            g.step(&InputFrame {
                left: true,
                ..Default::default()
            });
            held_body(&g, body);
        }
        assert_eq!(g.player.facing, -1);
        assert!(
            !probes(
                g.world.entity(body).unwrap(),
                g.world.entity(snake).unwrap()
            )
            .0
        );
        g.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        healthy(&g);
        assert_eq!(g.player.held, None);
        assert!(!g.world.entity(body).unwrap().held);
        assert!(g.world.entity(body).unwrap().vel.x < -2.0);
        assert_eq!(g.world.entity(snake).unwrap().health, 1);
        assert_eq!(g.data.get("snakes").copied().unwrap_or(0.0), 0.0);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert!(g.events.iter().any(|e| matches!(e,
            GameEvent::Sound {name,..} if name == "sndThrow")));
    }
}
