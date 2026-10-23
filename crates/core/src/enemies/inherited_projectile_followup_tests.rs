//! Configured moving-body callback branches, not a stock-room frequency claim.
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

fn fixture(legacy: bool, rules: RulesPreset, object: &str) -> (Game, Entity, u64) {
    let mut game = if legacy {
        Game::legacy_shell(71, rules)
    } else {
        Game::shell(71, rules)
    };
    game.world = World::new(40, 20, Area::Mines, 1);
    game.player.pos = Vec2::new(320.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let id = game.spawn(object, Vec2::new(88.0, 144.0));
    let mut donor = Entity::new(90000, "oCaveman", Vec2::new(80.0, 144.0));
    donor.state = 98;
    donor.vel = Vec2::new(6.0, 0.0);
    game.events.clear();
    (game, donor, id)
}

#[test]
fn moving_body_skips_retired_overlap_and_hits_next_live_enemy() {
    let (mut game, donor, retired) = fixture(false, RulesPreset::Classic, "oCaveman");
    let live = game.spawn("oCaveman", Vec2::new(88.0, 144.0));
    game.world.entity_mut(retired).unwrap().alive = false;
    let live_health = game.world.entity(live).unwrap().health;

    body_projectile(&mut game, &donor);

    assert!(game.world.entity(retired).is_none());
    assert_eq!(game.world.entity(live).unwrap().health, live_health - 1);
}

#[test]
fn moving_body_emits_literal_target_blood_before_damage_with_source_caps() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (object, offset) in [
                ("oCaveman", (8.0, 8.0)),
                ("oVampire", (8.0, 8.0)),
                ("oYeti", (8.0, 8.0)),
                ("oHawkman", (8.0, 8.0)),
                ("oShopkeeper", (0.0, 0.0)),
                ("oGiantSpider", (16.0, 24.0)),
                ("oTombLord", (16.0, 16.0)),
                ("oBat", (8.0, 8.0)),
            ] {
                for (cap, bloodless) in [(0, false), (15, false), (16, false), (0, true)] {
                    let (mut g, donor, id) = fixture(legacy, rules, object);
                    if object == "oGiantSpider" {
                        // This configured positive branch selects a warm,
                        // expired-cooldown target, unlike source newborn10.
                        assert_eq!(g.world.entity(id).unwrap().value("hit_cooldown"), 10.0);
                        g.world
                            .entity_mut(id)
                            .unwrap()
                            .data
                            .insert("hit_cooldown".into(), 0.0);
                    }
                    g.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("bloodless".into(), f64::from(u8::from(bloodless)));
                    for n in 0..cap {
                        g.spawn("oBone", Vec2::new(400.0 + f64::from(n), 80.0));
                    }
                    let hp = g.world.entity(id).unwrap().health;
                    let mut tape = Tape::at(&g);
                    let expected = if cap < 16 && !bloodless {
                        Some(tape.blood())
                    } else {
                        None
                    };
                    body_projectile(&mut g, &donor);
                    assert_eq!(g.world.entity(id).unwrap().health, hp - 1, "{object}");
                    assert_eq!(
                        g.rng.state_bits(),
                        tape.state,
                        "{object} cap{cap} bloodless{bloodless}"
                    );
                    let drops: Vec<_> = g
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oBlood")
                        .collect();
                    assert_eq!(drops.len(), usize::from(expected.is_some()), "{object}");
                    if let Some((velocity, gravity)) = expected {
                        assert_eq!(drops[0].pos, Vec2::new(88.0 + offset.0, 144.0 + offset.1));
                        assert_eq!(drops[0].vel, velocity);
                        assert_eq!(drops[0].value("gravity"), gravity);
                    }
                    assert!(g.events.iter().any(|event| matches!(event,crate::GameEvent::Sound{name,pos} if name=="sndHit"&&*pos==Vec2::new(88.0,144.0))));
                }
            }
        }
    }
}

#[test]
fn moving_body_mantrap_leaf_has_only_two_coordinate_draws_and_no_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            let (mut g, donor, id) = fixture(legacy, rules, "oManTrap");
            let hp = g.world.entity(id).unwrap().health;
            let mut tape = Tape::at(&g);
            let pos = Vec2::new(
                88.0 + f64::from(tape.range(17)),
                136.0 + f64::from(tape.range(17)),
            );
            body_projectile(&mut g, &donor);
            let leaf = g
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oLeaf")
                .unwrap();
            assert_eq!(leaf.pos, pos);
            assert_eq!(leaf.vel, Vec2::new(0.0, 0.4));
            assert_eq!(leaf.value("gravity"), 0.01);
            assert_eq!(g.world.entity(id).unwrap().health, hp - 1);
            assert_eq!(g.world.entity(id).unwrap().state, 98);
            assert_eq!(g.rng.state_bits(), tape.state);
            assert!(!g.world.entities.iter().any(|e| e.object == "oBlood"));
        }
    }
}

#[test]
fn moving_body_target_guards_suppress_damage_effects_and_rng() {
    for object in [
        "oCaveman",
        "oYeti",
        "oShopkeeper",
        "oGiantSpider",
        "oTombLord",
        "oAlienBoss",
    ] {
        for invincible in [false, true] {
            let (mut g, donor, id) = fixture(false, RulesPreset::Classic, object);
            let target = g.world.entity_mut(id).unwrap();
            target.state = 98;
            target.data.insert("hit_cooldown".into(), 1.0);
            if object == "oAlienBoss" {
                target.set_sprite("sAlienBossHurt");
            }
            if invincible {
                target.data.insert("invincible".into(), 1.0);
            }
            let hp = target.health;
            let rng = g.rng.state_bits();
            body_projectile(&mut g, &donor);
            assert_eq!(g.world.entity(id).unwrap().health, hp);
            assert_eq!(g.rng.state_bits(), rng);
            assert!(g.events.is_empty());
            assert!(
                !g.world
                    .entities
                    .iter()
                    .any(|e| matches!(e.object.as_str(), "oBlood" | "oLeaf"))
            );
        }
    }
}

#[test]
fn moving_body_ufo_orders_explosion_ejection_and_destroy_without_extra_hit() {
    for seed in 1..=12 {
        let (mut g, donor, id) = fixture(false, RulesPreset::Classic, "oUFO");
        // Initial RNG selection supplies both legal source ejection branches.
        g.rng = crate::Rng::new(seed);
        let hp = g.world.entity(id).unwrap().health;
        let mut tape = Tape::at(&g);
        let eject = tape.range(3) == 0;
        if eject {
            tape.range(2);
        }
        body_projectile(&mut g, &donor);
        let raw = g.world.entities.iter().find(|e| e.id == id).unwrap();
        assert!(!raw.alive);
        assert_eq!(raw.health, hp);
        let births: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| matches!(e.object.as_str(), "oExplosion" | "oAlienEject"))
            .map(|e| e.object.as_str())
            .collect();
        assert_eq!(
            births,
            if eject {
                vec!["oExplosion", "oAlienEject"]
            } else {
                vec!["oExplosion"]
            }
        );
        assert_eq!(g.data["ufos"], 1.0);
        assert_eq!(g.data["kills"], 1.0);
        assert_eq!(g.rng.state_bits(), tape.state);
        assert!(
            !g.events
                .iter()
                .any(|event| matches!(event,crate::GameEvent::Sound{name,..} if name=="sndHit"))
        );
    }
}

#[test]
fn source_newborn_giant_spider_is_protected_from_moving_body() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            let (mut g, donor, id) = fixture(legacy, rules, "oGiantSpider");
            assert_eq!(g.world.entity(id).unwrap().value("hit_cooldown"), 10.0);
            let hp = g.world.entity(id).unwrap().health;
            let rng = g.rng.state_bits();
            body_projectile(&mut g, &donor);
            assert_eq!(g.world.entity(id).unwrap().health, hp);
            assert_eq!(g.world.entity(id).unwrap().value("hit_cooldown"), 10.0);
            assert_eq!(g.rng.state_bits(), rng);
            assert!(g.events.is_empty());
            assert!(!g.world.entities.iter().any(|e| e.object == "oBlood"));
        }
    }
}
