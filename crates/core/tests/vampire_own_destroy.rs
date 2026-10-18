//! Vampire own Step continues after instance_destroy. Source event lines48–79,
//! 267–286 and373–398; ordinary top-level Step has other==self. Initial crush
//! contacts are explicit authored fixtures, not a claimed dynamic-crush route.
use spelunky_core::*;

#[derive(Clone)]
struct LegacySourceTape(u64);
impl LegacySourceTape {
    fn at(game: &Game) -> Self {
        Self(
            serde_json::to_value(&game.rng).unwrap()["state"]
                .as_u64()
                .unwrap(),
        )
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
    fn rand(&mut self, width: u64) -> u64 {
        self.next() % width
    }
    fn random(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }
    fn bones_then_skull(&mut self) -> (Vec<(Vec2, f64)>, Vec2) {
        let bones = (0..3)
            .map(|_| {
                let velocity = Vec2::new(
                    self.random() * 4.0 - self.random() * 4.0,
                    -1.0 - self.random() * 2.0,
                );
                let gravity = (1 + self.rand(6)) as f64 * 0.1;
                (velocity, gravity)
            })
            .collect();
        let y = -((1 + self.rand(3)) as f64);
        let skull = Vec2::new(self.rand(4) as f64 - self.rand(4) as f64, y);
        (bones, skull)
    }
}

#[derive(Clone, Copy)]
enum OracleMode {
    Legacy,
    Canonical,
}
impl OracleMode {
    fn game(self, seed: u64, rules: RulesPreset) -> Game {
        match self {
            Self::Legacy => Game::legacy_shell(seed, rules),
            Self::Canonical => Game::shell(u32::try_from(seed).unwrap(), rules),
        }
    }
}

#[derive(Clone)]
struct CanonicalSourceTape(u32);
impl CanonicalSourceTape {
    fn at(game: &spelunky_core::Game) -> Self {
        assert_eq!(
            game.rng.algorithm(),
            spelunky_core::RngAlgorithm::Gm8Lcg32V1
        );
        Self(u32::try_from(game.rng.state_bits()).unwrap())
    }
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.0
    }
    fn rand(&mut self, width: u32) -> u32 {
        ((u64::from(self.next()) * u64::from(width)) >> 32) as u32
    }
    fn random(&mut self) -> f64 {
        f64::from(self.next()) / 4_294_967_296.0
    }
    fn bones_then_skull(&mut self) -> (Vec<(spelunky_core::Vec2, f64)>, spelunky_core::Vec2) {
        let bones = (0..3)
            .map(|_| {
                let velocity = spelunky_core::Vec2::new(
                    self.random() * 4.0 - self.random() * 4.0,
                    -1.0 - self.random() * 2.0,
                );
                let gravity = f64::from(1 + self.rand(6)) * 0.1;
                (velocity, gravity)
            })
            .collect();
        let y = -f64::from(1 + self.rand(3));
        let skull = spelunky_core::Vec2::new(f64::from(self.rand(4)) - f64::from(self.rand(4)), y);
        (bones, skull)
    }
}

#[derive(Clone)]
enum SourceTape {
    Legacy(LegacySourceTape),
    Canonical(CanonicalSourceTape),
}
impl SourceTape {
    fn at(game: &Game) -> Self {
        match game.rng.algorithm() {
            RngAlgorithm::LegacySplitMix64V1 => Self::Legacy(LegacySourceTape::at(game)),
            RngAlgorithm::Gm8Lcg32V1 => Self::Canonical(CanonicalSourceTape::at(game)),
        }
    }
    fn state(&self) -> u64 {
        match self {
            Self::Legacy(t) => t.0,
            Self::Canonical(t) => u64::from(t.0),
        }
    }
    fn rand(&mut self, width: u64) -> u64 {
        match self {
            Self::Legacy(t) => t.rand(width),
            Self::Canonical(t) => u64::from(t.rand(u32::try_from(width).unwrap())),
        }
    }
    fn bones_then_skull(&mut self) -> (Vec<(Vec2, f64)>, Vec2) {
        match self {
            Self::Legacy(t) => t.bones_then_skull(),
            Self::Canonical(t) => t.bones_then_skull(),
        }
    }
}

fn contact_room(mode: OracleMode, rules: RulesPreset, crushed: bool, dead: bool) -> (Game, u64) {
    let mut game = mode.game(42, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    if crushed {
        for y in 5..=6 {
            for x in 6..=7 {
                game.world.set_tile(x, y, "oBlock");
            }
        }
    }
    let id = game.spawn("oVampire", Vec2::new(96.0, 80.0));
    let actor = game.world.entity_mut(id).unwrap();
    assert_eq!((actor.health, actor.state), (6, 0));
    assert_eq!(actor.vel, Vec2::new(2.5, 0.0));
    assert_eq!(actor.value("bloodless"), 1.0);
    if dead {
        // Explicit initial DEAD state isolates collision/death branch order.
        // Natural damage is exercised separately; this is not an input journey.
        actor.health = 0;
        actor.state = 99;
        actor.vel = Vec2::default();
    }
    assert_eq!(game.world.solid_at(104.0, 88.0), crushed);
    (game, id)
}
fn births<'a>(game: &'a Game, object: &str) -> Vec<&'a Entity> {
    let mut result: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .collect();
    result.sort_by_key(|e| e.creation_serial);
    result
}

fn authored_healthy_crush_runs_idle_rng_without_inventing_death_loot_case(mode: OracleMode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = contact_room(mode, rules, true, false);
        let mut tape = SourceTape::at(&game);
        tape.rand(2); // own IDLE counter0 selects facing after crush Destroy.
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(id).is_none(),
            "OWN_CRUSH_RETIRES_RECEIVER"
        );
        assert_eq!(births(&game, "oSmokePuff").len(), 1);
        assert_eq!(births(&game, "oSmokePuff")[0].pos, Vec2::new(104.0, 88.0));
        assert!(births(&game, "oCapePickup").is_empty());
        assert!(births(&game, "oBone").is_empty());
        assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(game.player.health, 4.0);
        assert_eq!(
            SourceTape::at(&game).state(),
            tape.state(),
            "POST_CRUSH_IDLE_DRAW_MUST_RUN"
        );
    }
}

fn authored_dead_crush_keeps_bones_skull_free_cape_and_single_credit_case(mode: OracleMode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = contact_room(mode, rules, true, true);
        let mut tape = SourceTape::at(&game);
        let (bones, skull) = tape.bones_then_skull();
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(births(&game, "oSmokePuff").len(), 1);
        assert_eq!(
            births(&game, "oBone").len(),
            3,
            "CRUSH_DOES_NOT_ABORT_DEAD_BODY"
        );
        assert_eq!(births(&game, "oSkull").len(), 1);
        assert_eq!(births(&game, "oCapePickup").len(), 1);
        let origin = Vec2::new(104.0, 88.0);
        for (bone, (velocity, gravity)) in births(&game, "oBone").into_iter().zip(bones) {
            assert_eq!(bone.pos, origin, "BONE654_ALREADY_PASSED_BEFORE_VAMPIRE915");
            assert_eq!(bone.vel, velocity);
            assert_eq!(bone.value("gravity"), gravity);
            assert_eq!(bone.value("life"), 60.0, "NEWBORN_HAS_NOT_STEPPED");
        }
        assert_eq!(births(&game, "oSkull")[0].pos, origin);
        assert_eq!(births(&game, "oSkull")[0].vel, skull);
        let cape = births(&game, "oCapePickup")[0];
        assert_eq!(cape.pos, origin, "CAPE740_HAS_NO_SAME_FRAME_STEP");
        assert_eq!((cape.value("cost"), cape.value("forSale")), (0.0, 0.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.data.get("vampires"), Some(&1.0));
        assert_eq!(game.lifetime_stats.enemy_kills[7], 1);
        assert_eq!(
            SourceTape::at(&game).state(),
            tape.state(),
            "THREE_COMPLETE_BONE_CREATES_THEN_SKULL_LAUNCH"
        );
        game.step(&InputFrame::default());
        assert_eq!(
            game.data.get("kills"),
            Some(&1.0),
            "NO_REPEATED_DEATH_CREDIT"
        );
        assert_eq!(game.data.get("vampires"), Some(&1.0));
    }
}

fn uncrushed_idle_and_dead_controls_keep_existing_rng_and_birth_phase_case(mode: OracleMode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut healthy, id) = contact_room(mode, rules, false, false);
        let mut tape = SourceTape::at(&healthy);
        tape.rand(2);
        healthy.step(&InputFrame::default());
        assert!(healthy.world.entity(id).is_some());
        assert_eq!(SourceTape::at(&healthy).state(), tape.state());
        assert!(births(&healthy, "oSmokePuff").is_empty());
        let (mut dead, id) = contact_room(mode, rules, false, true);
        let mut tape = SourceTape::at(&dead);
        tape.bones_then_skull();
        dead.step(&InputFrame::default());
        assert!(dead.world.entity(id).is_none());
        assert_eq!(births(&dead, "oBone").len(), 3);
        assert_eq!(births(&dead, "oCapePickup").len(), 1);
        assert!(births(&dead, "oSmokePuff").is_empty());
        assert_eq!(SourceTape::at(&dead).state(), tape.state());
    }
}

fn ordinary_pistol_damage_reaches_vampire_death_without_actor_state_edits_case(mode: OracleMode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut level = level::Level::default();
        level.set_tile(4, 8, '@').unwrap();
        level.set_tile(5, 8, '-').unwrap();
        level.set_tile(14, 8, 'A').unwrap();
        // Keep ordinary Pistol knockback under a ceiling and inside the own-Step viewport.
        // Authored rows 7/9 map to world y128/160; the open corridor stays y144..159.
        for x in 6..=18 {
            level.set_tile(x, 7, '1').unwrap();
        }
        level.set_tile(18, 8, '1').unwrap();
        for x in 0..level::LEVEL_WIDTH {
            level.set_tile(x, 9, '1').unwrap();
        }
        let mut game = mode.game(42, rules);
        game.load_custom(&level, "vampire-pistol-death-control", "");
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oVampire")
            .unwrap()
            .id;
        assert_eq!(game.world.entity(id).unwrap().health, 6);
        let geometry: Vec<_> = game.world.tiles.iter().map(|t| t.object.clone()).collect();
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        for _ in 0..4 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let gun = game.player.held.expect("ACTUAL_AUTHORED_PISTOL_PICKUP");
        assert_eq!(game.world.entity(gun).unwrap().object, "oPistol");
        for _ in 0..2 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.facing, 1);
        let mut hits = 0;
        let mut hp = 6;
        let mut lethal = false;
        for tick in 0..240 {
            let fire = tick % 12 == 0;
            game.step(&InputFrame {
                attack: fire,
                attack_pressed: fire,
                ..Default::default()
            });
            assert!(game.player.health > 0.0, "NATURAL_CONTROLLER_SURVIVES");
            if let Some(actor) = game.world.entity(id) {
                if actor.health != hp {
                    assert_eq!(actor.health, hp - 4, "TWO_ORDINARY_BULLET_HITS");
                    hp = actor.health;
                    hits += 1;
                }
                if actor.health <= 0 {
                    lethal = true;
                    break;
                }
            } else {
                panic!("WAIT_FOR_FATAL_BULLET_BEFORE_NEXT_VAMPIRE_STEP");
            }
        }
        assert!(lethal, "NATURAL_LETHAL_BULLET_PRECONDITION");
        assert_eq!(hits, 2);
        let actor = game.world.entity(id).unwrap().clone();
        assert!(!actor.held && actor.value("burning") == 0.0);
        assert!(
            !game.world.solid_at(actor.pos.x + 8.0, actor.pos.y + 8.0),
            "NORMAL_DEATH_CONTROL_NOT_CRUSH"
        );
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(births(&game, "oBone").len(), 3);
        assert_eq!(births(&game, "oSkull").len(), 1);
        assert_eq!(births(&game, "oCapePickup").len(), 1);
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.data.get("vampires"), Some(&1.0));
        assert_eq!(
            game.world
                .tiles
                .iter()
                .map(|t| t.object.clone())
                .collect::<Vec<_>>(),
            geometry
        );
    }
}

// Separate registrations keep either RNG mode from masking the other.
#[test]
fn legacy_authored_healthy_crush_runs_idle_rng_without_inventing_death_loot() {
    authored_healthy_crush_runs_idle_rng_without_inventing_death_loot_case(OracleMode::Legacy);
}

#[test]
fn canonical_authored_healthy_crush_runs_idle_rng_without_inventing_death_loot() {
    authored_healthy_crush_runs_idle_rng_without_inventing_death_loot_case(OracleMode::Canonical);
}

#[test]
fn legacy_authored_dead_crush_keeps_bones_skull_free_cape_and_single_credit() {
    authored_dead_crush_keeps_bones_skull_free_cape_and_single_credit_case(OracleMode::Legacy);
}

#[test]
fn canonical_authored_dead_crush_keeps_bones_skull_free_cape_and_single_credit() {
    authored_dead_crush_keeps_bones_skull_free_cape_and_single_credit_case(OracleMode::Canonical);
}

#[test]
fn legacy_uncrushed_idle_and_dead_controls_keep_existing_rng_and_birth_phase() {
    uncrushed_idle_and_dead_controls_keep_existing_rng_and_birth_phase_case(OracleMode::Legacy);
}

#[test]
fn canonical_uncrushed_idle_and_dead_controls_keep_existing_rng_and_birth_phase() {
    uncrushed_idle_and_dead_controls_keep_existing_rng_and_birth_phase_case(OracleMode::Canonical);
}

#[test]
fn legacy_ordinary_pistol_damage_reaches_vampire_death_without_actor_state_edits() {
    ordinary_pistol_damage_reaches_vampire_death_without_actor_state_edits_case(OracleMode::Legacy);
}

#[test]
fn canonical_ordinary_pistol_damage_reaches_vampire_death_without_actor_state_edits() {
    ordinary_pistol_damage_reaches_vampire_death_without_actor_state_edits_case(
        OracleMode::Canonical,
    );
}
