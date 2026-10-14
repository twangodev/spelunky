//! Explicit initial physical shop configuration, then ordinary inputs only.
//! Source seed 0 chooses a chest trap and horizontal speed -3 on its first
//! three LCG draws. No seed search, actor arming, timer writes or private calls.
use spelunky_core::*;

const ORIGIN: Vec2 = Vec2::new(136.0, 152.0);

fn scene(rules: RulesPreset, facing: i8) -> (Game, u64, u64) {
    let mut game = Game::shell(0, rules);
    game.world.name = "rLevel".into();
    game.player.pos = ORIGIN;
    game.player.previous_pos = ORIGIN;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = ORIGIN;
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // Authored initial source-global shop membership, not generated-shop proof.
    let mut path = [[0; 4]; 5];
    path[1][0] = 4;
    game.world.source_room_path = Some(path);
    // Keeper stays to the right of the seed-0 trap's leftward travel.
    let keeper = game.spawn("oShopkeeper", Vec2::new(152.0, 144.0));
    let chest = game.spawn("oChest", ORIGIN);
    assert_eq!(game.rng.state_bits(), 0, "native setup consumes no RNG");
    (game, keeper, chest)
}

fn no_prior_warning(game: &Game, keeper: u64) {
    assert_eq!(game.player.health, 4);
    assert_eq!(game.state, RunState::Playing);
    assert!(!game.flags.contains("wanted"), "warning before the action");
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
    let actor = game.world.entity(keeper).expect("real keeper survives");
    assert!(actor.engine_active);
    assert_eq!(
        actor.health, 20,
        "no Item/Explosion damage establishes anger"
    );
    assert_eq!(actor.state, 0);
    assert_eq!(actor.value("angered"), 0.0);
    assert_eq!(actor.value("dead"), 0.0);
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oExplosion")
    );
}

fn source_trap(game: &mut Game, keeper: u64, chest: u64) -> u64 {
    assert_eq!(catalog::object("oPlayer1").unwrap().id, 319);
    assert_eq!(catalog::object("oBomb").unwrap().id, 454);
    let mut oracle = Rng::new(0);
    assert_eq!(oracle.range(12), 0);
    assert_eq!(oracle.range(4), 0);
    assert_eq!(oracle.range(4), 3);
    game.step(&InputFrame {
        up: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    no_prior_warning(game, keeper);
    assert_eq!(game.world.entity(chest).unwrap().sprite, "sChestOpen");
    let bombs: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBomb")
        .collect();
    assert_eq!(bombs.len(), 1, "FIXTURE_NATIVE_TRAP_BRANCH");
    let bomb = bombs[0];
    assert_eq!(bomb.value("armed"), 0.0);
    assert_eq!(bomb.sprite, "sBombArmed");
    assert_eq!(bomb.pos, Vec2::new(133.0, 150.0));
    assert_eq!(bomb.vel, Vec2::new(-3.0, -2.0 + 0.6));
    assert_eq!(bomb.alarms[0], -1);
    assert_eq!(bomb.alarms[1], 40);
    assert!(bomb.engine_active);
    bomb.id
}

fn pickup_on_fixed_tape(game: &mut Game, keeper: u64, bomb: u64) {
    // Frames 2–14: thirteen held-Left frames. The first releases the opening
    // attack; Classic's two-frame acceleration delay also applies in Assisted.
    for frame in 2..=14 {
        game.step(&InputFrame {
            left: true,
            attack_released: frame == 2,
            ..Default::default()
        });
        no_prior_warning(game, keeper);
        let actor = game.world.entity(bomb).expect("trap fuse still running");
        assert_eq!(actor.value("armed"), 0.0);
        assert_eq!(actor.alarms[1], 41 - frame);
        assert_eq!(game.player.held, None);
    }
    // Frame 15: ordinary direction release and Down braking, without Attack.
    // engine_time = frame+1: frame16 uses time17, so the next -0.12
    // subpixel velocity does not move the player on the pickup frame.
    game.step(&InputFrame {
        down: true,
        left_released: true,
        ..Default::default()
    });
    no_prior_warning(game, keeper);
    assert_eq!(
        game.player.pos,
        Vec2::new(104.0, 152.0),
        "FIXTURE_SOURCE_WALK_TAPE"
    );
    assert_eq!(game.player.state, PlayerState::Ducking);
    assert_eq!(game.world.entity(bomb).unwrap().alarms[1], 26);
    // Frame 16: actual pickup. The descending/bounced Bomb is near the floor,
    // while the open Chest remains thirty pixels away. Do not select an outcome
    // or mutate either actor if this prerequisite fails in future execution.
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    no_prior_warning(game, keeper);
    assert_eq!(
        game.player.held,
        Some(bomb),
        "FIXTURE_REAL_TRAP_BOMB_PICKUP"
    );
    assert_eq!(game.player.pickup_item_type, "Bomb");
    let actor = game.world.entity(bomb).unwrap();
    assert!(actor.held && actor.engine_active);
    assert_eq!(actor.value("armed"), 0.0);
    assert_eq!(actor.sprite, "sBombArmed");
    assert_eq!(actor.alarms[1], 25);
    assert_eq!(actor.alarms[0], -1);
    // Frame 17: release Attack and Down, without moving or using a resource.
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    no_prior_warning(game, keeper);
    assert_eq!(game.player.held, Some(bomb));
    assert_eq!(game.player.pos, Vec2::new(104.0, 152.0));
    assert_eq!(game.world.entity(bomb).unwrap().alarms[1], 24);
}

#[test]
fn ordinary_native_trap_bomb_pickup_rethrow_keeps_shop_warning_disabled() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (mut game, keeper, chest) = scene(rules, -1);
            let inventory = (game.player.bombs, game.player.ropes, game.player.money);
            let bomb = source_trap(&mut game, keeper, chest);
            pickup_on_fixed_tape(&mut game, keeper, bomb);
            if facing > 0 {
                // Frames 18–19 face Right without changing x: the two-frame
                // acceleration delay admits this ordinary facing adjustment.
                for _ in 0..2 {
                    game.step(&InputFrame {
                        right: true,
                        ..Default::default()
                    });
                    no_prior_warning(&game, keeper);
                    assert_eq!(game.player.held, Some(bomb));
                    assert_eq!(game.player.pos, Vec2::new(104.0, 152.0));
                }
            }
            assert_eq!(game.player.facing, facing);
            let fuse = game.world.entity(bomb).unwrap().alarms[1];
            assert_eq!(fuse, if facing < 0 { 24 } else { 22 });
            assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
            // Frame 18 (Left) or20 (Right): repress Attack. sBombArmed bypasses
            // the first-use sBomb branch and selects the ordinary throw body.
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            // Mandatory release and fuse prerequisites precede the consequence.
            assert_eq!(game.player.held, None, "FIXTURE_REAL_THROW_COMPLETED");
            let actor = game
                .world
                .entity(bomb)
                .expect("same Bomb, no fuse explosion");
            assert!(!actor.held && actor.engine_active);
            assert_eq!(actor.alarms[1], fuse - 1);
            assert_eq!(actor.alarms[0], -1, "generic throw did not recook the fuse");
            assert_eq!(game.world.entity(keeper).unwrap().health, 20);
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oExplosion")
            );
            assert_eq!(
                (game.player.bombs, game.player.ropes, game.player.money),
                inventory
            );
            assert!(game.events.iter().any(|e| matches!(e,
                GameEvent::Sound { name, .. } if name == "sndThrow")));
            // Primary live consumer result before raw armed metadata: source
            // keeps armed=false, so the Bomb's own Step cannot accuse the player.
            assert!(
                !game.flags.contains("wanted"),
                "CONTRACT_RETHROW_DOES_NOT_ENABLE_BOMB_WARNING"
            );
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
            assert!(!game.events.iter().any(|e| matches!(e,
                GameEvent::Dialogue(message) if message.first == "TERRORIST!")));
            assert_eq!(game.world.entity(keeper).unwrap().state, 0);
            assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
        }
    }
}

#[test]
fn ordinary_button_bomb_still_warns_in_the_same_initial_shop() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (mut game, keeper, _) = scene(rules, facing);
            let before = game.player.bombs;
            no_prior_warning(&game, keeper);
            game.step(&InputFrame {
                bomb_pressed: true,
                ..Default::default()
            });
            assert!(
                game.flags.contains("wanted"),
                "source-explicit arming still warns"
            );
            assert!(game.data.get("thiefLevel").copied().unwrap_or(0.0) > 0.0);
            assert_eq!(game.player.bombs, before - 1);
            let actor = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oBomb")
                .unwrap();
            assert_eq!(actor.value("armed"), 1.0);
            assert_eq!(actor.sprite, "sBombArmed");
            assert_eq!(actor.alarms[0], 80);
            assert_eq!(game.world.entity(keeper).unwrap().health, 20);
        }
    }
}
