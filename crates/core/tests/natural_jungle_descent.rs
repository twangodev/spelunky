//! Seed-zero SourceClassicV2 Jungle descent driven from its generated entrance.
use spelunky_core::{
    Area, Game, GenerationRevision, InputFrame, PlayerState, RulesPreset, RunState, Vec2, catalog,
};

fn combat_or_hazard(object: &str) -> bool {
    catalog::is_a(object, "oEnemy")
        || matches!(
            object,
            "oArrowTrapLeft"
                | "oArrowTrapRight"
                | "oArrowTrapLeftLit"
                | "oArrowTrapRightLit"
                | "oSpikes"
                | "oSpearTrapTop"
                | "oSpearTrapBottom"
                | "oSmash"
                | "oGiantTikiHead"
                | "oTrapBlock"
        )
}

fn source_room_row(y: f64) -> usize {
    ((y - 16.0) / 128.0).floor().clamp(0.0, 3.0) as usize
}

#[test]
fn seed_zero_jungle_reaches_bottom_row_with_live_generated_combat() {
    let mut game = Game::new(0, RulesPreset::Classic);
    // Load the deterministic level-five room through the campaign generator.
    // Once level five is entered, the player and all generated actors remain
    // untouched; the route below advances only through Game::step input.
    while game.world.level < 5 {
        game.next_level();
    }

    assert_eq!(
        game.generation_revision,
        GenerationRevision::SourceClassicV2
    );
    assert_eq!(game.world.level, 5);
    assert_eq!(game.world.area, Area::Jungle);
    assert_eq!(game.world.name, "Jungle");
    assert_eq!(game.world.entrance, Vec2::new(424.0, 88.0));
    assert_eq!(game.world.exit, Vec2::new(104.0, 472.0));
    assert_eq!(game.player.pos, game.world.entrance);
    assert_eq!(game.player.health, 4.0);
    let path = game.world.source_room_path.expect("source room path");
    assert_eq!(
        path,
        [
            [2, 1, 1, 0],
            [3, 1, 1, 2],
            [0, 0, 0, 2],
            [1, 1, 1, 3],
            [0, 0, 0, 0],
        ]
    );
    let live_threats = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.engine_active && combat_or_hazard(&entity.object))
        .count();
    assert!(
        live_threats > 0,
        "generated combat and hazards must stay live"
    );

    // Each row's `roomPath == 2` cell marks the next descent. Room width is
    // 160 px; scan nearby floor openings when the center is blocked. All
    // movement, jumps, whips, and drops are ordinary Game::step input.
    let scan_offsets = [
        0.0, -16.0, 16.0, -32.0, 32.0, -48.0, 48.0, -64.0, 64.0, -72.0, 72.0,
    ];
    let mut visited_rows = [false; 4];
    let mut previous = game.player.pos;
    let mut previous_row = source_room_row(game.player.pos.y);
    visited_rows[previous_row] = true;
    let mut stuck = 0_u8;
    let mut jump_ticks = 0_u8;
    let mut row_stall = 0_u8;
    let mut scan_index = 0_usize;
    let mut attack_cooldown = 0_u8;
    let mut previous_attack = false;
    let mut reached_row_three = false;

    for _ in 0..700 {
        assert_eq!(
            game.state,
            RunState::Playing,
            "player died before row three"
        );
        let row = source_room_row(game.player.pos.y);
        visited_rows[row] = true;
        if row == 3 {
            reached_row_three = true;
            break;
        }
        let col = path[row]
            .iter()
            .position(|room| *room == 2)
            .expect("each descent row has a marked exit room");
        if row != previous_row {
            previous_row = row;
            row_stall = 0;
            scan_index = 0;
        }
        let anchor_x = 96.0 + col as f64 * 160.0;
        let target_x = anchor_x + scan_offsets[scan_index];
        let dx = target_x - game.player.pos.x;
        let horizontal = if dx.abs() < 5.0 {
            0
        } else if dx > 0.0 {
            1
        } else {
            -1
        };
        if game.player.grounded && dx.abs() < 8.0 {
            row_stall = row_stall.saturating_add(1);
            if row_stall >= 12 {
                scan_index = (scan_index + 1).min(scan_offsets.len() - 1);
                row_stall = 0;
            }
        } else {
            row_stall = 0;
        }

        let no_progress = (game.player.pos.x - previous.x).abs() < 0.5
            && (game.player.pos.y - previous.y).abs() < 0.5;
        if horizontal != 0
            && no_progress
            && (game.player.grounded || game.player.state == PlayerState::Hanging)
        {
            stuck = stuck.saturating_add(1);
        } else {
            stuck = 0;
        }
        if stuck >= 3 && jump_ticks == 0 && dx.abs() >= 8.0 {
            jump_ticks = 8;
            stuck = 0;
        }
        let jump = jump_ticks > 0;
        let jump_pressed = jump_ticks == 8;
        jump_ticks = jump_ticks.saturating_sub(1);

        attack_cooldown = attack_cooldown.saturating_sub(1);
        let enemy_ahead = horizontal != 0
            && game.world.entities.iter().any(|entity| {
                entity.alive
                    && entity.engine_active
                    && catalog::is_a(&entity.object, "oEnemy")
                    && (entity.pos.x - game.player.pos.x).abs() < 44.0
                    && (entity.pos.y - game.player.pos.y).abs() < 28.0
                    && (entity.pos.x - game.player.pos.x).signum() == f64::from(horizontal)
            });
        let attack_pressed = enemy_ahead && attack_cooldown == 0;
        if attack_pressed {
            attack_cooldown = 18;
        }
        game.step(&InputFrame {
            left: horizontal < 0,
            right: horizontal > 0,
            down: dx.abs() < 20.0,
            run: true,
            jump,
            jump_pressed,
            attack: attack_pressed,
            attack_pressed,
            attack_released: previous_attack && !attack_pressed,
            ..InputFrame::default()
        });
        previous_attack = attack_pressed;
        previous = game.player.pos;
    }

    assert!(
        reached_row_three,
        "route did not reach the bottom descent row"
    );
    assert_eq!(game.state, RunState::Playing);
    assert!(game.player.health > 0.0);
    assert!(visited_rows.into_iter().all(|visited| visited));
    assert_eq!(game.world.level, 5);
    assert_eq!(game.world.area, Area::Jungle);
    assert_eq!(source_room_row(game.player.pos.y), 3);
}
