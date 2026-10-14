//! FakeBones rises through ordinary neutral Steps before birthing Skeleton.
//! Initial floor/actor/Player geometry is authored; no gameplay state writes.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, RunState, Vec2, World, enemies};

fn first_birth(rules: RulesPreset, player_x: f64, expected_facing: i8) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    for x in 0..20 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // Retain source-default HP4, velocity0, Falling, equipment and inventory.
    // The first real controller Step establishes grounding from this floor.
    game.player.pos = Vec2::new(player_x, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    let bones = game.spawn("oFakeBones", Vec2::new(112.0, 144.0));
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.world.entity(bones).unwrap().sprite, "sFakeBonesLeft");
    assert_eq!(game.world.entity(bones).unwrap().animation, 0.0);
    assert_eq!(game.world.entity(bones).unwrap().vel, Vec2::default());
    assert!(game.world.solid_at(120.0, 160.0));
    assert!(!game.world.solid_at(120.0, 159.0));
    assert!((player_x - 120.0).abs() < 64.0);
    assert!(game.player.engine_active);
    assert_eq!(expected_facing, if player_x < 120.0 { -1 } else { 1 });
    let mut child = None;
    let mut saw_rise = false;
    for _ in 0..32 {
        let before_rng = game.rng.state_bits();
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.invincible, 0);
        assert_eq!(game.player.stun, 0);
        assert!(!game.player.is_stunned());
        assert_eq!(game.player.pos, Vec2::new(player_x, 152.0));
        assert_eq!(game.player.vel, Vec2::default());
        assert!(game.player.grounded && game.player.engine_active);
        assert_eq!(
            game.rng.state_bits(),
            before_rng,
            "quiet room, FakeBones and Skeleton Create consume no RNG"
        );
        let children = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oSkeleton")
            .collect::<Vec<_>>();
        if let Some(bones) = game.world.entity(bones) {
            assert!(
                children.is_empty(),
                "no child before producer Animation End"
            );
            assert_eq!(bones.pos, Vec2::new(112.0, 144.0));
            assert_eq!(bones.sprite, "sSkeletonCreateL");
            saw_rise = true;
            continue;
        }
        assert!(
            saw_rise,
            "SETUP: ordinary proximity must start the real rise animation"
        );
        assert_eq!(children.len(), 1);
        let skeleton = children[0];
        assert!(skeleton.engine_active && !skeleton.held);
        assert_eq!(skeleton.pos, Vec2::new(112.0, 144.0));
        assert_eq!(skeleton.health, 1);
        assert_eq!(skeleton.state, 0);
        assert_eq!(skeleton.timer, 20);
        assert_eq!(skeleton.age, 0, "birth is after completed own-Step phase");
        assert_eq!(skeleton.vel, Vec2::default());
        assert_eq!(skeleton.sprite, "sSkeletonCreateL");
        assert_eq!(skeleton.image_speed(), 0.5);
        assert_eq!(skeleton.value("collision_left"), 2.0);
        assert_eq!(skeleton.value("collision_top"), 0.0);
        assert_eq!(skeleton.value("collision_right"), 14.0);
        assert_eq!(skeleton.value("collision_bottom"), 16.0);
        eprintln!(
            "QUALIFIED_FIRST_SKELETON {rules:?} tick={} player_x={player_x} child={} source_facing={}",
            game.tick,
            skeleton.id,
            enemies::source_facing(skeleton)
        );
        assert_eq!(
            enemies::source_facing(skeleton),
            expected_facing,
            "SOURCE_SYNCHRONOUS_CREATE: Skeleton faces the existing Player at first birth"
        );
        assert_eq!(
            skeleton.value("source_facing_pending"),
            0.0,
            "context Create must finish before producer self-Destroy returns"
        );
        assert_eq!(skeleton.value("enemy_context_created"), 1.0);
        child = Some(skeleton.id);
        break;
    }
    let child = child.expect("SETUP: default-speed Skeleton rise must finish in32 neutral Steps");
    // Complete one ordinary subsequent Step, so early Create must not cause a
    // repeated constructor or preserve Create's IDLE counter at20 forever.
    let before_rng = game.rng.state_bits();
    game.step(&InputFrame::default());
    let skeleton = game.world.entity(child).unwrap();
    assert_eq!(skeleton.timer, 19);
    assert_eq!(enemies::source_facing(skeleton), expected_facing);
    assert_eq!(skeleton.value("source_facing_pending"), 0.0);
    assert_eq!(game.rng.state_bits(), before_rng);
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oSkeleton")
            .count(),
        1
    );
    assert!(game.world.entity(bones).is_none());
}

#[test]
fn ordinary_fake_bones_rise_completes_left_facing_skeleton_create_in_birth_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        first_birth(rules, 88.0, -1);
    }
}
#[test]
fn ordinary_fake_bones_rise_preserves_right_facing_create_and_single_birth() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        first_birth(rules, 152.0, 1);
    }
}
