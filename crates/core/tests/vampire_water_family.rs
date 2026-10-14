//! Authored room, default native Vampire and neutral inputs only. No generated
//! campaign or original executable trajectory is claimed. Every precondition
//! precedes the source-effect assertion; a failed precondition is not bug proof.
use spelunky_core::*;

#[derive(Clone)]
struct LiteralRng {
    state: u64,
    canonical: bool,
}
impl LiteralRng {
    fn range(&mut self, n: u32) -> u32 {
        if self.canonical {
            self.state = u64::from((self.state as u32).wrapping_mul(0x08088405).wrapping_add(1));
            ((self.state * u64::from(n)) >> 32) as u32
        } else {
            self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            ((z ^ (z >> 31)) % u64::from(n)) as u32
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum Pit {
    Lava,
    Water,
    Dry,
}
fn fluid_at(game: &Game, point: Vec2, family: &str) -> bool {
    game.world.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && catalog::is_a(&tile.object, family)
            && game
                .world
                .tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
                .contains(point, CollisionPrecision::BoundingBox)
    }) || game.world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && catalog::is_a(&e.object, family)
            && e.mask_contains(point, CollisionPrecision::BoundingBox)
    })
}
fn approximately(a: f64, b: f64, why: &str) {
    assert!((a - b).abs() < 1e-12, "{why}: {a} versus {b}");
}
fn velocity(e: &Entity, x: f64, y: f64) {
    approximately(e.vel.x, x, "literal source horizontal velocity/friction");
    approximately(e.vel.y, y, "literal source gravity/grounding");
}
fn healthy(game: &Game, id: u64, pit: Pit) {
    let e = game
        .world
        .entity(id)
        .expect("healthy source receiver exists");
    assert!(e.alive && e.engine_active && !e.held);
    assert_eq!(e.object, "oVampire");
    assert_eq!(e.health, 6);
    assert_eq!(e.value("swimming"), 0.0);
    assert_eq!(e.value("burning"), 0.0);
    assert_eq!(e.value("invincible"), 0.0);
    assert_eq!(e.value("bloodless"), 1.0);
    assert_eq!(e.timer, 0);
    assert_eq!(e.facing, 1);
    assert_eq!(e.data.get("image_xscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(e.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(catalog::sprite_dimensions(&e.sprite), Some((16, 16)));
    let v = game.simulation_view_rect();
    assert!(
        e.pos.x > v.x - 20.0
            && e.pos.x < v.x + v.w + 4.0
            && e.pos.y > v.y - 20.0
            && e.pos.y < v.y + v.h + 4.0
    );
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.pos, Vec2::new(184.0, 152.0));
    assert!(game.player.engine_active && game.player.held.is_none());
    assert_eq!(
        game.player.data.get("swimming").copied().unwrap_or(0.0),
        0.0
    );
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(!fluid_at(game, game.player.pos, "oWater"));
    assert!(!e.sprite_bounds().intersects(game.player.sprite_bounds()));
    assert!(
        e.pos.x + 32.0 < game.player.pos.x - 8.0,
        "disjoint stock canvas envelopes"
    );
    assert!(!game.world.fluid_dirty);
    for e in game.world.entities.iter().filter(|e| e.object == "oLava") {
        assert_eq!(e.value("spurt"), 0.0);
        assert!(e.value("spurt_counter") >= 100.0);
        assert_eq!(e.sprite, "sLava");
        assert_eq!(e.vel, Vec2::default());
    }
    let expected = if matches!(pit, Pit::Dry) { 1 } else { 4 };
    assert_eq!(
        game.world.entities.len(),
        expected,
        "only native actor and declared fluid births"
    );
    assert!(!game.events.iter().any(|event| matches!(event, GameEvent::Damage { .. })
        || matches!(event, GameEvent::Sound { name, .. } if name == "sndSplash" || name == "sndHit")));
    for counter in ["kills", "vampires"] {
        assert_eq!(game.data.get(counter).copied().unwrap_or(0.0), 0.0);
    }
}
fn run_case(rules: RulesPreset, canonical: bool, pit: Pit) {
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.world = World::new(24, 18, Area::Temple, 13);
    game.world.name = "rLevel".into();
    // Native hard-block floor ends at x128, leaving a three-cell pit. The
    // Player's separate supported floor starts at x176, beyond all contact.
    for x in (0..8).chain(11..24) {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // Initial authored player placement; retain the shell's native Player.
    game.player.pos = Vec2::new(184.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.vel = Vec2::default();
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game.camera = Vec2::new(24.0, 32.0);
    game.previous_camera = game.camera;
    let mut rng = LiteralRng {
        state: 42,
        canonical,
    };
    assert_eq!(game.rng.state_bits(), rng.state);
    for x in 8..11 {
        let object = match pit {
            Pit::Lava => Some("oLava"),
            Pit::Water => Some("oWaterSwim"),
            Pit::Dry => None,
        };
        if let Some(object) = object {
            let id = game.spawn(object, Vec2::new((x * 16) as f64, 160.0));
            let e = game.world.entity(id).unwrap();
            assert_eq!(e.pos, Vec2::new((x * 16) as f64, 160.0));
            if matches!(pit, Pit::Lava) {
                let time = f64::from(100 + rng.range(201));
                assert_eq!(e.value("spurt_time"), time);
                assert_eq!(e.value("spurt_counter"), time);
                assert_eq!(e.value("spurt"), 0.0);
            }
        }
    }
    assert_eq!(
        game.rng.state_bits(),
        rng.state,
        "exact native fluid Create prefix"
    );
    let id = game.spawn("oVampire", Vec2::new(112.0, 144.0));
    let born = game.world.entity(id).unwrap();
    assert_eq!(
        (born.state, born.timer, born.health, born.facing),
        (0, 0, 6, 1)
    );
    assert_eq!(born.vel, Vec2::new(2.5, 0.0));
    assert_eq!(born.sprite, "sVampireLeft");
    assert_eq!(born.image_speed(), 0.5);
    assert_eq!(game.rng.state_bits(), rng.state);
    // Source xVel is stored binary64: 2.5-.1 is 2.4, whose fraction is
    // .3999999999999999, so round(1/frac)=3. Clock3 moves3, not2. By clock5
    // the BOUNCE branch is already unsupported and selects RECOVER without
    // rand(3,6). No scene/outcome/seed search or runtime field write occurs.
    for (i, (x, state, vx, vy)) in [
        (115.0, 2, 2.4, 0.0),
        (118.0, 2, 3.9, 0.0),
        (122.0, 5, 3.9, 0.0),
        (126.0, 4, 3.8, 0.2),
    ]
    .into_iter()
    .enumerate()
    {
        healthy(&game, id, pit);
        if i == 0 {
            rng.range(2);
        } // source counter0 IDLE facing, then near Player overrides it
        game.step(&InputFrame::default());
        healthy(&game, id, pit);
        let e = game.world.entity(id).unwrap();
        assert_eq!(game.engine_time(), (i + 2) as u64);
        assert_eq!(
            e.pos,
            Vec2::new(x, 144.0),
            "literal moveTo clock/edge chronology"
        );
        assert_eq!(
            e.state, state,
            "native IDLE -> ATTACK -> ledge BOUNCE -> airborne RECOVER"
        );
        velocity(e, vx, vy);
        assert_eq!(
            game.rng.state_bits(),
            rng.state,
            "no supported bounce/random producer"
        );
    }
    let before = game.world.entity(id).unwrap().clone();
    assert_eq!((before.state, before.pos), (4, Vec2::new(126.0, 144.0)));
    velocity(&before, 3.8, 0.2);
    assert_eq!(before.sprite, "sVampireJumpL");
    // Parent probes run before moveTo; own RECOVER uses the post-move130,144.
    let post = Vec2::new(130.0, 144.0);
    for p in [
        Vec2::new(before.pos.x + 8.0, before.pos.y - 1.0),
        Vec2::new(before.pos.x + 8.0, before.pos.y + 14.0),
    ] {
        assert!(
            !fluid_at(&game, p, "oLava"),
            "parent Lava does not retire/sink this live receiver"
        );
    }
    assert!(!fluid_at(
        &game,
        Vec2::new(before.pos.x + 8.0, before.pos.y + 8.0),
        "oWaterSwim"
    ));
    assert!(
        !fluid_at(&game, Vec2::new(post.x + 8.0, post.y + 8.0), "oWater"),
        "own center8 dry"
    );
    assert!(
        !game
            .world
            .solid_line_except(Vec2::new(144.0, 144.0), Vec2::new(144.0, 159.0), id),
        "facingRight side guard clear"
    );
    assert!(
        !game
            .world
            .solid_line_except(Vec2::new(132.0, 160.0), Vec2::new(143.0, 160.0), id),
        "postmove full bottom unsupported"
    );
    assert!(
        !game
            .world
            .solid_point_except(Vec2::new(post.x + 8.0, post.y + 8.0), id),
        "no own crush"
    );
    let lower = Vec2::new(post.x + 8.0, post.y + 24.0);
    assert_eq!(lower, Vec2::new(138.0, 168.0));
    let wet = !matches!(pit, Pit::Dry);
    assert_eq!(
        fluid_at(&game, lower, "oWater"),
        wet,
        "literal Water-family native bbox at lower24"
    );
    assert_eq!(fluid_at(&game, lower, "oLava"), matches!(pit, Pit::Lava));
    assert_eq!(
        fluid_at(&game, lower, "oWaterSwim"),
        matches!(pit, Pit::Water)
    );
    let chance = 1 + rng.range(100);
    assert_ne!(
        chance, 1,
        "random FLY alternative must not qualify this witness"
    );
    println!(
        "QUALIFIED_VAMPIRE_WATER rules={rules:?} canonical={canonical} pit={pit:?} pre={:?} post={post:?} lower={lower:?} chance={chance} sourceFamily={wet}",
        before.pos
    );
    game.step(&InputFrame::default());
    healthy(&game, id, pit);
    let after = game.world.entity(id).unwrap();
    assert_eq!(
        after.pos, post,
        "actual postmove pose matches source branch guard geometry"
    );
    assert_eq!(game.engine_time(), 6);
    assert_eq!(
        game.rng.state_bits(),
        rng.state,
        "exact single RECOVER rand(1,100)"
    );
    velocity(after, 3.7, 0.4);
    assert_eq!(
        after.state,
        if wet { 7 } else { 4 },
        "SOURCE_VAMPIRE_WATER_FAMILY_LOWER24_SELECTS_FLY"
    );
    assert_eq!(
        after.sprite,
        if wet {
            "sVampireBatLeft"
        } else {
            "sVampireJumpL"
        }
    );
    assert_eq!(after.image_speed(), 1.0);
    let drawn = game.presentation();
    let visible = drawn.world.entity(id).unwrap();
    assert_eq!(visible.state, after.state);
    assert_eq!(visible.sprite, after.sprite);
}
#[test]
fn default_vampire_ledge_recovery_detects_native_lava_as_water_family() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, Pit::Lava);
        }
    }
}
#[test]
fn default_vampire_native_water_recovery_agrees_on_same_ledge_route() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, Pit::Water);
        }
    }
}
#[test]
fn default_vampire_dry_ledge_recovery_keeps_recover_after_false_random_gate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, Pit::Dry);
        }
    }
}
