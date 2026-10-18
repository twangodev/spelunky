//! Source-grid authored geometry/actors followed only by ordinary pickup/throw.
//! Explicit SourceClassicV2; no midrun actor/timer/velocity/equipment/RNG writes.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, GenerationRevision, InputFrame, PlayerState,
    RngAlgorithm, RulesPreset, Vec2, World,
};

#[derive(Clone, Copy)]
enum Volume {
    DeepLava,
    ShallowLava,
    Water,
    Dry,
}
struct Fixture {
    game: Game,
    crate_id: u64,
    player: Vec2,
    face: i8,
    attack_held: bool,
    resources: (f64, f64, u32),
}
impl Fixture {
    fn step(&mut self, mut input: InputFrame) {
        assert!(!input.attack_pressed || (input.attack && !self.attack_held));
        input.attack_released = self.attack_held && !input.attack;
        self.attack_held = input.attack;
        self.game.step(&input);
        assert_eq!(
            self.game.player.health, 4.0,
            "FIXTURE Player remains outside pool"
        );
        assert_eq!(self.game.player.stun, 0);
        assert_eq!(self.game.player.pos, self.player);
        assert_eq!(self.game.player.facing, self.face);
        assert_eq!(
            (
                self.game.player.bombs,
                self.game.player.ropes,
                self.game.player.money
            ),
            self.resources
        );
        assert!(
            self.game
                .world
                .entities
                .iter()
                .all(|e| !spelunky_core::catalog::is_a(&e.object, "oEnemy"))
        );
    }
}
fn ready(preset: RulesPreset, face: i8, volume: Volume) -> Fixture {
    let mut game = Game::shell(7, preset);
    game.generation_revision = GenerationRevision::SourceClassicV2;
    assert_eq!(game.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..24 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    let player = Vec2::new(if face > 0 { 104.0 } else { 132.0 }, 152.0);
    game.player.pos = player;
    game.player.previous_pos = player;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = face;
    game.world.entrance = player;
    let crate_id = game.spawn("oFlareCrate", player);
    match volume {
        Volume::DeepLava => {
            game.spawn("oLava", Vec2::new(112.0, 128.0));
            game.spawn("oLava", Vec2::new(112.0, 144.0));
        }
        Volume::ShallowLava => {
            game.spawn("oLava", Vec2::new(112.0, 144.0));
        }
        Volume::Water => {
            game.spawn("oWaterSwim", Vec2::new(112.0, 128.0));
            game.spawn("oWaterSwim", Vec2::new(112.0, 144.0));
        }
        Volume::Dry => {}
    }
    let resources = (game.player.bombs, game.player.ropes, game.player.money);
    let mut f = Fixture {
        game,
        crate_id,
        player,
        face,
        attack_held: false,
        resources,
    };
    f.step(InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        f.game.player.held,
        Some(crate_id),
        "FIXTURE ordinary pickup succeeds"
    );
    f.step(InputFrame::default());
    assert_eq!(f.game.player.state, PlayerState::Standing);
    let item = f.game.world.entity(crate_id).unwrap();
    assert!(item.held);
    assert_eq!(item.pos, Vec2::new(player.x + f64::from(face) * 4.0, 148.0));
    assert_eq!(item.vel, Vec2::default());
    assert_eq!(item.value("heavy"), 1.0);
    assert_eq!(f.game.player.pickup_item_type, "Flare Crate");
    assert!(
        f.game
            .world
            .entities
            .iter()
            .all(|e| !matches!(e.object.as_str(), "oSplash" | "oPoof"))
    );
    f
}
fn family_point(game: &Game, point: Vec2, family: &str) -> bool {
    game.world.entities.iter().any(|e| {
        e.alive
            && spelunky_core::catalog::is_a(&e.object, family)
            && e.mask_contains(point, CollisionPrecision::BoundingBox)
    })
}
fn run(preset: RulesPreset, face: i8, volume: Volume) {
    let mut f = ready(preset, face, volume);
    let last_id = f.game.world.entities.iter().map(|e| e.id).max().unwrap();
    let mut state = f.game.rng.state_bits() as u32;
    let mut draw = |n: u32| {
        state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
        ((u64::from(state) * u64::from(n)) >> 32) as u32
    };
    // This third Step's Alarm0 fires before Player throws. Four literal draws
    // create the spark at the prior held position; its later own Step moves-.1.
    let spark_x = f.player.x + f64::from(face) * 4.0 + f64::from(draw(4)) - f64::from(draw(4));
    let spark_y = 148.0 - 4.0 + f64::from(draw(4)) - f64::from(draw(4)) - 0.1;
    let lava = matches!(volume, Volume::DeepLava | Volume::ShallowLava);
    let water = !matches!(volume, Volume::Dry);
    let pos = Vec2::new(
        if face > 0 { 112.0 } else { 124.0 },
        146.0 + if lava { 0.05 } else { 0.0 },
    );
    assert_eq!(
        family_point(&f.game, pos, "oWater"),
        water,
        "FIXTURE child's original center-water predicate"
    );
    assert_eq!(
        family_point(&f.game, Vec2::new(pos.x, pos.y - 5.0), "oLava"),
        matches!(volume, Volume::DeepLava),
        "FIXTURE parent's independent deep-Lava predicate"
    );
    assert!(!family_point(
        &f.game,
        Vec2::new(f.player.x, f.player.y + 6.0),
        "oLava"
    ));
    f.step(InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(f.game.player.held.is_none());
    assert!(f.game.player.pickup_item_type.is_empty());
    let new: Vec<_> = f
        .game
        .world
        .entities
        .iter()
        .filter(|e| e.id > last_id)
        .collect();
    let spark = new
        .first()
        .expect("FIXTURE normal Alarm0 birth precedes throw");
    assert_eq!(spark.object, "oFlareSpark");
    assert_eq!(spark.pos, Vec2::new(spark_x, spark_y));
    assert_eq!(spark.vel, Vec2::new(0.0, -0.1));
    let sounds: Vec<_> = f
        .game
        .events
        .iter()
        .filter_map(|e| {
            if let GameEvent::Sound { name, pos } = e {
                Some((name.as_str(), *pos))
            } else {
                None
            }
        })
        .collect();
    if water {
        // Baseline deep-Lava already retires the inherited receiver; prove that
        // before the child's semantic birth oracle so failures are qualified.
        assert!(f.game.world.entities.iter().all(|e| e.id != f.crate_id));
        assert_eq!(
            new.iter().filter(|e| e.object == "oSplash").count(),
            1,
            "own FlareCrate Step must continue after inherited lava Destroy"
        );
        assert_eq!(new.len(), 3);
        assert_eq!(new[1].object, "oSplash");
        assert_eq!(new[2].object, "oPoof");
        assert_eq!(new[1].pos, pos);
        assert_eq!(new[2].pos, pos);
        assert_eq!(new[1].vel, Vec2::default());
        assert_eq!(new[2].vel, Vec2::default());
        assert_eq!(new[1].image_speed(), 0.6);
        assert_eq!(new[2].image_speed(), 0.4);
        assert!(new[1].alive && new[2].alive);
        assert!(new[0].id < new[1].id && new[1].id < new[2].id);
        assert_eq!(sounds, vec![("sndThrow", f.player), ("sndSplash", pos)]);
    } else {
        assert_eq!(new.len(), 1);
        let item = f
            .game
            .world
            .entity(f.crate_id)
            .expect("dry thrown crate stays alive");
        assert_eq!(item.pos, pos);
        assert_eq!(item.vel, Vec2::new(f64::from(face) * 4.0, -2.0 + 0.6));
        assert!(!item.held);
        assert_eq!(sounds, vec![("sndThrow", f.player)]);
    }
    assert_eq!(
        f.game.rng.state_bits(),
        u64::from(state),
        "Splash/Poof/Destroy add no RNG to literal Alarm0 tape"
    );
}
fn cases(volume: Volume) {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for face in [-1, 1] {
            run(preset, face, volume);
        }
    }
}
#[test]
fn ordinary_thrown_flare_crate_deep_lava_runs_own_water_block_after_parent_destroy() {
    cases(Volume::DeepLava);
}
#[test]
fn ordinary_thrown_flare_crate_normal_water_runs_source_splash_poof() {
    cases(Volume::Water);
}
#[test]
fn ordinary_thrown_flare_crate_shallow_lava_runs_source_splash_poof_once() {
    cases(Volume::ShallowLava);
}
#[test]
fn ordinary_thrown_flare_crate_dry_path_remains_alive_without_splash_or_poof() {
    cases(Volume::Dry);
}
