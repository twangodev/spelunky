//! Authored initial actors and geometry; betting, gravity, pickup and reroll
//! proceed through ordinary Game::step only. Explicit SourceClassicV2 fixture.
use spelunky_core::{
    Area, Game, GameEvent, GenerationRevision, InputFrame, PlayerState, RngAlgorithm, RulesPreset,
    Vec2, World,
};

struct Table {
    game: Game,
    keeper: u64,
    a: u64,
    b: u64,
    attack_held: bool,
}

fn table(rules: RulesPreset, lava: bool, initial_fall: bool) -> Table {
    let mut game = Game::shell(7, rules);
    game.generation_revision = GenerationRevision::SourceClassicV2;
    assert_eq!(game.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..24 {
        game.world.set_tile(x, 11, "oHardBlock");
    }
    game.player.pos = Vec2::new(96.0, 168.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.money = 5000;
    game.world.entrance = game.player.pos;
    // Keeper feet lie sixteen pixels below its origin, on floor176. Its Step
    // precedes Dice, so the same-frame anger cannot run ATTACK afterward.
    let keeper = game.spawn("oShopkeeper", Vec2::new(32.0, 160.0));
    game.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), "Craps".into());
    // This ordinary initial gravity drop is the first roll, not a first throw.
    // The untouched second die prevents settlement from clearing the bet.
    let a = game.spawn(
        "oDice",
        Vec2::new(96.0, if initial_fall { 160.0 } else { 168.0 }),
    );
    let b = game.spawn("oDice", Vec2::new(144.0, 168.0));
    // Original scrUseItem overhead point(104,158) cancels Up vy and adds1 vx.
    // Source-valid authored ceiling/Lava geometry is not a stock-room claim.
    game.spawn("oHardBlock", Vec2::new(104.0, 144.0));
    if lava {
        game.spawn("oLava", Vec2::new(115.0, 160.0));
    }
    Table {
        game,
        keeper,
        a,
        b,
        attack_held: false,
    }
}

fn thief_messages(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| {
            matches!(event,
                GameEvent::Dialogue(message) if message.first == "COME BACK HERE, THIEF!"
            )
        })
        .count()
}

fn step(table: &mut Table, mut input: InputFrame) {
    assert!(!input.attack_pressed || (input.attack && !table.attack_held));
    input.attack_released = table.attack_held && !input.attack;
    table.attack_held = input.attack;
    table.game.step(&input);
    assert_eq!(table.game.player.health, 4);
    assert_eq!(
        table.game.player.pos,
        Vec2::new(96.0, 168.0),
        "ordinary inputs never move Player"
    );
    assert_eq!(
        table.game.generation_revision,
        GenerationRevision::SourceClassicV2
    );
}

fn peaceful(table: &Table) {
    assert_eq!(
        table.game.data.get("thiefLevel").copied().unwrap_or(0.0),
        0.0
    );
    let keeper = table.game.world.entity(table.keeper).unwrap();
    assert_eq!((keeper.state, keeper.health), (0, 20));
    assert_eq!(keeper.value("angered"), 0.0);
    assert_eq!(thief_messages(&table.game), 0);
    for id in [table.a, table.b] {
        let die = table.game.world.entity(id).unwrap();
        assert!(
            (16.0..176.0).contains(&die.pos.x) && (144.0..272.0).contains(&die.pos.y),
            "pre-motion source shop room0,row1 must contain both dice"
        );
    }
}

fn pending_bet(table: &Table) {
    assert_eq!(table.game.player.money, 3500);
    assert_eq!(table.game.player.data.get("bet").copied(), Some(1500.0));
    assert_eq!(
        table.game.world.entity(table.b).unwrap().value("rolled"),
        0.0
    );
}

fn neutral(table: &mut Table) {
    step(table, InputFrame::default());
}

fn pay_and_establish_first_roll(table: &mut Table, initial_fall: bool) {
    // Actual pay is before gravity creates any fast-roll latch. Initial welcome
    // draws legitimately; fast Dice Steps legitimately draw subsequent faces.
    step(
        table,
        InputFrame {
            pay_pressed: true,
            ..Default::default()
        },
    );
    pending_bet(table);
    peaceful(table);
    assert_eq!(
        table.game.world.entity(table.a).unwrap().value("rolled"),
        0.0
    );
    let mut rolling_observed = false;
    let mut settled = !initial_fall;
    if initial_fall {
        for _ in 0..128 {
            let die = table.game.world.entity(table.a).unwrap();
            rolling_observed |= die.value("rolling") == 1.0;
            if die.value("rolled") == 1.0 && die.value("rolling") == 0.0 {
                assert_eq!(die.pos, Vec2::new(96.0, 168.0));
                assert_eq!(die.vel, Vec2::default());
                settled = true;
                break;
            }
            neutral(table);
            pending_bet(table);
            peaceful(table);
        }
        assert!(
            rolling_observed && settled,
            "natural first gravity roll must establish both latches"
        );
    } else {
        assert!(settled);
        let die = table.game.world.entity(table.a).unwrap();
        assert_eq!((die.value("rolled"), die.value("rolling")), (0.0, 0.0));
    }
    pending_bet(table);
    peaceful(table);
}

fn pick_up_and_begin_flat_reroll(table: &mut Table, rolled: f64) {
    step(
        table,
        InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(table.game.player.held, Some(table.a));
    neutral(table); // physical Attack release; held heavy Dice attaches at166
    assert_eq!(
        table.game.world.entity(table.a).unwrap().pos,
        Vec2::new(100.0, 166.0)
    );
    assert_eq!(
        table.game.world.entity(table.a).unwrap().value("rolled"),
        rolled
    );
    assert_eq!(table.game.player.vel, Vec2::default());
    // Ordinary neutral waiting, never a clock write: select an odd launch body
    // clock so the following .6 velocity moves1 on its source even clock.
    // dispatcher increments level_tick before Step; engine_time then adds1.
    if (table.game.level_tick + 2).is_multiple_of(2) {
        neutral(table);
    }
    assert!(!(table.game.level_tick + 2).is_multiple_of(2));
    step(
        table,
        InputFrame {
            up: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        },
    );
    assert!(table.game.player.held.is_none());
    assert!(!table.game.engine_time().is_multiple_of(2));
    let die = table.game.world.entity(table.a).unwrap();
    assert!(!die.held);
    assert_eq!(die.pos, Vec2::new(105.0, 166.0));
    assert_eq!(
        die.vel,
        Vec2::new(5.0, 0.6),
        "heavy4/Up-4 becomes5/0 from source overhead point, then gravity"
    );
    assert_eq!((die.value("rolled"), die.value("rolling")), (rolled, 1.0));
    pending_bet(table);
    peaceful(table);
    // Second natural fast dry Step: center110 + radius3 stays left of Lava115.
    neutral(table);
    assert!(table.game.engine_time().is_multiple_of(2));
    let die = table.game.world.entity(table.a).unwrap();
    assert_eq!(die.pos, Vec2::new(110.0, 167.0));
    assert_eq!(die.vel, Vec2::new(5.0, 1.2));
    assert_eq!((die.value("rolled"), die.value("rolling")), (rolled, 1.0));
    pending_bet(table);
    peaceful(table);
}

fn final_step(table: &mut Table, lava: bool, rerolled: bool) {
    let before = table.game.world.entity(table.a).unwrap().clone();
    assert!(!before.held);
    assert_eq!(before.pos, Vec2::new(110.0, 167.0));
    assert_eq!(before.vel, Vec2::new(5.0, 1.2));
    assert_eq!(
        (before.value("rolled"), before.value("rolling")),
        (f64::from(rerolled), 1.0)
    );
    // Literal source oracle:1.2 moves1 or2 (period5), floor176 clips center to168;
    // vx5 reaches115. Lava rectangle and y-5 point both hit [115,130]x[160,175].
    // Source bb=168+8=176, horizontal bottom line109..120 hits full initial floor.
    let next_clock = (table.game.level_tick + 2) % 100_000_001;
    let source_dy = 1 + i32::from(next_clock.is_multiple_of(5));
    let body_y = (167.0 + f64::from(source_dy)).min(168.0);
    assert_eq!(body_y, 168.0);
    assert_eq!(110.0 + 5.0, 115.0);
    assert_eq!(
        (body_y + if lava { 0.05 } else { 0.0 } + 8.0).round_ties_even(),
        176.0
    );
    assert!((160.0..176.0).contains(&(body_y + 0.05 - 5.0)));
    let rng = serde_json::to_value(&table.game.rng).unwrap();
    let next_id = table.game.world.next_id;
    neutral(table);
    pending_bet(table);
    if lava {
        assert!(
            table.game.world.entity(table.a).is_none(),
            "Lava retirement remains final"
        );
        assert_eq!(
            table
                .game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oDice")
                .count(),
            1
        );
    } else {
        let die = table
            .game
            .world
            .entity(table.a)
            .expect("healthy control retains live die");
        assert_eq!(die.pos, Vec2::new(115.0, 168.0));
        assert_eq!((die.value("rolled"), die.value("rolling")), (1.0, 0.0));
        assert_eq!(die.vel.y, 0.0);
    }
    assert_eq!(
        table.game.world.next_id, next_id,
        "unheld Dice Destroy has no actor births"
    );
    assert_eq!(
        serde_json::to_value(&table.game.rng).unwrap(),
        rng,
        "final zero-velocity/slow-ground branch has no face draw; earlier keeper already stepped"
    );
    assert_eq!(thief_messages(&table.game), usize::from(rerolled));
    assert_eq!(
        table.game.data.get("thiefLevel").copied().unwrap_or(0.0),
        if rerolled { 2.0 } else { 0.0 }
    );
    let keeper = table.game.world.entity(table.keeper).unwrap();
    assert_eq!(
        (keeper.state, keeper.health),
        (if rerolled { 2 } else { 0 }, 20)
    );
    assert_eq!(keeper.value("angered"), 0.0, "ATTACK latches on next frame");
    if rerolled {
        assert!(table.game.events.iter().any(|event| matches!(event,
            GameEvent::Dialogue(message) if message.first == "COME BACK HERE, THIEF!"
                && message.second.is_empty() && message.ticks == 80
        )));
    }
}

#[test]
fn real_paid_gravity_roll_and_flat_reroll_keep_anger_after_lava_retirement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut table = table(rules, true, true);
        pay_and_establish_first_roll(&mut table, true);
        pick_up_and_begin_flat_reroll(&mut table, 1.0);
        final_step(&mut table, true, true);
    }
}

#[test]
fn real_paid_gravity_roll_and_dry_reroll_keep_live_anger_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut table = table(rules, false, true);
        pay_and_establish_first_roll(&mut table, true);
        pick_up_and_begin_flat_reroll(&mut table, 1.0);
        final_step(&mut table, false, true);
    }
}

#[test]
fn real_paid_first_flat_throw_into_lava_does_not_report_cheating() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut table = table(rules, true, false);
        pay_and_establish_first_roll(&mut table, false);
        pick_up_and_begin_flat_reroll(&mut table, 0.0);
        final_step(&mut table, true, false);
    }
}
