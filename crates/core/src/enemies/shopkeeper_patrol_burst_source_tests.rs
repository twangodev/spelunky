//! Configured source branches; ordinary corpse witness stays in economy unchanged.
use super::*;
use crate::RngAlgorithm;
// Independent literal RNG tape for the two supported source/replay algorithms.
struct Tape {
    state: u64,
    canonical: bool,
}
impl Tape {
    fn at(game: &Game) -> Self {
        Self {
            state: game.rng.state_bits(),
            canonical: game.rng.algorithm() == RngAlgorithm::Gm8Lcg32V1,
        }
    }
    fn next(&mut self) -> u64 {
        if self.canonical {
            self.state = u64::from((self.state as u32).wrapping_mul(0x08088405).wrapping_add(1));
            self.state
        } else {
            self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }
    }
    fn unit(&mut self) -> f64 {
        if self.canonical {
            self.next() as f64 / 4294967296.0
        } else {
            (self.next() >> 11) as f64 / 9007199254740992.0
        }
    }
    fn range(&mut self, n: u32) -> u32 {
        if self.canonical {
            ((self.next() * u64::from(n)) >> 32) as u32
        } else {
            (self.next() % u64::from(n)) as u32
        }
    }
    fn blood(&mut self) -> (Vec2, f64) {
        (
            Vec2::new(
                self.unit() * 4.0 - self.unit() * 4.0,
                -1.0 - self.unit() * 2.0,
            ),
            f64::from(1 + self.range(6)) * 0.1,
        )
    }
}

fn room(rules: RulesPreset, legacy: bool) -> Game {
    let mut g = if legacy {
        Game::legacy_shell(1, rules)
    } else {
        Game::shell(1, rules)
    };
    g.world = World::new(40, 20, Area::Mines, 1);
    for x in 0..40 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    g.player.pos = Vec2::new(320.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g
}
fn keeper() -> Entity {
    let mut e = Entity::new(90000, "oShopkeeper", Vec2::new(128.0, 144.0));
    crate::enemies::init_entity(&mut e);
    e
}
#[test]
fn literal_walk_edge_then_independent_pause_preserves_overwrite_and_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for facing in [-1, 1] {
                for supported in [false, true] {
                    // Initial seeds select branches independently of gameplay outcomes.
                    let mut saw_pause = false;
                    let mut saw_no_pause = false;
                    for seed in [1, 6, 29, 32, 155] {
                        let mut g = room(rules, legacy);
                        g.rng = if legacy {
                            crate::Rng::legacy(seed)
                        } else {
                            crate::Rng::new(seed as u32)
                        };
                        let mut e = keeper();
                        e.state = 1;
                        e.facing = facing;
                        e.timer = 77;
                        if supported {
                            g.world
                                .set_tile(if facing < 0 { 7 } else { 9 }, 9, "oBlock");
                        }
                        assert!(!game_center_solid(&g, &e));
                        let contact = crate::enemies::body_contacts(&g.world, &e);
                        assert!(!contact.left && !contact.right && contact.bottom);
                        let mut tape = Tape::at(&g);
                        let mut timer = if supported { 77 } else { 20 + tape.range(31) };
                        let pause = tape.range(100) == 0;
                        if pause {
                            timer = 20 + tape.range(31);
                            saw_pause = true;
                        } else {
                            saw_no_pause = true;
                        }
                        shopkeeper(&mut g, &mut e);
                        assert_eq!(e.state, if !supported || pause { 4 } else { 1 });
                        assert_eq!(e.timer, timer);
                        let expected = if pause { 0.0 } else { f64::from(facing) * 1.4 };
                        assert!((e.vel.x - expected).abs() < 1e-12);
                        assert_eq!(e.sprite, if pause { "sShopLeft" } else { "sShopRunLeft" });
                        assert_eq!(g.rng.state_bits(), tape.state);
                    }
                    assert!(
                        saw_pause && saw_no_pause,
                        "all literal independent pause outcomes covered"
                    );
                }
            }
        }
    }
}
fn game_center_solid(g: &Game, e: &Entity) -> bool {
    g.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0)
}
#[test]
fn keeper_point_queries_use_active_family_held_members_and_literal_precision() {
    let mut g = room(RulesPreset::Classic, false);
    let p = Vec2::new(144.0, 144.0);
    g.world.set_tile(9, 9, "oBlock");
    assert!(keeper_solid_point(&g, p, CollisionPrecision::Precise));
    g.world.tiles[9 * g.world.width + 9].engine_active = false;
    assert!(!keeper_solid_point(&g, p, CollisionPrecision::BoundingBox));
    let solid = g.spawn("oBlock", p);
    {
        let e = g.world.entity_mut(solid).unwrap();
        e.held = true;
        e.solid = false;
    }
    assert!(keeper_solid_point(&g, p, CollisionPrecision::BoundingBox));
    g.world.entity_mut(solid).unwrap().engine_active = false;
    assert!(!keeper_solid_point(&g, p, CollisionPrecision::BoundingBox));
    {
        let e = g.world.entity_mut(solid).unwrap();
        e.engine_active = true;
        e.set_sprite("sBlood");
    }
    let mask = g.world.entity(solid).unwrap().collision_mask();
    let hole = (-8..=16)
        .flat_map(|dy| (-8..=16).map(move |dx| Vec2::new(p.x + f64::from(dx), p.y + f64::from(dy))))
        .find(|q| {
            mask.contains(*q, CollisionPrecision::BoundingBox)
                && !mask.contains(*q, CollisionPrecision::Precise)
        })
        .expect("original Blood sprite has transparent pixels within its source bbox");
    assert!(keeper_solid_point(
        &g,
        hole,
        CollisionPrecision::BoundingBox
    ));
    assert!(!keeper_solid_point(&g, hole, CollisionPrecision::Precise));
}
#[test]
fn literal_burst_birth_order_all_launch_rng_and_born_solid_retirement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for facing in [-1, 1] {
                for obstruction in [0, 1, 2] {
                    let mut g = room(rules, legacy);
                    g.player.pos = Vec2::new(if facing < 0 { 80.0 } else { 184.0 }, 152.0);
                    let mut e = keeper();
                    e.state = 2;
                    e.facing = facing;
                    e.data.insert("turn_timer".into(), 10.0);
                    let muzzle =
                        Vec2::new(e.pos.x + if facing < 0 { 4.0 } else { 12.0 }, e.pos.y + 8.0);
                    if obstruction > 0 {
                        let solid = g.spawn("oBlock", muzzle);
                        let b = g.world.entity_mut(solid).unwrap();
                        // Initial narrow active Solid overlaps only the source muzzle,
                        // not the keeper's crush center or sampled body edges.
                        b.data.insert("image_xscale".into(), 0.125);
                        b.data.insert("image_yscale".into(), 0.0625);
                        b.engine_active = obstruction == 1;
                        b.solid = false;
                    }
                    assert!(!game_center_solid(&g, &e));
                    assert_eq!(
                        keeper_solid_point(&g, muzzle, CollisionPrecision::BoundingBox),
                        obstruction == 1
                    );
                    let mut tape = Tape::at(&g);
                    let expected: Vec<_> = (0..6)
                        .map(|_| {
                            (
                                f64::from(facing) * f64::from(9 + tape.range(3)),
                                tape.unit() - tape.unit(),
                            )
                        })
                        .collect();
                    g.events.clear();
                    shopkeeper(&mut g, &mut e);
                    let births: Vec<_> = g
                        .world
                        .entities
                        .iter()
                        .filter(|b| b.object == "oBullet" || b.object.starts_with("oShotgunBlast"))
                        .collect();
                    assert_eq!(births.len(), 7);
                    assert_eq!(
                        births[0].object,
                        if facing < 0 {
                            "oShotgunBlastLeft"
                        } else {
                            "oShotgunBlastRight"
                        }
                    );
                    assert_eq!(
                        births[0].pos,
                        Vec2::new(128.0 + if facing < 0 { 0.0 } else { 16.0 }, 153.0)
                    );
                    assert_eq!(births[0].image_speed(), 0.8);
                    for (b, (vx, vy)) in births[1..].iter().zip(expected) {
                        assert_eq!(b.object, "oBullet");
                        assert_eq!(b.pos, muzzle);
                        assert_eq!(b.vel, Vec2::new(vx, vy));
                        assert_eq!(b.value("safe"), 1.0);
                        assert_eq!(b.alive, obstruction != 1);
                    }
                    assert_eq!(g.rng.state_bits(), tape.state);
                    assert_eq!(e.value("firing"), 30.0);
                    assert_eq!(e.vel, Vec2::new(0.0, -1.0));
                    assert_eq!(
                        e.sprite, "sShopLeft",
                        "post-recoil zero selects source idle art"
                    );
                    assert!(!g.world.entities.iter().any(|b| b.object == "oSmokePuff"));
                    assert!(!g.events.iter().any(
                        |event| matches!(event,crate::GameEvent::Sound{name,..} if name=="sndHit")
                    ));
                    assert_eq!(g.events.iter().filter(|event|matches!(event,crate::GameEvent::Sound{name,..} if name=="sndShotgun")).count(),1);
                }
            }
        }
    }
}
#[test]
fn final_sprite_tail_applies_after_follow_friction_and_preserves_fraction() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules, false);
        g.player.pos = Vec2::new(64.0, 152.0);
        let gun = g.spawn("oShotgun", g.player.pos);
        g.player.held = Some(gun);
        g.world.entity_mut(gun).unwrap().held = true;
        let mut e = keeper();
        e.state = 5;
        e.facing = -1;
        e.animation = 0.5;
        shopkeeper(&mut g, &mut e);
        assert!(e.vel.x < 0.0);
        assert_eq!(e.sprite, "sShopRunLeft");
        assert_eq!(
            e.animation, 0.5,
            "source assignment keeps an index valid in new art"
        );
        e.state = 3;
        e.set_sprite("sShopThrowL");
        e.vel = Vec2::new(1.0, 0.0);
        shopkeeper(&mut g, &mut e);
        assert_eq!(e.sprite, "sShopThrowL");
        e.held = true;
        shopkeeper(&mut g, &mut e);
        assert_eq!(e.sprite, "sShopHeldL");
    }
}

#[test]
fn source_minus_one_walk_point_flags_admit_bbox_transparent_edge_pixels() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let mut g = room(rules, false);
            let solid = g.spawn("oBlock", Vec2::new(144.0, 144.0));
            g.world.entity_mut(solid).unwrap().set_sprite("sBlood");
            let mask = g.world.entity(solid).unwrap().collision_mask();
            let bounds = mask.point_bounds();
            let x = if facing > 0 {
                bounds.x
            } else {
                bounds.x + bounds.w - 1.0
            };
            let mut hole = (-8..=16)
                .map(|dy| Vec2::new(x, 144.0 + f64::from(dy)))
                .find(|point| {
                    mask.contains(*point, CollisionPrecision::BoundingBox)
                        && !mask.contains(*point, CollisionPrecision::Precise)
                })
                .expect("original Blood mask has transparent boundary pixels inside its bbox");
            // Authored initial source sprite/position overrides establish a
            // discriminator without changing actor state after any Game.step.
            g.world.entity_mut(solid).unwrap().pos.y += 144.0 - hole.y;
            hole.y = 144.0;
            let mut e = keeper();
            e.pos = Vec2::new(hole.x + if facing < 0 { 1.0 } else { -16.0 }, 144.0);
            e.previous_pos = e.pos;
            e.state = 1;
            e.facing = facing;
            e.timer = 77;
            let contacts = crate::enemies::body_contacts(&g.world, &e);
            assert!(!contacts.left && !contacts.right && contacts.bottom);
            assert!(!game_center_solid(&g, &e));
            assert!(keeper_solid_point(
                &g,
                hole,
                CollisionPrecision::BoundingBox
            ));
            assert!(!keeper_solid_point(&g, hole, CollisionPrecision::Precise));
            g.rng = crate::Rng::new(1);
            let mut tape = Tape::at(&g);
            assert_ne!(
                tape.range(100),
                0,
                "independent pause must not obscure edge result"
            );
            shopkeeper(&mut g, &mut e);
            assert_eq!(e.state, 1, "-1 source precision selects bbox support");
            assert_eq!(e.timer, 77, "bbox support skips the edge counter RNG");
            assert!((e.vel.x - f64::from(facing) * 1.4).abs() < 1e-12);
            assert_eq!(g.rng.state_bits(), tape.state);
        }
    }
}
