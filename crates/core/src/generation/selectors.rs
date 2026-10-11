//! SourceClassicV2 outer room-selector phase, before template alternatives,
//! obstacle expansion and instance stamping. HistoricalV1 keeps its original
//! adapter. This does not certify those later phases or original birth order.
use super::{Features, has, rand};
use crate::{Rng, World};
use std::collections::BTreeSet;

pub(super) struct Context<'a> {
    pub script_name: &'a str,
    pub path: u8,
    pub above: u8,
    pub start: bool,
    pub end: bool,
    pub col: usize,
    pub row: usize,
    pub level: u32,
    pub flags: &'a BTreeSet<String>,
    pub world: &'a World,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Selection {
    pub kind: String,
    pub number: u32,
    pub shop: &'static str,
}
fn room(kind: &str, number: u32) -> Selection {
    Selection {
        kind: kind.into(),
        number,
        shop: "General",
    }
}
fn shop_name(number: u32) -> &'static str {
    match number {
        2 => "Bomb",
        3 => "Weapon",
        4 => "Rare",
        5 => "Clothing",
        6 => "Craps",
        7 => "Kissing",
        _ => "General",
    }
}
fn king_exists(world: &World) -> bool {
    world
        .entities
        .iter()
        .any(|e| e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oYetiKing"))
}

pub(super) fn select(rng: &mut Rng, c: Context<'_>, f: &mut Features) -> Selection {
    #[cfg(test)]
    let before = trace_before(rng, &c, f);
    let selection = select_inner(rng, &c, f);
    #[cfg(test)]
    trace_after(rng, before, &selection, f);
    selection
}
fn select_inner(rng: &mut Rng, c: &Context<'_>, f: &mut Features) -> Selection {
    let mines = c.script_name == "scrRoomGen";
    let jungle = c.script_name == "scrRoomGen2";
    let ice = c.script_name == "scrRoomGen3";
    let temple = c.script_name == "scrRoomGen4";
    let market = c.script_name == "scrRoomGenMarket";
    let yeti = c.script_name == "scrRoomGenYeti";
    if c.script_name == "scrRoomGen5" {
        // scrRoomGen5:50-73; no common side/main guard in this script.
        return room(
            if c.row < 3 {
                "boss_upper"
            } else {
                "boss_lower"
            },
            rand(rng, 1, 6),
        );
    }
    assert!(mines || jungle || ice || temple || market || yeti);
    if c.start {
        // Fixed-cardinality rand calls are still source calls (Ice/Temple52-53).
        let (lo, hi) = if mines {
            if c.path == 2 { (5, 8) } else { (1, 4) }
        } else if ice || temple {
            if c.path == 2 { (2, 2) } else { (1, 1) }
        } else if c.path == 2 {
            (3, 4)
        } else {
            (1, 2)
        };
        return room("start", rand(rng, lo, hi));
    }
    if c.end {
        let (lo, hi) = if jungle && has(c.flags, "lake") {
            (5, 5) // scrRoomGen2:68, not a constant assignment.
        } else if mines {
            if c.above == 2 { (2, 4) } else { (3, 6) }
        } else if ice || temple {
            (1, 1)
        } else if yeti {
            if c.above == 2 { (1, 4) } else { (3, 6) }
        } else if c.above == 2 {
            (1, 2)
        } else {
            (3, 4)
        };
        return room("end", rand(rng, lo, hi));
    }
    // Original GM8 evaluates both logical operands. Each applicable side roll
    // runs even when path != 0; only earlier start/end branches bypass it.
    let side = if mines {
        c.path == 0
    } else if jungle {
        let n = rand(rng, 1, 3);
        c.path == 0 && n <= 2
    } else if ice {
        let n = rand(rng, 1, 2);
        c.path == 0 && n == 1
    } else if temple {
        let n = rand(rng, 1, 4);
        c.path == 0 && n > 1
    } else if yeti {
        let n = rand(rng, 1, 4);
        c.path == 0 && n < 4
    } else {
        false
    };
    if side {
        let number = if yeti {
            rand(rng, 1, if f.idol || c.row == 3 { 2 } else { 1 })
        } else if temple && has(c.flags, "city_of_gold") {
            let n = rand(rng, 1, 14);
            if n == 12 { 15 } else { n }
        } else {
            let altar_roll = rand(rng, 1, if mines { 16 } else { 12 });
            if (!mines || c.level > 1) && !f.altar && altar_roll == 1 {
                f.altar = true;
                if mines {
                    11
                } else if temple {
                    16
                } else {
                    10
                }
            } else {
                let upper = if mines {
                    if f.idol || c.row == 3 { 9 } else { 10 }
                } else if temple {
                    if f.idol { 11 } else { 12 }
                } else if f.idol {
                    8
                } else {
                    9
                };
                let n = rand(rng, 1, upper);
                if (mines && n == 10) || (temple && n == 12) || (!mines && !temple && n == 9) {
                    f.idol = true;
                }
                n
            }
        };
        return room("side", number);
    }
    if ice {
        // scrRoomGen3:105: the random operand is evaluated even for shop,
        // special and landing paths which fail the other operand.
        let n = rand(rng, 1, 10);
        if matches!(c.path, 0..=2) && n < 10 {
            return room("main", rand(rng, 1, 9));
        }
    } else if (market && c.path == 1) || (!market && matches!(c.path, 0 | 1)) {
        let number = if yeti {
            if king_exists(c.world) {
                rand(rng, 1, 9)
            } else {
                let mut n = rand(rng, 1, 10);
                let y = 16 + c.row * 128;
                if y > 384 {
                    n = 10;
                } else {
                    let half = rand(rng, 1, 2);
                    if y > 256 && half == 1 {
                        n = 10;
                    } else {
                        let third = rand(rng, 1, 3);
                        if y > 128 && third == 1 {
                            n = 10;
                        }
                    }
                }
                n
            }
        } else {
            rand(
                rng,
                1,
                if mines {
                    12
                } else if temple {
                    if has(c.flags, "city_of_gold") { 12 } else { 10 }
                } else if market {
                    8
                } else {
                    10
                },
            )
        };
        return room("main", number);
    }
    if c.path == 3 && !ice {
        return room(
            "landing",
            rand(
                rng,
                1,
                if mines {
                    8
                } else if yeti {
                    9
                } else if temple {
                    4
                } else {
                    7
                },
            ),
        );
    }
    if matches!(c.path, 4 | 5) {
        let kind = if c.path == 4 {
            "shop_left"
        } else {
            "shop_right"
        };
        if market {
            if c.path == 5 {
                return Selection {
                    kind: kind.into(),
                    number: 0,
                    shop: "Craps",
                };
            }
            // scrRoomGenMarket:113-117: the Ankh branch makes no random call
            // and does not claim any of the five ordinary shop flags.
            if c.col == 3 {
                return Selection {
                    kind: kind.into(),
                    number: 0,
                    shop: "Ankh",
                };
            }
            let first = rand(rng, 1, 5);
            let mut n = first;
            loop {
                let selected = f.market_shops.insert(n);
                let name = shop_name(n);
                n = n % 5 + 1;
                // Original checks wraparound AFTER accepting/flagging a shop.
                // The fifth candidate is overridden to General even if fresh.
                if n == first {
                    return Selection {
                        kind: kind.into(),
                        number: 0,
                        shop: "General",
                    };
                }
                if selected {
                    return Selection {
                        kind: kind.into(),
                        number: 0,
                        shop: name,
                    };
                }
            }
        }
        let n = rand(rng, 1, if temple && f.damsel { 6 } else { 7 });
        if n == 7 {
            f.damsel = true;
        }
        return Selection {
            kind: kind.into(),
            number: if n >= 6 { n } else { 0 },
            shop: shop_name(n),
        };
    }
    if temple && matches!(c.path, 6..=9) {
        // scrRoomGen4:190-205 assigns literals, with no rand(1,1).
        return room(&format!("special_{}", c.path), 0);
    }
    let special = if (mines || yeti) && matches!(c.path, 8 | 9) {
        Some(1)
    } else if jungle && c.path == 7 {
        Some(8)
    } else if jungle && c.path == 8 {
        Some(if c.above == 2 { 5 } else { 8 })
    } else if jungle && c.path == 9 {
        Some(1)
    } else if ice && c.path == 6 {
        Some(2)
    } else if ice && matches!(c.path, 7..=9) {
        Some(1)
    } else {
        None
    };
    if let Some(high) = special {
        return room(&format!("special_{}", c.path), rand(rng, 1, high));
    }
    let (lo, hi) = if mines || yeti {
        if c.path == 7 {
            (4, 12)
        } else if c.above != 2 {
            (1, 12)
        } else {
            (1, 8)
        }
    } else if temple {
        (1, 8)
    } else if ice {
        (1, 1)
    } else if c.above != 2 {
        (1, 6)
    } else {
        (1, 5)
    };
    room("drop", rand(rng, lo, hi))
}

// Test-only opt-in snapshots record this phase boundary. They consume no RNG,
// allocate nothing when disabled and do not alter production/serialized state.
#[cfg(test)]
#[derive(Clone, Debug)]
pub(super) struct Trace {
    pub script_name: String,
    pub path: u8,
    pub above: u8,
    pub start: bool,
    pub end: bool,
    pub col: usize,
    pub row: usize,
    pub level: u32,
    pub flags: BTreeSet<String>,
    pub king_present: bool,
    pub features_before: Features,
    pub features_after: Features,
    pub rng_before: Rng,
    pub rng_after: Rng,
    pub selection: Selection,
}
#[cfg(test)]
std::thread_local! {
    static TRACE: std::cell::RefCell<Option<Vec<Trace>>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
pub(super) fn begin_trace() {
    TRACE.with(|t| *t.borrow_mut() = Some(Vec::new()));
}
#[cfg(test)]
pub(super) fn take_trace() -> Vec<Trace> {
    TRACE.with(|t| t.borrow_mut().take().unwrap_or_default())
}
#[cfg(test)]
fn trace_before(rng: &Rng, c: &Context<'_>, f: &Features) -> Option<Trace> {
    TRACE.with(|t| {
        t.borrow().as_ref().map(|_| Trace {
            script_name: c.script_name.into(),
            path: c.path,
            above: c.above,
            start: c.start,
            end: c.end,
            col: c.col,
            row: c.row,
            level: c.level,
            flags: c.flags.clone(),
            king_present: king_exists(c.world),
            features_before: f.clone(),
            features_after: f.clone(),
            rng_before: rng.clone(),
            rng_after: rng.clone(),
            selection: room("", 0),
        })
    })
}
#[cfg(test)]
fn trace_after(rng: &Rng, before: Option<Trace>, selection: &Selection, f: &Features) {
    if let Some(mut trace) = before {
        trace.rng_after = rng.clone();
        trace.features_after = f.clone();
        trace.selection = selection.clone();
        TRACE.with(|t| {
            if let Some(rows) = t.borrow_mut().as_mut() {
                rows.push(trace);
            }
        });
    }
}
