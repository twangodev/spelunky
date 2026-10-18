//! Independent outer-selector statement oracles. These intentionally stop before
//! template alternatives, obstacle expansion, and Create callbacks. Original
//! script anchors below refer to references/classic-1.1/Scripts/Level Generation.
//! Public canonical Rng supplies draws; no independent GM8 arithmetic is claimed.
use super::{
    Features, GenerationContext, GenerationState, generate_campaign_result,
    selectors::{self, Context, Selection},
};
use crate::{Area, Game, Rng, RulesPreset, Vec2, World};
use std::collections::BTreeSet;

const SCRIPTS: [&str; 7] = [
    "scrRoomGen",
    "scrRoomGen2",
    "scrRoomGen3",
    "scrRoomGen4",
    "scrRoomGen5",
    "scrRoomGenMarket",
    "scrRoomGenYeti",
];

#[derive(Clone, Copy, Debug)]
struct Input<'a> {
    script: &'a str,
    path: u8,
    above: u8,
    start: bool,
    end: bool,
    col: usize,
    row: usize,
    level: u32,
    king: bool,
}
impl<'a> Input<'a> {
    fn new(script: &'a str, path: u8) -> Self {
        Self {
            script,
            path,
            above: 0,
            start: false,
            end: false,
            col: 1,
            row: 1,
            level: 2,
            king: false,
        }
    }
    fn context<'b>(self, flags: &'b BTreeSet<String>, world: &'b World) -> Context<'b>
    where
        'a: 'b,
    {
        Context {
            script_name: self.script,
            path: self.path,
            above: self.above,
            start: self.start,
            end: self.end,
            col: self.col,
            row: self.row,
            level: self.level,
            flags,
            world,
        }
    }
}

struct Program {
    rng: Rng,
    calls: Vec<(u32, u32)>,
}
impl Program {
    fn new(rng: &Rng) -> Self {
        Self {
            rng: rng.clone(),
            calls: Vec::new(),
        }
    }
    fn draw(&mut self, lo: u32, hi: u32) -> u32 {
        self.calls.push((lo, hi));
        lo + self.rng.range(hi - lo + 1)
    }
}
fn room(kind: &str, number: u32) -> Selection {
    Selection {
        kind: kind.into(),
        number,
        shop: "General",
    }
}
fn shop(path: u8, n: u32, f: &mut Features) -> Selection {
    if n == 7 {
        f.damsel = true;
    }
    Selection {
        kind: if path == 4 { "shop_left" } else { "shop_right" }.into(),
        number: if n < 6 { 0 } else { n },
        shop: style(n),
    }
}
fn style(n: u32) -> &'static str {
    match n {
        1 => "General",
        2 => "Bomb",
        3 => "Weapon",
        4 => "Rare",
        5 => "Clothing",
        6 => "Craps",
        7 => "Kissing",
        _ => panic!("bad source style"),
    }
}
fn flag(flags: &BTreeSet<String>, names: &[&str]) -> bool {
    names.iter().any(|name| flags.contains(*name))
}
fn same_rng(actual: &Rng, expected: &Rng) {
    assert_eq!(
        (actual.algorithm(), actual.state_bits()),
        (expected.algorithm(), expected.state_bits())
    );
}

// scrRoomGen:54-238. The level/altar predicates do not suppress rand(1,16).
fn mines(p: &mut Program, i: Input<'_>, f: &mut Features) -> Selection {
    if i.start {
        return room(
            "start",
            if i.path == 2 {
                p.draw(5, 8)
            } else {
                p.draw(1, 4)
            },
        );
    }
    if i.end {
        return room(
            "end",
            if i.above == 2 {
                p.draw(2, 4)
            } else {
                p.draw(3, 6)
            },
        );
    }
    if i.path == 0 {
        let altar = p.draw(1, 16);
        if i.level > 1 && !f.altar && altar == 1 {
            f.altar = true;
            return room("side", 11);
        }
        let n = if f.idol || i.row == 3 {
            p.draw(1, 9)
        } else {
            p.draw(1, 10)
        };
        if n == 10 {
            f.idol = true;
        }
        return room("side", n);
    }
    match i.path {
        1 => room("main", p.draw(1, 12)),
        3 => room("landing", p.draw(1, 8)),
        4 | 5 => shop(i.path, p.draw(1, 7), f),
        8 => room("special_8", p.draw(1, 1)),
        9 => room("special_9", p.draw(1, 1)),
        _ => room(
            "drop",
            if i.path == 7 {
                p.draw(4, 12)
            } else if i.above != 2 {
                p.draw(1, 12)
            } else {
                p.draw(1, 8)
            },
        ),
    }
}
// scrRoomGen2:53-244. The side gate runs on every reached non-start/end room.
fn jungle(p: &mut Program, i: Input<'_>, f: &mut Features, flags: &BTreeSet<String>) -> Selection {
    if i.start {
        return room(
            "start",
            if i.path == 2 {
                p.draw(3, 4)
            } else {
                p.draw(1, 2)
            },
        );
    }
    if i.end {
        return room(
            "end",
            if flag(flags, &["lake"]) {
                p.draw(5, 5)
            } else if i.above == 2 {
                p.draw(1, 2)
            } else {
                p.draw(3, 4)
            },
        );
    }
    let gate = p.draw(1, 3);
    if i.path == 0 && gate <= 2 {
        let altar = p.draw(1, 12);
        if !f.altar && altar == 1 {
            f.altar = true;
            return room("side", 10);
        }
        let n = if f.idol { p.draw(1, 8) } else { p.draw(1, 9) };
        if n == 9 {
            f.idol = true;
        }
        return room("side", n);
    }
    match i.path {
        0 | 1 => room("main", p.draw(1, 10)),
        3 => room("landing", p.draw(1, 7)),
        4 | 5 => shop(i.path, p.draw(1, 7), f),
        7 => room("special_7", p.draw(1, 8)),
        8 => room(
            "special_8",
            if i.above == 2 {
                p.draw(1, 5)
            } else {
                p.draw(1, 8)
            },
        ),
        9 => room("special_9", p.draw(1, 1)),
        _ => room(
            "drop",
            if i.above != 2 {
                p.draw(1, 6)
            } else {
                p.draw(1, 5)
            },
        ),
    }
}
// scrRoomGen3:50-190. Both side2 and main10 operands are eager.
fn ice(p: &mut Program, i: Input<'_>, f: &mut Features) -> Selection {
    if i.start {
        return room(
            "start",
            if i.path == 2 {
                p.draw(2, 2)
            } else {
                p.draw(1, 1)
            },
        );
    }
    if i.end {
        return room("end", p.draw(1, 1));
    }
    let side = p.draw(1, 2);
    if i.path == 0 && side == 1 {
        let altar = p.draw(1, 12);
        if !f.altar && altar == 1 {
            f.altar = true;
            return room("side", 10);
        }
        let n = if f.idol { p.draw(1, 8) } else { p.draw(1, 9) };
        if n == 9 {
            f.idol = true;
        }
        return room("side", n);
    }
    let main = p.draw(1, 10);
    if (i.path == 0 || i.path == 1 || i.path == 2) && main < 10 {
        return room("main", p.draw(1, 9));
    }
    match i.path {
        4 | 5 => shop(i.path, p.draw(1, 7), f),
        6 => room("special_6", p.draw(1, 2)),
        7 => room("special_7", p.draw(1, 1)),
        8 => room("special_8", p.draw(1, 1)),
        9 => room("special_9", p.draw(1, 1)),
        _ => room("drop", p.draw(1, 1)),
    }
}
// scrRoomGen4:50-216. City branch bypasses altar; ordinary main has 10 cases.
fn temple(p: &mut Program, i: Input<'_>, f: &mut Features, flags: &BTreeSet<String>) -> Selection {
    if i.start {
        return room(
            "start",
            if i.path == 2 {
                p.draw(2, 2)
            } else {
                p.draw(1, 1)
            },
        );
    }
    if i.end {
        return room("end", p.draw(1, 1));
    }
    let side = p.draw(1, 4);
    let city = flag(flags, &["cityOfGold", "city_of_gold"]);
    if i.path == 0 && side > 1 {
        if city {
            let n = p.draw(1, 14);
            return room("side", if n == 12 { 15 } else { n });
        }
        let altar = p.draw(1, 12);
        if !f.altar && altar == 1 {
            f.altar = true;
            return room("side", 16);
        }
        let n = if f.idol { p.draw(1, 11) } else { p.draw(1, 12) };
        if n == 12 {
            f.idol = true;
        }
        return room("side", n);
    }
    match i.path {
        0 | 1 => room("main", if city { p.draw(1, 12) } else { p.draw(1, 10) }),
        3 => room("landing", p.draw(1, 4)),
        4 | 5 => shop(
            i.path,
            if f.damsel { p.draw(1, 6) } else { p.draw(1, 7) },
            f,
        ),
        6 => room("special_6", 0),
        7 => room("special_7", 0),
        8 => room("special_8", 0),
        9 => room("special_9", 0),
        _ => room("drop", p.draw(1, 8)),
    }
}
// scrRoomGenMarket:53-162. Literal while body checks its cycle after acceptance.
fn market(p: &mut Program, i: Input<'_>, f: &mut Features) -> Selection {
    if i.start {
        return room(
            "start",
            if i.path == 2 {
                p.draw(3, 4)
            } else {
                p.draw(1, 2)
            },
        );
    }
    if i.end {
        return room(
            "end",
            if i.above == 2 {
                p.draw(1, 2)
            } else {
                p.draw(3, 4)
            },
        );
    }
    if i.path == 1 {
        return room("main", p.draw(1, 8));
    }
    if i.path == 3 {
        return room("landing", p.draw(1, 7));
    }
    if i.path == 4 {
        let name = if i.col == 3 {
            "Ankh"
        } else {
            let mut n = p.draw(1, 5);
            let m = n;
            let mut name = "";
            while name.is_empty() {
                if !f.market_shops.contains(&n) {
                    name = style(n);
                    f.market_shops.insert(n);
                }
                n += 1;
                if n > 5 {
                    n = 1;
                }
                if n == m {
                    name = "General";
                    break;
                }
            }
            name
        };
        return Selection {
            kind: "shop_left".into(),
            number: 0,
            shop: name,
        };
    }
    if i.path == 5 {
        return Selection {
            kind: "shop_right".into(),
            number: 0,
            shop: "Craps",
        };
    }
    room(
        "drop",
        if i.above != 2 {
            p.draw(1, 6)
        } else {
            p.draw(1, 5)
        },
    )
}
// scrRoomGenYeti:53-212. Depth controls assignment; it does not short-circuit RNG.
fn yeti(p: &mut Program, i: Input<'_>, f: &mut Features) -> Selection {
    if i.start {
        return room(
            "start",
            if i.path == 2 {
                p.draw(3, 4)
            } else {
                p.draw(1, 2)
            },
        );
    }
    if i.end {
        return room(
            "end",
            if i.above == 2 {
                p.draw(1, 4)
            } else {
                p.draw(3, 6)
            },
        );
    }
    let side = p.draw(1, 4);
    if i.path == 0 && side < 4 {
        return room(
            "side",
            if f.idol || i.row == 3 {
                p.draw(1, 2)
            } else {
                p.draw(1, 1)
            },
        );
    }
    if i.path == 0 || i.path == 1 {
        if i.king {
            return room("main", p.draw(1, 9));
        }
        let mut n = p.draw(1, 10);
        let y = 16 + 128 * i.row;
        if y > 384 {
            n = 10;
        } else {
            let half = p.draw(1, 2);
            if y > 256 && half == 1 {
                n = 10;
            } else {
                let third = p.draw(1, 3);
                if y > 128 && third == 1 {
                    n = 10;
                }
            }
        }
        return room("main", n);
    }
    match i.path {
        3 => room("landing", p.draw(1, 9)),
        4 | 5 => shop(i.path, p.draw(1, 7), f),
        8 => room("special_8", p.draw(1, 1)),
        9 => room("special_9", p.draw(1, 1)),
        _ => room(
            "drop",
            if i.path == 7 {
                p.draw(4, 12)
            } else if i.above != 2 {
                p.draw(1, 12)
            } else {
                p.draw(1, 8)
            },
        ),
    }
}
fn oracle(p: &mut Program, i: Input<'_>, f: &mut Features, flags: &BTreeSet<String>) -> Selection {
    match i.script {
        "scrRoomGen" => mines(p, i, f),
        "scrRoomGen2" => jungle(p, i, f, flags),
        "scrRoomGen3" => ice(p, i, f),
        "scrRoomGen4" => temple(p, i, f, flags),
        "scrRoomGenMarket" => market(p, i, f),
        "scrRoomGenYeti" => yeti(p, i, f),
        // scrRoomGen5:50-73; row3 is stamped at y592 by its caller.
        "scrRoomGen5" => room(
            if i.row < 3 {
                "boss_upper"
            } else {
                "boss_lower"
            },
            p.draw(1, 6),
        ),
        _ => panic!("unknown original script"),
    }
}
fn check(
    seed: u32,
    i: Input<'_>,
    flags: &BTreeSet<String>,
    initial: &Features,
    world: &World,
) -> (Selection, Program, Features) {
    let mut actual_rng = Rng::new_canonical(seed);
    let mut expected = Program::new(&actual_rng);
    let mut actual_features = initial.clone();
    let mut expected_features = initial.clone();
    let wanted = oracle(&mut expected, i, &mut expected_features, flags);
    let actual = selectors::select(
        &mut actual_rng,
        i.context(flags, world),
        &mut actual_features,
    );
    assert_eq!(actual, wanted, "seed {seed}, input {i:?}");
    assert_eq!(
        actual_features, expected_features,
        "seed {seed}, input {i:?}"
    );
    same_rng(&actual_rng, &expected.rng);
    // An additional public draw observes continuation beyond the phase boundary.
    let mut continuation = expected.rng.clone();
    assert_eq!(actual_rng.range(251), continuation.range(251));
    (wanted, expected, expected_features)
}

#[test]
fn source_all_seven_outer_scripts_eager_guards_and_fallbacks() {
    let world = World::new(42, 34, Area::Mines, 2);
    let flags = BTreeSet::new();
    // Every case is checked; no source or production success filtering.
    for script in SCRIPTS {
        for seed in 0..64 {
            for path in 0..=10 {
                for above in [0, 2] {
                    for latched in [false, true] {
                        let f = Features {
                            idol: latched,
                            altar: latched,
                            damsel: latched,
                            ..Features::default()
                        };
                        let mut i = Input::new(script, path);
                        i.above = above;
                        check(seed, i, &flags, &f, &world);
                    }
                }
            }
        }
    }
}

#[test]
fn source_start_end_precedence_and_fixed_cardinality_calls() {
    let world = World::new(42, 34, Area::Mines, 2);
    for seed in 0..64 {
        for script in SCRIPTS {
            for path in [0, 2, 4] {
                for above in [0, 2] {
                    for (start, end) in [(true, false), (false, true), (true, true)] {
                        let mut i = Input::new(script, path);
                        i.start = start;
                        i.end = end;
                        i.above = above;
                        let (_, p, _) =
                            check(seed, i, &BTreeSet::new(), &Features::default(), &world);
                        assert_eq!(p.calls.len(), 1, "start/end must bypass all later guards");
                    }
                }
            }
        }
    }
    for seed in 0..64 {
        let mut i = Input::new("scrRoomGen2", 4);
        i.end = true;
        let (_, p, _) = check(
            seed,
            i,
            &BTreeSet::from(["lake".into()]),
            &Features::default(),
            &world,
        );
        assert_eq!(p.calls, [(5, 5)]);
    }
}

#[test]
fn source_ineligible_altar_still_draws_and_yeti_side_draws_one_to_one() {
    let world = World::new(42, 34, Area::Mines, 1);
    for seed in 0..128 {
        let mut i = Input::new("scrRoomGen", 0);
        i.level = 1;
        let (_, p, _) = check(seed, i, &BTreeSet::new(), &Features::default(), &world);
        assert_eq!(p.calls[0], (1, 16));
        assert_eq!(p.calls.len(), 2);
        for script in ["scrRoomGen2", "scrRoomGen3", "scrRoomGen4"] {
            let f = Features {
                altar: true,
                ..Features::default()
            };
            check(seed, Input::new(script, 0), &BTreeSet::new(), &f, &world);
        }
        let i = Input::new("scrRoomGenYeti", 0);
        let (s, p, _) = check(seed, i, &BTreeSet::new(), &Features::default(), &world);
        if s.kind == "side" {
            assert_eq!(p.calls, [(1, 4), (1, 1)]);
        }
    }
}

#[test]
fn source_temple_city_main_bounds_and_literal_sacrifice_rooms() {
    let world = World::new(42, 34, Area::Temple, 13);
    for seed in 0..128 {
        for city in [false, true] {
            let flags = if city {
                BTreeSet::from(["cityOfGold".into()])
            } else {
                BTreeSet::new()
            };
            for path in [0, 1, 6, 7, 8, 9] {
                let i = Input::new("scrRoomGen4", path);
                let (s, p, _) = check(seed, i, &flags, &Features::default(), &world);
                if path == 1 {
                    assert_eq!(p.calls, [(1, 4), (1, if city { 12 } else { 10 })]);
                }
                if path >= 6 {
                    assert_eq!(s.number, 0);
                    assert_eq!(p.calls, [(1, 4)]);
                }
            }
        }
    }
}

#[test]
fn source_damsel_latch_limits_temple_only_and_survives_later_shops() {
    let world = World::new(42, 34, Area::Temple, 13);
    let flags = BTreeSet::new();
    let mut kissing = 0;
    for seed in 0..128 {
        for script in [
            "scrRoomGen",
            "scrRoomGen2",
            "scrRoomGen3",
            "scrRoomGen4",
            "scrRoomGenYeti",
        ] {
            let mut rng = Rng::new_canonical(seed);
            let mut p = Program::new(&rng);
            let mut f = Features::default();
            let mut expected = f.clone();
            for path in [4, 5, 4, 5, 4, 5, 4, 5] {
                let i = Input::new(script, path);
                let wanted = oracle(&mut p, i, &mut expected, &flags);
                let actual = selectors::select(&mut rng, i.context(&flags, &world), &mut f);
                assert_eq!(actual, wanted);
                assert_eq!(f, expected);
                same_rng(&rng, &p.rng);
                kissing += usize::from(actual.shop == "Kissing");
            }
            // scrLevelGen:86-87 pre-seeds idol+damsel for sacrifice pit.
            let preseed = Features {
                idol: true,
                damsel: true,
                ..Features::default()
            };
            let (_, calls, after) = check(seed, Input::new(script, 4), &flags, &preseed, &world);
            assert!(after.damsel);
            assert_eq!(
                calls.calls.last(),
                Some(&(1, if script == "scrRoomGen4" { 6 } else { 7 }))
            );
        }
    }
    assert!(
        kissing > 0,
        "the complete fixed seed matrix must exercise the latch"
    );
}

#[test]
fn source_market_ankh_casino_no_draw_and_fifth_cycle_override() {
    let world = World::new(42, 34, Area::Jungle, 5);
    let flags = BTreeSet::new();
    for seed in 0..128 {
        let mut i = Input::new("scrRoomGenMarket", 4);
        i.col = 3;
        let f = Features {
            market_shops: BTreeSet::from([2, 4]),
            ..Features::default()
        };
        let (s, p, after) = check(seed, i, &flags, &f, &world);
        assert_eq!(s.shop, "Ankh");
        assert!(p.calls.is_empty());
        assert_eq!(after, f);
        i.path = 5;
        let (s, p, after) = check(seed, i, &flags, &f, &world);
        assert_eq!(s.shop, "Craps");
        assert!(p.calls.is_empty());
        assert_eq!(after, f);
        // All five possible initial positions, with only the fifth visited style fresh.
        for first in 1..=5 {
            let fresh = if first == 1 { 5 } else { first - 1 };
            let f = Features {
                market_shops: (1..=5).filter(|n| *n != fresh).collect(),
                ..Features::default()
            };
            i.path = 4;
            i.col = 1;
            let (s, p, after) = check(seed, i, &flags, &f, &world);
            let mut probe = Rng::new_canonical(seed);
            let drawn = 1 + probe.range(5);
            // Assert every case; a fixed matrix contains all initial positions.
            if drawn == first {
                assert_eq!(s.shop, "General");
                assert_eq!(after.market_shops.len(), 5);
            }
            assert_eq!(p.calls, [(1, 5)]);
        }
        let f = Features {
            market_shops: (1..=5).collect(),
            ..Features::default()
        };
        i.path = 4;
        i.col = 1;
        assert_eq!(check(seed, i, &flags, &f, &world).0.shop, "General");
    }
}

#[test]
fn source_yeti_depth_eager_half_third_and_live_king_queries() {
    for status in 0..6 {
        let mut world = World::new(42, 38, Area::Ice, 9);
        if status != 0 {
            let first = world.spawn("oYetiKing", Vec2::new(96.0, 96.0));
            if status == 2 || status == 5 {
                world.entity_mut(first).unwrap().engine_active = false;
            }
            if status == 3 || status == 4 {
                world.entity_mut(first).unwrap().alive = false;
            }
            if status == 4 || status == 5 {
                world.spawn("oYetiKing", Vec2::new(256.0, 384.0));
            }
        }
        for row in 0..4 {
            for seed in 0..128 {
                let mut i = Input::new("scrRoomGenYeti", 1);
                i.row = row;
                i.king = status == 1 || status == 4 || status == 5;
                let (s, p, _) = check(seed, i, &BTreeSet::new(), &Features::default(), &world);
                assert_eq!(p.calls[0], (1, 4)); // eager side test, even on path1
                if i.king {
                    assert_eq!(p.calls, [(1, 4), (1, 9)]);
                } else if row == 3 {
                    assert_eq!(p.calls, [(1, 4), (1, 10)]);
                    assert_eq!(s.number, 10);
                } else {
                    assert_eq!(p.calls[1..3], [(1, 10), (1, 2)]);
                    if row < 2 {
                        assert_eq!(p.calls[3], (1, 3));
                    } else {
                        assert!(p.calls.len() == 3 || p.calls.len() == 4);
                    }
                }
            }
        }
    }
}

#[test]
fn source_selector_trace_at_actual_next_level_boundary() {
    // Exercise ordinary transitions through the real Game API. The Market
    // scenario uses the persistent genBlackMarket request; per-room special
    // flags are cleared at this boundary. Pit forcing is covered separately
    // by the target-level generation entry point below.
    // Trace RNG starts after previous room stamping and ends before alternatives.
    let scenarios = [
        (1, false),
        (5, false),
        (9, false),
        (13, false),
        (16, false),
        (5, true),
    ];
    for (level, request_black_market) in scenarios {
        for seed in [0, 1, 42, 127] {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.world.level = level - 1;
            game.flags.insert("no_dark_level".into());
            if request_black_market {
                // This run-persistent source flag survives next_level's
                // per-room cleanup and selects the target Jungle layout.
                game.flags.insert("genBlackMarket".into());
            }
            selectors::begin_trace();
            game.next_level();
            if request_black_market {
                assert_eq!(game.world.name, "Black Market");
            }
            let rows = selectors::take_trace();
            assert!(!rows.is_empty(), "next_level must reach the real selector");
            if request_black_market {
                assert!(rows.iter().all(|t| t.script_name == "scrRoomGenMarket"));
            }
            for t in rows {
                let i = Input {
                    script: &t.script_name,
                    path: t.path,
                    above: t.above,
                    start: t.start,
                    end: t.end,
                    col: t.col,
                    row: t.row,
                    level: t.level,
                    king: t.king_present,
                };
                let mut p = Program::new(&t.rng_before);
                let mut f = t.features_before.clone();
                assert_eq!(t.selection, oracle(&mut p, i, &mut f, &t.flags));
                same_rng(&t.rng_after, &p.rng);
                assert_eq!(t.features_after, f);
            }
        }
    }
}

#[test]
fn source_selector_trace_for_selected_target_level_specials() {
    // The generation entry point receives a selected special destination for
    // the target room. Setting the flag before Game::next_level would be stale:
    // that transition clears specials belonging to the previous room first.
    for (level, flag, expected_name, expected_script) in [
        (9, "yeti_lair", "Yeti Lair", "scrRoomGenYeti"),
        (14, "sacrifice_pit", "Sacrifice Pit", "scrRoomGen4"),
    ] {
        let mut rng = Rng::new(42);
        let mut state = GenerationState::fresh(&rng);
        let flags = BTreeSet::from(["no_dark_level".into(), flag.into()]);
        selectors::begin_trace();
        let generated = generate_campaign_result(
            &mut rng,
            level,
            &flags,
            GenerationContext::default(),
            &mut state,
        );
        let rows = selectors::take_trace();
        assert_eq!(generated.world.name, expected_name);
        assert!(
            !rows.is_empty(),
            "target-level generation must reach selectors"
        );

        for t in rows {
            assert_eq!(t.script_name, expected_script);
            if flag == "sacrifice_pit" {
                assert!(t.features_before.idol && t.features_before.damsel);
            }
            let i = Input {
                script: &t.script_name,
                path: t.path,
                above: t.above,
                start: t.start,
                end: t.end,
                col: t.col,
                row: t.row,
                level: t.level,
                king: t.king_present,
            };
            let mut p = Program::new(&t.rng_before);
            let mut f = t.features_before.clone();
            assert_eq!(t.selection, oracle(&mut p, i, &mut f, &t.flags));
            same_rng(&t.rng_after, &p.rng);
            assert_eq!(t.features_after, f);
        }
    }
}

#[test]
fn source_lamp_uses_shared_damsel_after_shop_style_changes() {
    // scrRoomGen:430, scrRoomGen2:445, scrRoomGen3:372,
    // scrRoomGenYeti:391 use oGame.damsel; Temple/Market keep native Lamp.
    for (area, name, red) in [
        (Area::Mines, "", true),
        (Area::Jungle, "", true),
        (Area::Ice, "", true),
        (Area::Ice, "Yeti Lair", true),
        (Area::Temple, "", false),
        (Area::Jungle, "Black Market", false),
    ] {
        for latch in [false, true] {
            let mut world = World::new(42, 38, area, 2);
            world.name = name.into();
            let mut rng = Rng::new_canonical(42);
            let template = format!("l{}", "0".repeat(79));
            super::tiles::stamp_with_damsel(
                &mut world, &mut rng, &template, 1, 1, false, false, "General", latch,
            );
            let lamps: Vec<_> = world
                .entities
                .iter()
                .filter(|e| e.alive && (e.object == "oLamp" || e.object == "oLampRed"))
                .collect();
            assert_eq!(lamps.len(), 1);
            assert_eq!(
                lamps[0].object,
                if latch && red { "oLampRed" } else { "oLamp" }
            );
        }
    }
}
