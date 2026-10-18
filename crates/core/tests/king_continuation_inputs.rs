//! Source-default King and ordinary Pistol pickup/fire. Initial ThinIce only;
//! no receiver or RNG writes after the first Game.step. Independent death-prefix
//! oracle protects the existing blood/bone/loot sequence before the final yell.
use spelunky_core::{
    Entity, Game, GameEvent, InputFrame, PlayerState, Rng, RngAlgorithm, RulesPreset, Vec2,
    catalog, level::Level,
};

fn arena(seed: u32, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level {
        music: "TEMPLE".into(),
        starting_life: 99, // ordinary authored level metadata, no mid-combat healing
        ..Default::default()
    };
    level.set_tile(9, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    level.set_tile(10, 7, '-').unwrap(); // ordinary source custom-level Pistol
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    // Source King attackTimer can still be positive after its thirty-tick
    // post-yell IDLE, allowing real WALK steps toward the player before attack.
    // This initial upper Brick stops its custom left bound at x208, hence
    // origin>=202, while the ordinary Pistol muzzle (y132..134) passes below.
    level.set_tile(11, 6, '1').unwrap();
    // A second initial upper Brick confines the incoming positive recoil
    // below the strict right viewport edge without blocking muzzle y132..134.
    level.set_tile(17, 6, '1').unwrap();
    let mut game = Game::shell(seed, rules);
    game.load_custom(&level, "king-ordinary-pistol-debris", "");
    assert_eq!(
        game.world.tile(12, 7).unwrap().object,
        "oBrick",
        "custom border maps the initial stopper to [192,208) x [112,128)"
    );
    assert_eq!(
        game.world.tile(14, 9).unwrap().object,
        "oBrick",
        "floor top stays144"
    );
    // Authored initial enemy birth: the custom character palette has no King.
    // Source Create supplies all HP/state/facing/velocity/image/clock fields.
    let king = game.spawn("oYetiKing", Vec2::new(224.0, 112.0));
    assert_eq!(game.world.entity(king).unwrap().health, 30);
    assert_eq!(game.world.entity(king).unwrap().value("bloodless"), 0.0);
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| catalog::is_a(&e.object, "oIce"))
    );
    assert!(
        !game
            .world
            .tiles
            .iter()
            .any(|t| catalog::is_a(&t.object, "oIce"))
    );
    game.spawn("oThinIce", Vec2::new(288.0, 80.0));
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
    let held = game
        .player
        .held
        .expect("ordinary down+attack picks up the authored Pistol");
    assert_eq!(game.world.entity(held).unwrap().object, "oPistol");
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.facing, 1);
    (game, king)
}

// Predict only the original steady supported ATTACK and its first IDLE frame.
// This uses local snapshots and canonical precise masks, not Game::step on a
// second world, and never changes a live receiver, input cache, or RNG stream.
fn final_contact_matches_source_phase(
    game: &Game,
    king: u64,
    bullet_template: &Entity,
    yelling: bool,
) -> bool {
    let actor = game.world.entity(king).unwrap();
    if actor.state != 3
        || actor.vel != Vec2::default()
        || game.player.vel != Vec2::default()
        || game.player.state != PlayerState::Standing
    {
        return false;
    }
    assert_eq!(catalog::sprite_frame_count(&actor.sprite), 13);
    let mut probe_rng = game.rng.clone();
    let speed = (6.0 + f64::from(rand(&mut probe_rng, 3)) + game.player.vel.x).max(6.0);
    let mut bullet = bullet_template.clone();
    bullet.pos = Vec2::new(game.player.pos.x + 8.0, game.player.pos.y - 2.0);
    let mut target = actor.clone();
    let mut attack = true;
    for _ in 0..40 {
        if !attack {
            target.set_sprite(if target.facing < 0 {
                "sYetiKingLeft"
            } else {
                "sYetiKingRight"
            });
        }
        // Source Bullet669 moves before King877; both automatic masks are
        // queried before Draw/Animation advances the receiver's image index.
        bullet.pos.x += speed;
        let contact = bullet.event_intersects(&target);
        let mut frame = target.animation + if attack { 0.5 } else { 0.25 };
        if attack && frame >= 13.0 {
            frame -= 13.0;
            attack = false;
        }
        if contact {
            return attack == yelling && frame == if yelling { 11.25 } else { 0.5 };
        }
        target.animation = frame;
        // Beyond the first requested IDLE frame, this deliberately stops
        // rather than guessing subsequent WALK movement or another attack.
        if !attack && frame >= 0.5 {
            return false;
        }
    }
    false
}

fn shoot_until(game: &mut Game, king: u64, fatal: bool, yelling: bool) -> usize {
    let mut previous_hp = 30;
    let mut hits = 0;
    let mut bullet_template = None;
    let mut attacks = usize::from(game.world.entity(king).unwrap().state == 3);
    for _ in 0..480 {
        let before_state = game.world.entity(king).unwrap().state;
        let pending = game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oBullet");
        let ready = game
            .player
            .data
            .get("weapon_cooldown")
            .copied()
            .unwrap_or(0.0)
            <= 1.0;
        let fire = !pending
            && ready
            && (hits < 7
                || !fatal
                || (attacks == 2
                    && final_contact_matches_source_phase(
                        game,
                        king,
                        bullet_template
                            .as_ref()
                            .expect("native first Bullet template"),
                        yelling,
                    )));
        if fire {
            assert!(
                game.player.pos.y - 2.0 >= 128.0,
                "ordinary Pistol muzzle remains below the initial upper Brick"
            );
        }
        game.step(&InputFrame {
            attack: fire,
            attack_pressed: fire,
            ..Default::default()
        });
        if bullet_template.is_none() {
            bullet_template = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oBullet")
                .cloned();
        }
        let actor = game
            .world
            .entity(king)
            .expect("fatal Bullet Collision precedes the following King Normal Step");
        if actor.state == 3 && before_state != 3 {
            attacks += 1;
        }
        assert!(
            actor.pos.x < game.camera.x + 320.0,
            "SOURCE_KING_STEP_REMAINS_ACTIVE"
        );
        if actor.health != previous_hp {
            assert_eq!(
                actor.health,
                previous_hp - 4,
                "every HP change is an ordinary Pistol hit"
            );
            previous_hp = actor.health;
            hits += 1;
        }
        if (fatal && actor.health <= 0) || (!fatal && hits == 1) {
            assert!(
                game.player.health > 0.0,
                "ordinary player controller remains live"
            );
            assert!(
                game.player.held.is_some(),
                "weapon acquired and retained through normal play"
            );
            return hits;
        }
    }
    panic!("ordinary Pistol inputs must reach the requested hit boundary");
}

// Independent arithmetic over one raw draw, not production range()/unit().
// Canonical mapping follows frozen EXE-derived math; Legacy retains historical
// SplitMix mapping. These controls are not original gameplay runtime captures.
fn rand(rng: &mut Rng, width: u64) -> u32 {
    assert!((1..=u64::from(u32::MAX)).contains(&width));
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
    }
}
fn unit(rng: &mut Rng) -> f64 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => raw as f64 / 4_294_967_296.0,
        RngAlgorithm::LegacySplitMix64V1 => (raw >> 11) as f64 / 9_007_199_254_740_992.0,
    }
}

fn launch(rng: &mut Rng) -> (Vec2, f64) {
    let velocity = Vec2::new(unit(rng) * 4.0 - unit(rng) * 4.0, -1.0 - unit(rng) * 2.0);
    let gravity = (1 + rand(rng, 6)) as f64 * 0.1;
    (velocity, gravity)
}

fn death_prefix(rng: &mut Rng) {
    rand(rng, 5);
    rand(rng, 5);
    for _ in 0..3 {
        launch(rng);
    }
    for _ in 0..4 {
        rand(rng, 5);
        rand(rng, 7);
        launch(rng);
    }
    for _ in 0..4 {
        if rand(rng, 12) != 0 {
            rand(rng, 2);
        }
        rand(rng, 4);
        rand(rng, 4);
        rand(rng, 2);
    }
}

#[test]
fn ordinary_fatal_pistol_hit_keeps_king_final_yell_and_thin_ice_after_complete_death_prefix() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [17, 42] {
            let (mut game, king) = arena(seed, rules);
            assert_eq!(shoot_until(&mut game, king, true, seed == 42), 8);
            let actor = game.world.entity(king).unwrap();
            assert_eq!(actor.health, -2);
            let yelling = seed == 42;
            assert_eq!(actor.state, if yelling { 3 } else { 0 });
            assert_eq!(actor.animation, if yelling { 11.25 } else { 0.5 });
            assert!(game.player.health > 0.0);
            assert!(!game.world.entities.iter().any(|e| e.object == "oBullet"));
            let count = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
                .count();
            assert!(count <= 13, "all three scripted Blood births fit cap16");
            let thin = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oThinIce")
                .unwrap();
            let thin_id = thin.id;
            let thickness = thin.value("thickness");
            assert_eq!(thickness, if yelling { 24.0 } else { 20.0 });
            let serial = game.world.creation_serial;
            let kills = game.data.get("kills").copied().unwrap_or(0.0);
            let mut expected = game.rng.clone();
            death_prefix(&mut expected);

            game.step(&InputFrame::default());

            assert!(game.world.entity(king).is_none());
            assert_eq!(
                game.world.entity(thin_id).unwrap().value("thickness"),
                thickness - if yelling { 2.0 } else { 0.0 }
            );
            let yells = game
                .events
                .iter()
                .filter(
                    |event| matches!(event,GameEvent::SoundIfIdle{name,..} if name=="sndYetiYell"),
                )
                .count();
            assert_eq!(yells, usize::from(yelling));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.creation_serial > serial
                        && matches!(
                            e.object.as_str(),
                            "oBlood" | "oBone" | "oSpikeShoes" | "oSapphireBig" | "oRopePile"
                        ))
                    .count(),
                11
            );
            assert_eq!(game.data.get("kills"), Some(&(kills + 1.0)));
            assert_eq!(game.data.get("yetikings"), Some(&1.0));
            assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());

            game.step(&InputFrame::default());
            assert!(game.world.entity(king).is_none());
            assert_eq!(
                game.world.entity(thin_id).unwrap().value("thickness"),
                thickness - if yelling { 2.0 } else { 0.0 }
            );
            assert!(!game.events.iter().any(
                |event| matches!(event,GameEvent::SoundIfIdle{name,..} if name=="sndYetiYell")
            ));
        }
    }
}
