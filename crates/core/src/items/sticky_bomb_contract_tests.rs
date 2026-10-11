//! Authored rooms, default actors and ordinary input only. These source-oracle
//! predictions are prepared without executing Rust or changing state mid-run.
use crate::{Area, Game, InputFrame, RulesPreset, Vec2, World};

fn room(rules: RulesPreset, paste: bool) -> Game {
    let mut g = Game::shell(42, rules);
    g.world = World::new(30, 20, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..30 {
        g.world.set_tile(x, 7, "oHardBlock");
    }
    g.player.pos = Vec2::new(80.0, 104.0);
    g.player.previous_pos = g.player.pos;
    g.player.grounded = true;
    // Source-obtainable initial equipment; no attachment/velocity/timer is set.
    if paste {
        g.player.equipment.insert("Paste".into());
    }
    g
}

fn throw_bomb(g: &mut Game) -> u64 {
    let old = g.world.next_id;
    g.step(&InputFrame {
        bomb_pressed: true,
        ..Default::default()
    });
    g.world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oBomb" && e.id >= old)
        .expect("ordinary bomb input must create the source armed bomb")
        .id
}

fn gate(g: &Game, id: u64, point: Vec2) -> bool {
    g.world
        .entity(id)
        .unwrap()
        .collision_mask()
        .intersects_rectangle_bbox(
            Vec2::new(point.x - 2.0, point.y - 2.0),
            Vec2::new(point.x + 2.0, point.y + 2.0),
        )
}

#[test]
fn actual_bomb_input_enemy_contact_selects_nearest_all_enemy_before_damsel() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for nearer_noncontact in [false, true] {
            let mut g = room(rules, true);
            // Insertion order intentionally favors the later source fallback.
            let damsel = g.spawn("oDamsel", Vec2::new(88.0, 101.0));
            let touching = g.spawn("oGiantSpider", Vec2::new(88.0, 72.0));
            let nearest = nearer_noncontact.then(|| g.spawn("oBat", Vec2::new(94.0, 88.0)));
            let bomb = throw_bomb(&mut g);
            let b = g.world.entity(bomb).unwrap();
            let point = b.pos;
            assert_eq!(point, Vec2::new(88.0, 101.0));
            assert!(
                gate(&g, touching, point),
                "the independent Enemy rectangle gate must hit"
            );
            assert!(gate(&g, damsel, point), "Damsel fallback also overlaps");
            if let Some(id) = nearest {
                assert!(!gate(&g, id, point), "nearest is outside the rectangle");
                assert!(
                    g.world.entity(id).unwrap().pos.distance(point)
                        < g.world.entity(touching).unwrap().pos.distance(point)
                );
            }
            let expected = nearest.unwrap_or(touching);
            assert_eq!(b.value("attached_id"), expected as f64);
            assert_eq!(
                g.world.entity(expected).unwrap().value("bomb_id"),
                bomb as f64
            );
            assert_eq!(g.world.entity(damsel).unwrap().value("bomb_id"), 0.0);
            assert_eq!(g.player.health, 4);
        }
    }
}

#[test]
fn actual_bomb_input_damsel_fallback_requires_contact_and_paste() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for paste in [false, true] {
            let mut g = room(rules, paste);
            let damsel = g.spawn("oDamsel", Vec2::new(88.0, 101.0));
            let bomb = throw_bomb(&mut g);
            assert_eq!(
                g.world.entity(bomb).unwrap().value("attached_id"),
                if paste { damsel as f64 } else { 0.0 }
            );
            assert_eq!(
                g.world.entity(damsel).unwrap().value("bomb_id"),
                if paste { bomb as f64 } else { 0.0 }
            );
        }
        let mut g = room(rules, true);
        g.spawn("oBat", Vec2::new(94.0, 88.0));
        let bomb = throw_bomb(&mut g);
        assert_eq!(
            g.world.entity(bomb).unwrap().value("attached_id"),
            0.0,
            "nearest Enemy alone does not replace the rectangle contact gate"
        );
    }
}

#[test]
fn actual_two_bomb_inputs_keep_both_attached_and_older_alarm_clears_latest_backlink() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules, true);
        // Authored side support keeps the ordinary Left escape on the floor
        // while both natural fuses run; source movement has no room-edge clamp.
        for y in 0..7 {
            g.world.set_tile(0, y, "oHardBlock");
        }
        // Ceiling holds the default Bat in HANG: player.y < bat.y+16.
        // Its center at y102 is below the ceiling, so the source crush probe is clear.
        g.world.set_tile(5, 5, "oHardBlock");
        // Bat center separation13 exceeds the source damage gate of12.
        let receiver = g.spawn("oBat", Vec2::new(85.0, 94.0));
        let first = throw_bomb(&mut g);
        let second = throw_bomb(&mut g);
        assert_eq!(
            g.world.entity(first).unwrap().value("attached_id"),
            receiver as f64
        );
        assert_eq!(
            g.world.entity(second).unwrap().value("attached_id"),
            receiver as f64
        );
        assert_eq!(
            g.world.entity(receiver).unwrap().value("bomb_id"),
            second as f64
        );
        for _ in 0..140 {
            if g.world.entity(first).is_none() {
                break;
            }
            g.step(&InputFrame {
                left: true,
                ..Default::default()
            });
        }
        assert!(
            g.world.entity(first).is_none(),
            "first bomb must reach its natural Alarm1"
        );
        assert!(
            g.world.entity(second).is_some(),
            "second bomb has a later natural fuse"
        );
        let receiver = g
            .world
            .entity(receiver)
            .expect("Bat Step precedes blast damage; death is next Step");
        assert_eq!(
            receiver.value("bomb_id"),
            0.0,
            "older Bomb Destroy clears bombID unconditionally, despite newer backlink"
        );
        assert_eq!(g.player.health, 4);
    }
}

#[test]
fn actual_pistol_pickup_and_attack_destroys_attached_receiver_and_releases_bombs() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules, true);
        g.world.set_tile(5, 5, "oHardBlock");
        // Duck right-mirror maxX87 stays clear of BatHang minX88.
        let receiver = g.spawn("oBat", Vec2::new(85.0, 94.0));
        let pistol = g.spawn("oPistol", Vec2::new(80.0, 108.0));
        g.step(&InputFrame::default());
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            attack: true,
            ..Default::default()
        });
        assert_eq!(
            g.player.held,
            Some(pistol),
            "ordinary pickup is a fixture prerequisite"
        );
        g.step(&InputFrame::default());
        let first = throw_bomb(&mut g);
        let second = throw_bomb(&mut g);
        assert_eq!(
            g.world.entity(receiver).unwrap().value("bomb_id"),
            second as f64
        );
        assert_eq!(
            g.world.entity(first).unwrap().value("attached_id"),
            receiver as f64
        );
        assert_eq!(
            g.world.entity(second).unwrap().value("attached_id"),
            receiver as f64
        );
        g.step(&InputFrame {
            attack_pressed: true,
            attack: true,
            ..Default::default()
        });
        for _ in 0..4 {
            if g.world.entity(receiver).is_none() {
                break;
            }
            g.step(&InputFrame::default());
        }
        assert!(
            g.world.entity(receiver).is_none(),
            "ordinary Pistol bullet must cause natural Bat Destroy"
        );
        for bomb in [first, second] {
            assert_eq!(g.world.entity(bomb).unwrap().value("attached_id"), 0.0);
        }
        assert_eq!(g.player.health, 4);
    }
}

#[test]
fn actual_attached_bomb_keeps_parent_step_velocity_before_end_step_follow() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules, true);
        g.spawn("oGiantSpider", Vec2::new(88.0, 72.0));
        let target = g.spawn("oBat", Vec2::new(94.0, 88.0));
        let bomb = throw_bomb(&mut g);
        let b = g.world.entity(bomb).unwrap();
        assert_eq!(b.value("attached_id"), target as f64);
        assert_eq!(b.vel.x, 8.0);
        let offset = Vec2::new(b.value("sticky_x"), b.value("sticky_y"));
        let before_vy = b.vel.y;
        g.step(&InputFrame::default());
        let b = g.world.entity(bomb).unwrap();
        assert_eq!(b.value("attached_id"), target as f64);
        assert_eq!(
            b.vel.x, 8.0,
            "parent oItem motion does not zero attached velocity"
        );
        assert!(
            (b.vel.y - (before_vy + 0.6)).abs() < 1e-12,
            "parent gravity continues before the Bomb End Step override"
        );
        let target = g.world.entity(target).unwrap();
        assert_eq!(
            b.pos,
            Vec2::new(target.pos.x - offset.x, target.pos.y - offset.y)
        );
    }
}

#[test]
fn actual_teleporter_pickup_and_activation_destroy_sticky_receiver() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules, true);
        // Portable-RNG seed0 maps inclusive rand(4,8) to4: a64px right teleport.
        // This does not claim original GM8 seedstream equivalence.
        g.seed = 0;
        g.rng = crate::Rng::new(0);
        g.world.set_tile(5, 5, "oHardBlock");
        // Duck right-mirror maxX87 stays clear of BatHang minX88.
        let receiver = g.spawn("oBat", Vec2::new(85.0, 94.0));
        let teleporter = g.spawn("oTeleporter", Vec2::new(80.0, 108.0));
        g.step(&InputFrame::default());
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            attack: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(teleporter));
        g.step(&InputFrame::default());
        let bomb = throw_bomb(&mut g);
        assert_eq!(
            g.world.entity(receiver).unwrap().value("bomb_id"),
            bomb as f64
        );
        assert_eq!(
            g.world.entity(bomb).unwrap().value("attached_id"),
            receiver as f64
        );
        // Walk away from the hanging receiver, then use the ordinary Teleporter
        // input to return into its sprite. No actor/timer/velocity is injected.
        for _ in 0..60 {
            if g.player.pos.x <= 24.0 {
                break;
            }
            g.step(&InputFrame {
                left: true,
                ..Default::default()
            });
        }
        assert!(g.player.pos.x <= 24.0);
        assert_eq!(g.player.health, 4);
        assert!(g.world.entity(receiver).is_some());
        assert!(
            g.world.entity(bomb).is_some(),
            "activation precedes the natural fuse"
        );
        assert_eq!(
            (g.rng.clone().next_u64() * 5) >> 32,
            0,
            "source first teleport is64px"
        );
        let activate = InputFrame {
            right: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        };
        // A supplementary callback-context witness stops before Bomb Step and
        // End Step. Setup above used only real Game.step inputs. End Step alone
        // clears stale IDs, so final-frame assertions cannot prove this timing.
        let mut callback_probe = g.clone();
        crate::player::tick(&mut callback_probe, &activate);
        crate::items::player_step(&mut callback_probe, &activate);
        let retired = callback_probe
            .world
            .entities
            .iter()
            .find(|e| e.id == receiver)
            .expect("raw receiver tombstone is retained before engine cleanup");
        assert!(!retired.alive);
        assert_eq!(
            retired.value("sticky_destroyed"),
            1.0,
            "receiver Destroy must finish inside the Teleporter caller"
        );
        assert_eq!(
            callback_probe
                .world
                .entity(bomb)
                .unwrap()
                .value("attached_id"),
            0.0,
            "backlink must clear before any Bomb Step or End Step can mask the timing"
        );
        assert_eq!(
            g.world.entity(bomb).unwrap().value("attached_id"),
            receiver as f64
        );
        g.step(&activate);
        assert!((84.0..=90.0).contains(&g.player.pos.x));
        assert_eq!(g.player.pos.y, 104.0);
        assert!(
            g.world.entity(receiver).is_none(),
            "normal Teleporter activation telefrags the Bat"
        );
        assert_eq!(g.world.entity(bomb).unwrap().value("attached_id"), 0.0);
        assert_eq!(g.player.health, 4);
    }
}
