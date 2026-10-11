//! Real pickup, pursuit, and attack input; no mid-run actor/HP/RNG edits.
use spelunky_core::{Area, Game, InputFrame, RngAlgorithm, RulesPreset, Vec2, World};

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

fn fixture(legacy: bool, rules: RulesPreset, weapon: &str, facing: i8) -> (Game, u64, u64) {
    let mut game = if legacy {
        Game::legacy_shell(42, rules)
    } else {
        Game::shell(42, rules)
    };
    game.world = World::new(40, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(if facing > 0 { 80.0 } else { 240.0 }, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    // Initial authored hard floor prevents Mattock digging outputs/RNG.
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    let item = game.spawn(weapon, game.player.pos);
    let bat = game.spawn(
        "oBat",
        Vec2::new(if facing > 0 { 208.0 } else { 112.0 }, 144.0),
    );
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(item), "REAL_WEAPON_PICKUP");
    assert!(game.world.entity(item).unwrap().held);
    game.step(&InputFrame::default());
    game.step(&InputFrame {
        left: facing < 0,
        right: facing > 0,
        ..Default::default()
    });
    assert_eq!(game.player.facing, facing);
    assert_eq!(game.player.health, 4);
    assert!(game.player.grounded);
    assert_eq!(
        game.world.entity(bat).unwrap().state,
        1,
        "default Bat naturally flies"
    );
    assert_eq!(game.world.entity(bat).unwrap().health, 1);
    (game, bat, item)
}

fn natural_front_case(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (weapon, hitbox) in [("oMachete", "oSlash"), ("oMattock", "oMattockHit")] {
            for facing in [-1, 1] {
                let (mut game, target, item) = fixture(legacy, rules, weapon, facing);
                let mut reached = false;
                for _ in 0..80 {
                    let bat = game.world.entity(target).unwrap();
                    let bbox = bat.sprite_bounds();
                    let near_edge = if facing > 0 {
                        bbox.x
                    } else {
                        bbox.x + bbox.w - 1.0
                    };
                    let gap = (near_edge - game.player.pos.x) * f64::from(facing);
                    if gap <= 30.0 {
                        assert!(gap > 20.0, "approach must stop before body contact");
                        reached = true;
                        break;
                    }
                    game.step(&InputFrame {
                        left: facing < 0,
                        right: facing > 0,
                        ..Default::default()
                    });
                    assert_eq!(game.player.health, 4, "approach cannot damage player");
                }
                assert!(
                    reached,
                    "ordinary walking must reach source attack distance"
                );
                assert_eq!(game.player.held, Some(item));
                let mut hit = false;
                for frame in 0..16 {
                    let mut tape = Tape::at(&game);
                    game.step(&InputFrame {
                        attack: frame == 0,
                        attack_pressed: frame == 0,
                        ..Default::default()
                    });
                    assert_eq!(
                        game.player.health, 4,
                        "natural fixture must avoid Bat contact"
                    );
                    let bat = game
                        .world
                        .entity(target)
                        .expect("observe collision before Bat own next death Step");
                    let blood: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood")
                        .collect();
                    if bat.health == 1 {
                        assert!(blood.is_empty(), "no earlier Blood producer");
                        continue;
                    }
                    assert_eq!(bat.health, -1, "source descendant damage is2");
                    // Draw is captured after Collision and before Animation can
                    // change this mask or retire the last attack frame.
                    let draw = game
                        .draw_view
                        .as_deref()
                        .expect("actual Draw-phase snapshot");
                    let drawn_bat = draw.world.entity(target).unwrap();
                    let attack = draw
                        .world
                        .entities
                        .iter()
                        .find(|e| e.alive && e.object == hitbox)
                        .expect("ordinary attack must create the selected front descendant");
                    assert_eq!(drawn_bat.health, -1);
                    assert!(
                        attack.event_intersects(drawn_bat),
                        "actual precise automatic event contact"
                    );
                    assert_eq!(blood.len(), 1, "NATURAL_DESCENDANT_INHERITED_BLOOD");
                    assert_eq!(bat.value("blood_left"), 3.0, "source hp<0 exhaustion");
                    assert_eq!(bat.value("counts_as_kill"), 1.0);
                    assert_eq!(
                        bat.value("assaulted"),
                        0.0,
                        "base event has no custom assault write"
                    );
                    assert_eq!(
                        bat.value("killed_by_player"),
                        0.0,
                        "base event leaves own death to next Step"
                    );
                    let (velocity, gravity) = tape.blood();
                    assert_eq!(blood[0].pos, Vec2::new(bat.pos.x + 8.0, bat.pos.y + 8.0));
                    assert_eq!(blood[0].vel, velocity);
                    assert_eq!(blood[0].value("gravity"), gravity);
                    assert_eq!(blood[0].value("life"), 60.0);
                    assert_eq!(blood[0].alarms[..3], [1, 1, 5]);
                    assert_eq!(
                        game.rng.state_bits(),
                        tape.state,
                        "quiet full hit frame consumes exactly four Create draws"
                    );
                    hit = true;
                    break;
                }
                assert!(hit, "normal front weapon input must hit default flying Bat");
            }
        }
    }
}

#[test]
fn canonical_normal_front_descendants_reach_inherited_enemy_blood() {
    natural_front_case(false);
}

#[test]
fn legacy_normal_front_descendants_reach_inherited_enemy_blood() {
    natural_front_case(true);
}

#[test]
fn normal_descendant_swing_away_keeps_default_bat_healthy_and_blood_absent() {
    for legacy in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (weapon, hitbox) in [("oMachete", "oSlash"), ("oMattock", "oMattockHit")] {
                let (mut game, bat, _) = fixture(legacy, rules, weapon, 1);
                game.step(&InputFrame {
                    left: true,
                    ..Default::default()
                });
                let mut saw_front = false;
                for frame in 0..12 {
                    game.step(&InputFrame {
                        attack: frame == 0,
                        attack_pressed: frame == 0,
                        ..Default::default()
                    });
                    let target = game.world.entity(bat).unwrap();
                    assert_eq!(target.health, 1);
                    assert_eq!(game.player.health, 4);
                    assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
                    let draw = game
                        .draw_view
                        .as_deref()
                        .expect("actual Draw-phase snapshot");
                    let drawn_bat = draw.world.entity(bat).unwrap();
                    for attack in draw
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == hitbox)
                    {
                        saw_front = true;
                        assert!(!attack.event_intersects(drawn_bat));
                    }
                }
                assert!(
                    saw_front,
                    "negative control must perform a real front swing"
                );
            }
        }
    }
}
