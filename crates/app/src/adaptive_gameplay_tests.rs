//! Adaptive gameplay-view witnesses using authored rooms and ordinary inputs.
//! The paired old-source arm alone uses cfg(adaptive_view_baseline) to avoid
//! requiring a new API; its Game.step implementation remains unchanged.
//! All24 registrations are independent; a failed Classic case cannot hide Assisted.
use crate::{
    atlas::{Atlas, quad_matches_sprite_transition},
    scene::game_scene_in_view,
    viewport::Viewport,
};
use spelunky_core::{Entity, Game, InputFrame, RngAlgorithm, RulesPreset, RunState, Vec2};

fn configure(game: &mut Game, view: Viewport) {
    #[cfg(not(adaptive_view_baseline))]
    game.set_simulation_view_extent(f64::from(view.width), f64::from(view.height))
        .expect("finite source-logical extent >=canonical");
    #[cfg(adaptive_view_baseline)]
    let _ = (game, view); // Old arm has no adaptive-context API; never fabricates game state.
}

fn room(text: &str, name: &str, rules: RulesPreset, rng: RngAlgorithm) -> Game {
    let level = spelunky_core::level::Level::parse(text).expect("valid authored map");
    let mut game = Game::shell_with_algorithm(5, rules, rng).unwrap();
    game.load_custom(&level, name, "");
    assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
    game
}

fn actor(game: &Game, id: u64) -> Option<&Entity> {
    game.world
        .entities
        .iter()
        .chain(game.dispatcher.hidden_entities())
        .find(|e| e.id == id && e.alive)
}

fn sole_actor(game: &Game, object: &str) -> u64 {
    let all: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .collect();
    assert_eq!(all.len(), 1, "fixture must contain exactly one {object}");
    all[0].id
}

fn warmup(game: &mut Game) {
    for _ in 0..3 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.state, RunState::Playing);
}

fn source_own_guard(game: &Game, point: Vec2) -> bool {
    // Literal ORIGINAL Snake/Enemy guard: strict endpoints, before20/after4.
    point.x > game.camera.x - 20.0
        && point.x < game.camera.x + 324.0
        && point.y > game.camera.y - 20.0
        && point.y < game.camera.y + 244.0
}

fn rendered(game: &Game, id: u64, view: Viewport, alpha: f64, atlas: &Atlas) -> bool {
    let draw = game.presentation();
    let entity = actor(draw, id).expect("alive frozen Draw witness");
    let frame = atlas
        .sprite(&entity.sprite, entity.animation)
        .expect("original source sprite");
    // These authored witnesses use16px top-left origins. oEnemy Draw's16px
    // mirrored compensation preserves their world top-left, in both facings.
    assert_eq!(frame.size, [16.0, 16.0]);
    assert_eq!(frame.origin, [0.0, 0.0]);
    let discontinuity = draw.flags.contains("render_discontinuity");
    let animation_alpha = if discontinuity {
        0.0
    } else {
        alpha.clamp(0.0, 1.0)
    };
    let alpha = if discontinuity { 1.0 } else { alpha };
    let camera = view.camera(
        draw.previous_camera.lerp(draw.camera, alpha),
        draw.world.width as f64 * 16.0,
        draw.world.height as f64 * 16.0,
    );
    let position = if entity.engine_active {
        entity.previous_pos.lerp(entity.pos, alpha)
    } else {
        entity.pos
    };
    let expected = [
        (position.x - camera.x) as f32,
        (position.y - camera.y) as f32,
    ];
    let animation = entity.animation + entity.image_speed() * animation_alpha;
    let rendered_frame = atlas
        .sprite(&entity.sprite, animation)
        .expect("presentation source sprite");
    let hash = game.state_hash();
    let draw_hash = draw.state_hash();
    let rng = game.rng.clone().next_u64();
    let quads = game_scene_in_view(game, alpha, false, atlas, view);
    let found = quads.iter().any(|q| {
        quad_matches_sprite_transition(
            atlas,
            q,
            &entity.sprite,
            animation,
            q.uv[2].is_sign_negative(),
        ) && {
            let uv = [
                q.uv[0].min(q.uv[0] + q.uv[2]),
                q.uv[1],
                q.uv[2].abs(),
                q.uv[3],
            ];
            uv.iter()
                .zip(rendered_frame.uv)
                .all(|(a, b)| (*a - b).abs() < 1e-6)
        } && (q.rect[0] - expected[0]).abs() < 1e-4
            && (q.rect[1] - expected[1]).abs() < 1e-4
            && q.rect[2..] == rendered_frame.size
    });
    assert_eq!(
        game.state_hash(),
        hash,
        "rendering cannot unfreeze source AI"
    );
    assert_eq!(game.presentation().state_hash(), draw_hash);
    assert_eq!(game.rng.clone().next_u64(), rng);
    found
}

fn visible_only_in_extra_strip(game: &Game, id: u64, view: Viewport, atlas: &Atlas) {
    assert!(!rendered(game, id, Viewport::canonical(), 1.0, atlas));
    for alpha in [0.0, 0.5, 1.0] {
        assert!(rendered(game, id, view, alpha, atlas));
    }
}

fn movement_witness(
    rules: RulesPreset,
    rng: RngAlgorithm,
    map: &str,
    name: &str,
    object: &str,
    surface: (u32, u32),
    expected_start: Vec2,
) {
    let atlas = Atlas::load().unwrap();
    let initial = room(map, name, rules, rng);
    let id = sole_actor(&initial, object);
    assert_eq!(
        actor(&initial, id).unwrap().pos,
        expected_start,
        "authored origin prerequisite"
    );
    let view = Viewport::from_surface(surface.0, surface.1);
    // The unchanged compatibility sibling proves actual old-strip visibility/gate.
    // It is stepped, never given a new context or edited actor/camera/timer.
    let mut compatibility = initial.clone();
    warmup(&mut compatibility);
    assert_eq!(actor(&compatibility, id).unwrap().pos, expected_start);
    assert!(!source_own_guard(&compatibility, expected_start));
    visible_only_in_extra_strip(&compatibility, id, view, &atlas);
    let mut adaptive = initial;
    configure(&mut adaptive, view); // Initial view setup, before any Game.step.
    warmup(&mut adaptive);
    assert_eq!(
        adaptive.player.health, 4.0,
        "distant actor does not damage Player during witness"
    );
    let e = actor(&adaptive, id).expect("live visible adaptive actor");
    assert!(
        e.engine_active,
        "visible actor must participate in source phases"
    );
    assert_ne!(
        e.pos, expected_start,
        "adaptive visible actor own+inherited movement must run"
    );
    for alpha in [0.0, 0.5, 1.0] {
        assert!(rendered(&adaptive, id, view, alpha, &atlas));
    }
}
fn wide_snake(r: RulesPreset, g: RngAlgorithm) {
    movement_witness(
        r,
        g,
        include_str!("../tests/fixtures/adaptive-gameplay/wide-snake.LVL"),
        "wide-snake.LVL",
        "oSnake",
        (1920, 1080),
        Vec2::new(368.0, 144.0),
    );
}
fn wide_keeper(r: RulesPreset, g: RngAlgorithm) {
    movement_witness(
        r,
        g,
        include_str!("../tests/fixtures/adaptive-gameplay/wide-shopkeeper.LVL"),
        "wide-shopkeeper.LVL",
        "oShopkeeper",
        (1920, 1080),
        Vec2::new(368.0, 144.0),
    );
}
fn ultra_snake(r: RulesPreset, g: RngAlgorithm) {
    movement_witness(
        r,
        g,
        include_str!("../tests/fixtures/adaptive-gameplay/ultrawide-snake.LVL"),
        "ultrawide-snake.LVL",
        "oSnake",
        (3440, 1440),
        Vec2::new(464.0, 144.0),
    );
}
fn tall_snake(r: RulesPreset, g: RngAlgorithm) {
    movement_witness(
        r,
        g,
        include_str!("../tests/fixtures/adaptive-gameplay/portrait-snake.LVL"),
        "portrait-snake.LVL",
        "oSnake",
        (720, 960),
        Vec2::new(128.0, 304.0),
    );
}
fn wide_spring(rules: RulesPreset, rng: RngAlgorithm) {
    let atlas = Atlas::load().unwrap();
    let initial = room(
        include_str!("../tests/fixtures/adaptive-gameplay/wide-spring.LVL"),
        "wide-spring.LVL",
        rules,
        rng,
    );
    let id = sole_actor(&initial, "oSpringTrap");
    assert_eq!(actor(&initial, id).unwrap().pos, Vec2::new(384.0, 144.0));
    assert!(
        !initial.world.solid_at(384.0, 160.0),
        "initial authored missing support"
    );
    let view = Viewport::from_surface(1920, 1080);
    let mut compatibility = initial.clone();
    warmup(&mut compatibility);
    assert!(actor(&compatibility, id).unwrap().engine_active);
    visible_only_in_extra_strip(&compatibility, id, view, &atlas);
    let mut adaptive = initial;
    configure(&mut adaptive, view);
    warmup(&mut adaptive);
    assert_eq!(adaptive.player.health, 4.0);
    assert!(
        actor(&adaptive, id).is_none(),
        "visible unsupported spring must run source support Destroy"
    );
    assert!(
        !adaptive
            .presentation()
            .world
            .entities
            .iter()
            .chain(adaptive.presentation().dispatcher.hidden_entities())
            .any(|e| e.id == id && e.alive),
        "frozen Draw cannot retain destroyed trap"
    );
}
fn canonical_identity(rules: RulesPreset, rng: RngAlgorithm) {
    let atlas = Atlas::load().unwrap();
    let mut source = room(
        include_str!("../tests/fixtures/adaptive-gameplay/wide-snake.LVL"),
        "wide-snake.LVL",
        rules,
        rng,
    );
    let mut configured = source.clone();
    configure(&mut configured, Viewport::canonical());
    for _ in 0..120 {
        let input = InputFrame {
            right: true,
            ..Default::default()
        };
        source.step(&input);
        configured.step(&input);
        assert_eq!(
            serde_json::to_vec(&source).unwrap(),
            serde_json::to_vec(&configured).unwrap(),
            "canonical context must preserve exact serialized bytes"
        );
        assert_eq!(
            source.state_hash(),
            configured.state_hash(),
            "canonical context must be byte-identical"
        );
        assert_eq!(
            source.presentation().state_hash(),
            configured.presentation().state_hash()
        );
        assert_eq!(
            source.rng.clone().next_u64(),
            configured.rng.clone().next_u64()
        );
        assert_eq!(
            serde_json::to_vec(&source.events).unwrap(),
            serde_json::to_vec(&configured.events).unwrap()
        );
        let before = configured.state_hash();
        let draw = configured.presentation().state_hash();
        for view in [
            Viewport::canonical(),
            Viewport::from_surface(1920, 1080),
            Viewport::from_surface(3440, 1440),
            Viewport::from_surface(720, 960),
        ] {
            for alpha in [0.0, 0.5, 1.0] {
                let _ = game_scene_in_view(&configured, alpha, false, &atlas, view);
            }
        }
        assert_eq!(before, configured.state_hash());
        assert_eq!(draw, configured.presentation().state_hash());
    }
}
macro_rules! witness {
    ($name:ident,$body:ident,$rules:ident,$rng:ident) => {
        #[test]
        fn $name() {
            $body(RulesPreset::$rules, RngAlgorithm::$rng);
        }
    };
}
witness!(wide_snake_classic_gm8, wide_snake, Classic, Gm8Lcg32V1);
witness!(
    wide_snake_classic_legacy,
    wide_snake,
    Classic,
    LegacySplitMix64V1
);
witness!(wide_snake_assisted_gm8, wide_snake, Assisted, Gm8Lcg32V1);
witness!(
    wide_snake_assisted_legacy,
    wide_snake,
    Assisted,
    LegacySplitMix64V1
);
witness!(wide_keeper_classic_gm8, wide_keeper, Classic, Gm8Lcg32V1);
witness!(
    wide_keeper_classic_legacy,
    wide_keeper,
    Classic,
    LegacySplitMix64V1
);
witness!(wide_keeper_assisted_gm8, wide_keeper, Assisted, Gm8Lcg32V1);
witness!(
    wide_keeper_assisted_legacy,
    wide_keeper,
    Assisted,
    LegacySplitMix64V1
);
witness!(wide_spring_classic_gm8, wide_spring, Classic, Gm8Lcg32V1);
witness!(
    wide_spring_classic_legacy,
    wide_spring,
    Classic,
    LegacySplitMix64V1
);
witness!(wide_spring_assisted_gm8, wide_spring, Assisted, Gm8Lcg32V1);
witness!(
    wide_spring_assisted_legacy,
    wide_spring,
    Assisted,
    LegacySplitMix64V1
);
witness!(ultra_snake_classic_gm8, ultra_snake, Classic, Gm8Lcg32V1);
witness!(
    ultra_snake_classic_legacy,
    ultra_snake,
    Classic,
    LegacySplitMix64V1
);
witness!(ultra_snake_assisted_gm8, ultra_snake, Assisted, Gm8Lcg32V1);
witness!(
    ultra_snake_assisted_legacy,
    ultra_snake,
    Assisted,
    LegacySplitMix64V1
);
witness!(tall_snake_classic_gm8, tall_snake, Classic, Gm8Lcg32V1);
witness!(
    tall_snake_classic_legacy,
    tall_snake,
    Classic,
    LegacySplitMix64V1
);
witness!(tall_snake_assisted_gm8, tall_snake, Assisted, Gm8Lcg32V1);
witness!(
    tall_snake_assisted_legacy,
    tall_snake,
    Assisted,
    LegacySplitMix64V1
);
witness!(
    canonical_identity_classic_gm8,
    canonical_identity,
    Classic,
    Gm8Lcg32V1
);
witness!(
    canonical_identity_classic_legacy,
    canonical_identity,
    Classic,
    LegacySplitMix64V1
);
witness!(
    canonical_identity_assisted_gm8,
    canonical_identity,
    Assisted,
    Gm8Lcg32V1
);
witness!(
    canonical_identity_assisted_legacy,
    canonical_identity,
    Assisted,
    LegacySplitMix64V1
);
