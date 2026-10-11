//! Olmec own-Step continuation. Ordinary sink routes start with a fresh boss
//! above a contained authored pool and use only neutral public Game::step.
//! Head-only contact and attachment-query controls are explicitly authored.
//! Original GameMaker gameplay is not executed by these tests.
use spelunky_core::*;

fn shell(canonical: bool, rules: RulesPreset) -> Game {
    let mut g = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    g.world = World::new(30, 24, Area::Mines, 1);
    g.world.name = "rLevel".into();
    g.player.pos = Vec2::new(256.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.world.set_tile(16, 10, "oBlock");
    g
}
fn sink(canonical: bool, rules: RulesPreset) -> (Game, u64) {
    let mut g = shell(canonical, rules);
    for y in 10..=21 {
        g.world.set_tile(3, y, "oBlock");
        g.world.set_tile(9, y, "oBlock");
    }
    for x in 3..=9 {
        g.world.set_tile(x, 21, "oBlock");
    }
    for y in 10..21 {
        for x in 4..9 {
            g.spawn("oLava", Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
        }
    }
    let boss = g.spawn("oOlmec", Vec2::new(64.0, 64.0));
    let e = g.world.entity(boss).unwrap();
    assert_eq!(e.state, -1, "FRESH_CREATE_STATE");
    assert_eq!(e.vel, Vec2::default());
    assert!(e.health > 0);
    assert!(
        !g.world.entities.iter().any(|lava| lava.object == "oLava"
            && lava.mask_contains(Vec2::new(64.0, 128.0), CollisionPrecision::BoundingBox)),
        "FEET_START_DRY"
    );
    (g, boss)
}
fn neutral_sink_to_death(g: &mut Game, boss: u64) {
    let mut entered = false;
    for _ in 0..900 {
        let before = g.world.entity(boss).unwrap().clone();
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 4, "CONTROLLER_SURVIVES_ORDINARY_SINK");
        if let Some(e) = g.world.entity(boss) {
            if e.state == 4 {
                entered = true;
                assert_eq!(e.vel, Vec2::new(0.0, 0.1));
            }
        } else {
            assert!(
                entered && before.state == 4,
                "NATURAL_DROWNING_MUST_PRECEDE_HEAD_DEATH"
            );
            assert!(
                before.pos.y >= 159.0,
                "ORDINARY_DESCENT_REACHES_POOL_HEAD_CONTACT"
            );
            assert!(g.flags.contains("olmec_dead"));
            assert_eq!(g.data.get("kills"), Some(&1.0));
            assert_eq!(g.lifetime_stats.enemy_kills[21], 1);
            return;
        }
    }
    panic!("ORDINARY_FRESH_BOSS_SINK_MUST_QUALIFY");
}
fn flame_requests(g: &Game) -> usize {
    g.events
        .iter()
        .filter(|event| matches!(event, GameEvent::SoundIfIdle { name, .. } if name == "sndFlame"))
        .count()
}
fn natural_final_shake(canonical: bool, rules: RulesPreset) {
    let (mut g, boss) = sink(canonical, rules);
    neutral_sink_to_death(&mut g, boss);
    assert_eq!(
        g.data.get("shake"),
        Some(&10.0),
        "POST_DESTROY_DROWNING_SHAKE"
    );
}
fn natural_final_flame(canonical: bool, rules: RulesPreset) {
    let (mut g, boss) = sink(canonical, rules);
    neutral_sink_to_death(&mut g, boss);
    assert_eq!(flame_requests(&g), 1, "POST_DESTROY_DROWNING_SOUND_GATE");
}
fn natural_final_water_request(canonical: bool, rules: RulesPreset) {
    let (mut g, boss) = sink(canonical, rules);
    neutral_sink_to_death(&mut g, boss);
    assert!(
        g.world.fluid_dirty,
        "SYNCHRONOUS_INHERITED_SOLID_DESTROY_WATER_REQUEST"
    );
}
fn natural_sink_retires_attached_spikes(canonical: bool, rules: RulesPreset) {
    let (mut g, boss) = sink(canonical, rules);
    // Spikes has a three-pixel transparent top: y156 gives source bbox
    // top159, qualifying the ordinary final upper probe at boss y160.
    let spikes = g.spawn("oSpikes", Vec2::new(64.0, 156.0));
    neutral_sink_to_death(&mut g, boss);
    assert!(
        g.world.entity(spikes).is_none(),
        "SOURCE_SOLID_DESTROY_ATTACHMENT_BEFORE_TAIL"
    );
}
fn head_contact(canonical: bool, rules: RulesPreset) -> (Game, u64) {
    let mut g = shell(canonical, rules);
    // Authored inactive-player boss-view branch; retain its source camera pan
    // through the pre-Draw follow so that the continuation is observable.
    g.world.area = Area::Boss;
    g.player.data.insert("active".into(), 0.0);
    g.world.set_tile(5, 5, "oLava");
    let boss = g.spawn("oOlmec", Vec2::new(80.0, 96.0));
    assert_eq!(g.world.entity(boss).unwrap().state, -1);
    assert!(g.world.tiles_point(Vec2::new(80.0, 94.0), "oLava"));
    assert!(!g.world.tiles_point(Vec2::new(80.0, 160.0), "oLava"));
    (g, boss)
}
fn authored_head_only_retains_independent_start_camera(canonical: bool, rules: RulesPreset) {
    let (mut g, boss) = head_contact(canonical, rules);
    g.step(&InputFrame::default());
    assert!(g.world.entity(boss).is_none());
    assert_eq!(g.data.get("kills"), Some(&1.0));
    assert_eq!(flame_requests(&g), 0, "DRY_FEET_DO_NOT_INVENT_DROWNING");
    assert_eq!(g.camera.x, 2.0, "POST_DESTROY_START_CAMERA_BRANCH");
}
fn authored_first_hit_attachment(canonical: bool, rules: RulesPreset, family: &str) {
    let (mut g, boss) = head_contact(canonical, rules);
    if family == "oGrave" {
        // Grave Step runs after Olmec. Independent supports keep the unselected
        // grave alive after the boss retires, isolating the Destroy query.
        g.world.set_tile(5, 6, "oBlock");
        g.world.set_tile(6, 6, "oBlock");
    }
    let first = g.spawn(family, Vec2::new(96.0, 95.0));
    let gate_target = g.spawn(family, Vec2::new(82.0, 80.0));
    let gate = Vec2::new(88.0, 95.0);
    assert!(
        !g.world
            .entity(first)
            .unwrap()
            .mask_contains(gate, CollisionPrecision::BoundingBox)
    );
    assert!(
        g.world
            .entity(gate_target)
            .unwrap()
            .mask_contains(gate, CollisionPrecision::BoundingBox)
    );
    let mut translated = g.world.entity(boss).unwrap().clone();
    translated.pos = gate;
    assert!(
        translated.event_intersects(g.world.entity(first).unwrap()),
        "TRANSLATED_FULL_BOSS_MASK_REACHES_DIFFERENT_TARGET"
    );
    g.step(&InputFrame::default());
    assert!(g.world.entity(boss).is_none());
    assert!(
        g.world.entity(first).is_none(),
        "FIRST_QUERY_HIT_NOT_THE_GATE_HIT"
    );
    assert!(
        g.world.entity(gate_target).is_some(),
        "ONE_INSTANCE_PLACE_TARGET_ONLY"
    );
    if family == "oGrave" {
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                .count(),
            3,
            "SELECTED_GRAVE_SYNCHRONOUS_OWN_DESTROY"
        );
    }
}
fn authored_lamp_conversion_order(canonical: bool, rules: RulesPreset) {
    let (mut g, boss) = head_contact(canonical, rules);
    let mut selected = Vec::new();
    let mut gate_targets = Vec::new();
    for family in ["oLampRed", "oLamp"] {
        let first = g.spawn(family, Vec2::new(96.0, 114.0));
        let gate_target = g.spawn(family, Vec2::new(80.0, 112.0));
        let gate = Vec2::new(88.0, 114.0);
        assert!(
            !g.world
                .entity(first)
                .unwrap()
                .mask_contains(gate, CollisionPrecision::BoundingBox)
        );
        assert!(
            g.world
                .entity(gate_target)
                .unwrap()
                .mask_contains(gate, CollisionPrecision::BoundingBox)
        );
        selected.push(first);
        gate_targets.push(gate_target);
    }
    g.step(&InputFrame::default());
    assert!(g.world.entity(boss).is_none());
    assert!(
        g.world.entity(selected[0]).is_none(),
        "FIRST_RED_LAMP_RETIREMENT"
    );
    // oLampRed inherits oLamp. After the red branch removes its first target,
    // the ordinary Lamp query selects the remaining earlier red descendant.
    assert!(
        g.world.entity(gate_targets[0]).is_none(),
        "LAMP_PARENT_QUERY_SELECTS_REMAINING_RED_DESCENDANT"
    );
    assert!(
        g.world.entity(selected[1]).is_some(),
        "LATER_ORDINARY_LAMP_SURVIVES_PARENT_FIRST_HIT"
    );
    assert!(g.world.entity(gate_targets[1]).is_some());
    let red = g
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oLampRedItem")
        .expect("RED_LAMP_CREATE");
    let lamp = g
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oLampItem")
        .expect("LAMP_CREATE");
    assert_eq!(red.previous_pos, Vec2::new(104.0, 126.0));
    assert_eq!(lamp.previous_pos, Vec2::new(88.0, 124.0));
    assert!(
        red.creation_serial < lamp.creation_serial,
        "RED_BRANCH_COMPLETES_BEFORE_ORDINARY_LAMP"
    );
}
fn authored_mixed_storage_first_hit(canonical: bool, rules: RulesPreset) {
    let (mut g, boss) = head_contact(canonical, rules);
    g.world.set_tile(6, 6, "oSpikes");
    let gate_target = g.spawn("oSpikes", Vec2::new(84.0, 80.0));
    assert!(
        g.world
            .entity(gate_target)
            .unwrap()
            .mask_contains(Vec2::new(88.0, 95.0), CollisionPrecision::BoundingBox)
    );
    g.step(&InputFrame::default());
    assert!(g.world.entity(boss).is_none());
    assert_eq!(
        g.world.tile(6, 6).unwrap().object,
        "",
        "FIRST_TERRAIN_TARGET_RETIREMENT"
    );
    assert!(
        g.world.entity(gate_target).is_some(),
        "LATER_ENTITY_GATE_TARGET_SURVIVES"
    );
}
fn dry_fresh_boss_control(canonical: bool, rules: RulesPreset) {
    let mut g = shell(canonical, rules);
    g.world.area = Area::Boss;
    g.player.data.insert("active".into(), 0.0);
    let boss = g.spawn("oOlmec", Vec2::new(80.0, 96.0));
    let camera = g.camera.x;
    g.step(&InputFrame::default());
    assert!(g.world.entity(boss).is_some());
    assert!(!g.flags.contains("olmec_dead"));
    assert_eq!(g.data.get("kills"), None);
    assert_eq!(flame_requests(&g), 0);
    assert_eq!(g.camera.x, camera + 2.0);
    assert!(!g.world.fluid_dirty);
}
fn authored_spikes(canonical: bool, rules: RulesPreset) {
    authored_first_hit_attachment(canonical, rules, "oSpikes");
}
fn authored_torch(canonical: bool, rules: RulesPreset) {
    authored_first_hit_attachment(canonical, rules, "oTikiTorch");
}
fn authored_grave(canonical: bool, rules: RulesPreset) {
    authored_first_hit_attachment(canonical, rules, "oGrave");
}
fn fractional_lava_bbox(canonical: bool, rules: RulesPreset) {
    let mut g = shell(canonical, rules);
    let lava = g.spawn("oLava", Vec2::new(80.5, 80.0));
    let boss = g.spawn("oOlmec", Vec2::new(80.0, 96.0));
    assert!(
        g.world
            .entity(lava)
            .unwrap()
            .mask_contains(Vec2::new(80.0, 94.0), CollisionPrecision::BoundingBox),
        "SOURCE_ROUNDED_LAVA_BBOX_CONTACT"
    );
    assert!(
        !g.world
            .entity(lava)
            .unwrap()
            .bounds()
            .contains(Vec2::new(80.0, 94.0)),
        "CONTINUOUS_BODY_RECT_IS_DISTINCT"
    );
    g.step(&InputFrame::default());
    assert!(
        g.world.entity(boss).is_none(),
        "SOURCE_COLLISION_POINT_USES_ROUNDED_SPRITE_BBOX"
    );
    assert_eq!(g.data.get("kills"), Some(&1.0));
}

// Configured CREATE-state witnesses isolate the independent branch after
// head-only retirement. Error cases model the original dialog's Ignore choice.
fn configured_create(canonical: bool, rules: RulesPreset, mode: u8) {
    let (mut g, boss) = if mode == 2 {
        let mut g = shell(canonical, rules);
        g.world.area = Area::Boss;
        g.player.data.insert("active".into(), 0.0);
        let boss = g.spawn("oOlmec", Vec2::new(80.0, 96.0));
        (g, boss)
    } else {
        head_contact(canonical, rules)
    };
    g.world.entity_mut(boss).unwrap().state = 7;
    // Keep the surviving boss clear of the controller: an overlapping solid
    // independently damages the player and consumes Blood RNG before this Step.
    let survivor = (mode == 1).then(|| g.spawn("oOlmec", Vec2::new(300.0, 96.0)));
    if let Some(survivor) = survivor {
        assert!(
            !g.world
                .entity(survivor)
                .unwrap()
                .sprite_bounds()
                .intersects(g.player.sprite_bounds())
        );
    }
    let target = Vec2::new(if mode == 1 { 332.0 } else { 112.0 }, 112.0);
    let mut rng = g.rng.clone();
    let positions: Vec<_> = (0..6)
        .map(|_| {
            Vec2::new(
                112.0 + f64::from(rng.range(33)) - f64::from(rng.range(33)),
                110.0 + f64::from(rng.range(33)) - f64::from(rng.range(33)),
            )
        })
        .collect();
    let launches: Vec<_> = (0..3)
        .map(|_| {
            let vy = -4.0 - rng.unit() * 3.0;
            let mut vx = f64::from(2 + rng.range(4));
            if rng.chance(1, 2) {
                vx = -vx;
            }
            Vec2::new(vx, vy)
        })
        .collect();
    g.step(&InputFrame::default());
    assert_eq!(g.world.entity(boss).is_some(), mode == 2);
    if let Some(survivor) = survivor {
        assert!(g.world.entity(survivor).is_some());
    }
    let mut effects: Vec<_> = g
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oPsychicCreate2")
        .collect();
    let mut balls: Vec<_> = g
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oYellowBall")
        .collect();
    // World storage may reorder when a retired boss is swap-removed.
    // Source emission order is represented by creation serials.
    effects.sort_by_key(|e| e.creation_serial);
    balls.sort_by_key(|e| e.creation_serial);
    assert_eq!(
        effects.len(),
        6,
        "POST_DESTROY_CREATE_RETURNS_ALL_SIX_NEWBORN_IDS"
    );
    assert_eq!(
        balls.len(),
        3,
        "CHILD_CREATE_ERROR_DOES_NOT_ABORT_PARENT_REPEAT_TAIL"
    );
    let ids: Vec<_> = effects.iter().map(|e| e.id).collect();
    for (effect, pos) in effects.iter().zip(&positions) {
        assert_eq!(effect.previous_pos, *pos);
        assert!(effect.creation_serial < balls[0].creation_serial);
        if mode == 0 {
            assert_eq!(
                effect.value("psychic_missing_dir"),
                1.0,
                "MISSING_QUALIFIED_OWNER_MUST_NOT_INVENT_DIR"
            );
            assert_eq!(effect.vel, Vec2::default());
            assert_eq!(
                effect.pos, *pos,
                "FAILED_DIR_READ_ABORTS_CHILD_STEP_BEFORE_MOVEMENT"
            );
            assert_eq!(effect.value("y_acc"), 0.6);
            assert_eq!(effect.text.get("type").map(String::as_str), Some(""));
            assert_eq!(effect.data.get("blinkToggle"), Some(&0.0));
            assert_eq!(effect.image_speed(), 0.4);
        } else {
            let angle = (target.y - pos.y).atan2(target.x - pos.x);
            let velocity = Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin());
            let expected_dir = (-angle.to_degrees()).rem_euclid(360.0);
            let dir = *effect.data.get("dir").expect("source Create assigns dir");
            assert!((dir - expected_dir).abs() < 1e-12);
            assert_eq!(effect.data.get("y_vel"), Some(&0.0));
            // Source Step converts stored degrees back to radians. The adapter
            // uses f64; native x87 transcendental bits are not an oracle.
            assert!(
                (effect.vel.x - velocity.x).abs() < 1e-12
                    && (effect.vel.y - velocity.y).abs() < 1e-12,
                "QUALIFIED_GETTER_TARGETS_REMAINING_LIVE_ACTIVE_OWNER"
            );
            assert!(
                (effect.pos.x - pos.x - velocity.x).abs() < 1e-12
                    && (effect.pos.y - pos.y - velocity.y).abs() < 1e-12,
                "source psychic displacement after degree conversion"
            );
        }
    }
    for (ball, launch) in balls.iter().zip(launches) {
        assert_eq!(ball.previous_pos, Vec2::new(112.0, 112.0));
        assert_eq!(ball.pos, Vec2::new(112.0 + launch.x, 112.0 + launch.y));
        assert_eq!(ball.vel, Vec2::new(launch.x, launch.y + 0.15));
    }
    assert_eq!(
        g.events
            .iter()
            .filter(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndPsychic"))
            .count(),
        1
    );
    assert_eq!(
        g.rng.clone().next_u64(),
        rng.next_u64(),
        "POSITION_DRAWS_PRECEDE_THREE_BALL_CREATE_DRAWS"
    );
    if mode == 0 {
        assert_eq!(
            g.data.get("source_runtime_errors"),
            Some(&12.0),
            "SIX_CREATE_AND_SIX_LATER_STEP_ERRORS_UNDER_IGNORE"
        );
        for phase in ["Create:", "Step:"] {
            assert_eq!(g.events.iter().filter(|event|matches!(event,GameEvent::Message(message) if message.contains("SOURCE RUNTIME ERROR: oPsychicCreate2") && message.contains(phase))).count(),6);
        }
        g.step(&InputFrame::default());
        assert_eq!(g.data.get("source_runtime_errors"), Some(&18.0));
        for (id, pos) in ids.into_iter().zip(positions) {
            assert_eq!(
                g.world.entity(id).unwrap().pos,
                pos,
                "SUBSEQUENT_UNKNOWN_DIR_STEP_STAYS_STATIONARY"
            );
        }
        assert_eq!(g.data.get("kills"), Some(&1.0));
    } else {
        assert_eq!(g.data.get("source_runtime_errors"), None);
    }
}
fn configured_missing_owner(canonical: bool, rules: RulesPreset) {
    configured_create(canonical, rules, 0);
}
fn configured_remaining_owner(canonical: bool, rules: RulesPreset) {
    configured_create(canonical, rules, 1);
}
fn positive_live_create(canonical: bool, rules: RulesPreset) {
    configured_create(canonical, rules, 2);
}

macro_rules! cases {
    ($module:ident, $case:ident) => {
        mod $module {
            use super::*;
            #[test]
            fn legacy_classic() {
                $case(false, RulesPreset::Classic);
            }
            #[test]
            fn legacy_assisted() {
                $case(false, RulesPreset::Assisted);
            }
            #[test]
            fn canonical_classic() {
                $case(true, RulesPreset::Classic);
            }
            #[test]
            fn canonical_assisted() {
                $case(true, RulesPreset::Assisted);
            }
        }
    };
}
cases!(natural_shake, natural_final_shake);
cases!(natural_flame, natural_final_flame);
cases!(natural_water, natural_final_water_request);
cases!(natural_attachment, natural_sink_retires_attached_spikes);
cases!(
    authored_start_tail,
    authored_head_only_retains_independent_start_camera
);
cases!(authored_spikes_query, authored_spikes);
cases!(authored_torch_query, authored_torch);
cases!(authored_grave_query, authored_grave);
cases!(fractional_lava_query, fractional_lava_bbox);
cases!(authored_lamps, authored_lamp_conversion_order);
cases!(authored_mixed_storage, authored_mixed_storage_first_hit);
cases!(positive_dry, dry_fresh_boss_control);

cases!(configured_missing_owner_create, configured_missing_owner);
cases!(
    configured_remaining_owner_create,
    configured_remaining_owner
);
cases!(positive_live_create_branch, positive_live_create);
