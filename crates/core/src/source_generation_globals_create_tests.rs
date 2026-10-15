use super::*;
use crate::source_generation_clear_globals::Globals;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Number(f64),
    Text(&'static str),
}
#[derive(Debug, Clone, PartialEq)]
enum Call {
    Attach,
    Declare(&'static str),
    Number(&'static str, f64),
    Text(&'static str, &'static str),
    Get(&'static str),
    Joystick(u8),
    Score(u8),
    ScoreClear,
    ScoreAdd(&'static str, f64),
}
#[derive(Debug, Clone, PartialEq)]
struct Fault(&'static str);
struct Spy {
    raw: Option<BTreeMap<&'static str, Value>>,
    declared: BTreeSet<&'static str>,
    joysticks: [bool; 2],
    table: Vec<f64>,
    calls: Vec<Call>,
    fail: Option<&'static str>,
    score_calls: usize,
    fail_score_call: Option<usize>,
}
impl Spy {
    fn fresh() -> Self {
        Self {
            raw: None,
            declared: BTreeSet::new(),
            joysticks: [true, true],
            table: vec![0.0; 10],
            calls: Vec::new(),
            fail: None,
            score_calls: 0,
            fail_score_call: None,
        }
    }
    fn values(&mut self) -> Result<&mut BTreeMap<&'static str, Value>, Fault> {
        self.raw.as_mut().ok_or(Fault("unbound"))
    }
    fn n(&self, name: &str) -> f64 {
        match self.raw.as_ref().and_then(|raw| raw.get(name)) {
            Some(Value::Number(value)) => *value,
            other => panic!("required number {name}: {other:?}"),
        }
    }
    fn has(&self, name: &str) -> bool {
        self.raw.as_ref().is_some_and(|raw| raw.contains_key(name))
    }
    fn scores(&self) -> Vec<u8> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                Call::Score(index) => Some(*index),
                _ => None,
            })
            .collect()
    }
}
impl Globals for Spy {
    type Error = Fault;
    fn number(&mut self, name: &'static str) -> Result<f64, Fault> {
        self.calls.push(Call::Get(name));
        match self.values()?.get(name) {
            Some(Value::Number(value)) => Ok(*value),
            _ => Err(Fault("missing-or-wrong-kind")),
        }
    }
    fn set_number(&mut self, name: &'static str, value: f64) -> Result<(), Fault> {
        self.calls.push(Call::Number(name, value));
        if self.fail == Some(name) {
            return Err(Fault(name));
        }
        self.values()?.insert(name, Value::Number(value));
        Ok(())
    }
    fn set_text(&mut self, name: &'static str, value: &'static str) -> Result<(), Fault> {
        self.calls.push(Call::Text(name, value));
        if self.fail == Some(name) {
            return Err(Fault(name));
        }
        self.values()?.insert(name, Value::Text(value));
        Ok(())
    }
}
impl Capabilities for Spy {
    fn attach_globals_receiver(&mut self) -> Result<(), Fault> {
        self.calls.push(Call::Attach);
        if self.fail == Some("attach") {
            return Err(Fault("attach"));
        }
        if self.raw.is_some() {
            return Err(Fault("already-attached"));
        }
        self.raw = Some(BTreeMap::new());
        Ok(())
    }
    fn declare_globalvar(&mut self, name: &'static str) -> Result<(), Fault> {
        self.calls.push(Call::Declare(name));
        self.values()?;
        if self.fail == Some("declare") {
            return Err(Fault("declare"));
        }
        self.declared.insert(name);
        Ok(())
    }
    fn joystick_exists(&mut self, slot: u8) -> Result<bool, Fault> {
        self.calls.push(Call::Joystick(slot));
        if slot == 2 && self.fail == Some("joystick2") {
            return Err(Fault("joystick2"));
        }
        Ok(self.joysticks[usize::from(slot - 1)])
    }
}
impl reset::Highscores for Spy {
    fn highscore_value(&mut self, index: u8) -> Result<f64, Fault> {
        self.calls.push(Call::Score(index));
        self.score_calls += 1;
        if self.fail_score_call == Some(self.score_calls) {
            return Err(Fault("score"));
        }
        self.table
            .get(usize::from(index - 1))
            .copied()
            .ok_or(Fault("score-range"))
    }
    fn highscore_clear(&mut self) -> Result<(), Fault> {
        self.calls.push(Call::ScoreClear);
        self.table = vec![0.0; 10];
        if self.fail == Some("reset-after-clear") {
            return Err(Fault("reset-after-clear"));
        }
        Ok(())
    }
    fn highscore_add(&mut self, name: &'static str, value: f64) -> Result<(), Fault> {
        self.calls.push(Call::ScoreAdd(name, value));
        if self.fail == Some(name) {
            return Err(Fault(name));
        }
        // Configured backend follows the accepted native signed-integer strict
        // insertion for these reset-domain arguments. No generic native host,
        // registry, malformed-file, or arbitrary-f64 sorting claim is made.
        if let Some(at) = self.table.iter().position(|old| *old < value) {
            self.table.insert(at, value);
            self.table.truncate(10);
        }
        Ok(())
    }
}

#[test]
fn attachment_and_declaration_errors_never_invent_a_numeric_joyid() {
    for fail in ["attach", "declare", "joyid"] {
        let mut spy = Spy::fresh();
        spy.fail = Some(fail);
        assert_eq!(create(&mut spy), Err(Error::Capability(Fault(fail))));
        match fail {
            "attach" => {
                assert_eq!(spy.calls, [Call::Attach]);
                assert!(spy.raw.is_none());
            }
            "declare" => {
                assert_eq!(spy.calls, [Call::Attach, Call::Declare("joyid")]);
                assert_eq!(spy.raw, Some(BTreeMap::new()));
                assert!(spy.declared.is_empty());
            }
            _ => {
                assert_eq!(
                    spy.calls,
                    [
                        Call::Attach,
                        Call::Declare("joyid"),
                        Call::Number("joyid", 1.0)
                    ]
                );
                assert!(spy.declared.contains("joyid"));
                assert!(!spy.has("joyid"));
            }
        }
    }
    let mut attached = Spy::fresh();
    attached.raw = Some(BTreeMap::from([("sentinel", Value::Number(99.0))]));
    assert_eq!(
        create(&mut attached),
        Err(Error::Capability(Fault("already-attached")))
    );
    assert_eq!(
        attached.n("sentinel"),
        99.0,
        "attach cannot replace prior namespace"
    );
}

#[test]
fn both_joystick_operands_run_and_second_error_retains_first_assignment() {
    let mut spy = Spy::fresh();
    spy.fail = Some("joystick2");
    assert_eq!(create(&mut spy), Err(Error::Capability(Fault("joystick2"))));
    assert_eq!(
        spy.calls,
        [
            Call::Attach,
            Call::Declare("joyid"),
            Call::Number("joyid", 1.0),
            Call::Joystick(1),
            Call::Joystick(2)
        ]
    );
    assert_eq!(spy.n("joyid"), 1.0);
    assert!(!spy.has("levelSelect"));
    assert!(spy.scores().is_empty());
}

#[test]
fn initial_score9_error_occurs_after_score8_assignment_without_prefetch() {
    let mut spy = Spy::fresh();
    spy.table[7] = 2_012_345.0;
    spy.fail_score_call = Some(2);
    assert_eq!(
        create(&mut spy),
        Err(Error::Score(reset::Error::Capability(Fault("score"))))
    );
    assert_eq!(spy.scores(), [8, 9]);
    assert_eq!(spy.n("tunnel1"), 12_345.0);
    assert!(!spy.has("tunnel2"));
    assert!(!spy.has("mini1"));
    assert_eq!(spy.n("cleanSolids"), 0.0);
    assert!(!spy.has("marketChance"));
    assert_eq!(
        &spy.calls[spy.calls.len() - 3..],
        [
            Call::Score(8),
            Call::Number("tunnel1", 12_345.0),
            Call::Score(9)
        ]
    );
}

#[test]
fn literal_clear_text_error_preserves_boot_and_partial_clear_without_score_guard() {
    let mut spy = Spy::fresh();
    spy.fail = Some("pickupItem");
    assert_eq!(
        create(&mut spy),
        Err(Error::Clear(clear::Error::Capability(Fault("pickupItem"))))
    );
    assert_eq!(spy.scores(), [8, 9]);
    assert_eq!(spy.n("newTime"), 0.0);
    assert_eq!(spy.n("checkWater"), 0.0);
    assert!(!spy.has("pickupItem"));
    assert!(!spy.has("crapsPoint"));
    assert!(!spy.has("plife"));
    assert!(!spy.calls.contains(&Call::Get("isTunnelMan")));
}

#[test]
fn false_first_threshold_still_queries_every_original_score_in_order() {
    let mut spy = Spy::fresh();
    spy.joysticks = [false, true];
    spy.table[0] = 9_000_001.0;
    create(&mut spy).unwrap();
    assert_eq!(spy.scores(), [8, 9, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert!(!spy.calls.contains(&Call::ScoreClear));
    assert_eq!(spy.n("joyid"), 2.0);
    assert_eq!(spy.n("firstTime"), 0.0);
    assert_eq!(spy.n("marketChance"), 160.0);
    assert_eq!(
        [spy.n("plife"), spy.n("bombs"), spy.n("rope")],
        [4.0, 4.0, 4.0]
    );
    let i = spy
        .calls
        .iter()
        .position(|c| *c == Call::Get("isTunnelMan"))
        .unwrap();
    assert!(spy.calls[..i].contains(&Call::Number("isTunnelMan", 0.0)));
    assert_eq!(spy.calls.last(), Some(&Call::Score(9)));
}

#[test]
fn successful_reset_callback_updates_same_namespace_before_first_time_write() {
    let mut spy = Spy::fresh();
    create(&mut spy).unwrap();
    assert_eq!(spy.scores(), [8, 9, 1, 2, 3, 4, 5, 6, 7, 8, 9, 8, 9]);
    assert_eq!(spy.n("firstTime"), 1.0);
    assert_eq!(spy.n("tunnel1"), 100_001.0);
    assert_eq!(spy.n("tunnel2"), 200_001.0);
    assert_eq!(spy.calls.last(), Some(&Call::Number("firstTime", 1.0)));
    let reset = spy
        .calls
        .iter()
        .position(|c| *c == Call::ScoreClear)
        .unwrap();
    assert_eq!(spy.calls[reset + 1], Call::ScoreAdd("MONEY", 9_000_000.0));
    assert_eq!(
        spy.calls
            .iter()
            .filter(|c| matches!(c, Call::ScoreAdd(..)))
            .count(),
        10
    );
}

#[test]
fn failed_reset_keeps_callback_clear_and_completed_initializer_prefix() {
    let mut spy = Spy::fresh();
    spy.fail = Some("reset-after-clear");
    assert_eq!(
        create(&mut spy),
        Err(Error::Reset(reset::Error::Capability(Fault(
            "reset-after-clear"
        ))))
    );
    assert_eq!(
        spy.table,
        vec![0.0; 10],
        "primitive clear's effect retained"
    );
    assert_eq!(spy.n("firstTime"), 0.0);
    assert_eq!(spy.n("tunnel1"), -2_000_000.0);
    assert_eq!(spy.n("marketChance"), 160.0);
    assert_eq!(spy.n("currLevel"), 1.0);
    assert_eq!(spy.calls.last(), Some(&Call::ScoreClear));
    assert!(!spy.calls.contains(&Call::Number("firstTime", 1.0)));
}

#[test]
fn reached_threshold_query_error_keeps_clear_and_skips_later_queries_and_reset() {
    let mut spy = Spy::fresh();
    spy.fail_score_call = Some(7); // two initial reads, then threshold1..5
    assert_eq!(
        create(&mut spy),
        Err(Error::Score(reset::Error::Capability(Fault("score"))))
    );
    assert_eq!(spy.scores(), [8, 9, 1, 2, 3, 4, 5]);
    assert_eq!(spy.n("currLevel"), 1.0);
    assert_eq!(spy.n("firstTime"), 0.0);
    assert!(!spy.calls.contains(&Call::ScoreClear));
}

#[test]
fn unsupported_nonfinite_score_has_typed_reached_domain_error_without_default() {
    let mut spy = Spy::fresh();
    spy.table[7] = f64::INFINITY;
    assert!(
        matches!(create(&mut spy), Err(Error::Score(reset::Error::UnsupportedScore { index: 8, value })) if value == f64::INFINITY)
    );
    assert_eq!(spy.scores(), [8]);
    assert!(!spy.has("tunnel1"));
    assert_eq!(spy.n("tunnel3Max"), 300_000.0);
    assert!(!spy.has("mini1"));
}

#[test]
fn fractional_and_out_of_signed32_scores_are_rejected_at_the_reached_query() {
    for value in [
        2_012_345.5,
        -0.0,
        f64::from(i32::MAX) + 1.0,
        f64::from(i32::MIN) - 1.0,
    ] {
        let mut spy = Spy::fresh();
        spy.table[7] = value;
        assert_eq!(
            create(&mut spy),
            Err(Error::Score(reset::Error::UnsupportedScore {
                index: 8,
                value
            }))
        );
        assert_eq!(spy.scores(), [8]);
        assert!(!spy.has("tunnel1"));
        assert_eq!(spy.n("tunnel3Max"), 300_000.0);
    }
}

fn reset_input() -> Spy {
    let mut spy = Spy::fresh();
    spy.raw = Some(BTreeMap::from([
        ("tunnel1Max", Value::Number(100_000.0)),
        ("tunnel2Max", Value::Number(200_000.0)),
        ("tunnel1", Value::Number(12.0)),
        ("tunnel2", Value::Number(34.0)),
    ]));
    spy.table = vec![5.0; 10];
    spy
}

#[test]
fn reset_is_literal_primitives_and_fresh_score_set_order_with_strict_zero_insertion() {
    let mut spy = reset_input();
    reset::run(&mut spy).unwrap();
    assert_eq!(
        spy.calls,
        [
            Call::ScoreClear,
            Call::ScoreAdd("MONEY", 9_000_000.0),
            Call::ScoreAdd("TIME", 8_000_000.0),
            Call::ScoreAdd("KILLS", 7_000_000.0),
            Call::ScoreAdd("SAVES", 6_000_000.0),
            Call::ScoreAdd("PLAYS", 5_000_000.0),
            Call::ScoreAdd("WINS", 4_000_000.0),
            Call::ScoreAdd("DEATHS", 3_000_000.0),
            Call::Get("tunnel1Max"),
            Call::ScoreAdd("TUNNEL1", 2_100_001.0),
            Call::Get("tunnel2Max"),
            Call::ScoreAdd("TUNNEL2", 1_200_001.0),
            Call::ScoreAdd("MINIGAMES", 0.0),
            Call::Score(8),
            Call::Number("tunnel1", 100_001.0),
            Call::Score(9),
            Call::Number("tunnel2", 200_001.0),
        ]
    );
    assert_eq!(
        spy.table,
        [
            9_000_000.0,
            8_000_000.0,
            7_000_000.0,
            6_000_000.0,
            5_000_000.0,
            4_000_000.0,
            3_000_000.0,
            2_100_001.0,
            1_200_001.0,
            0.0
        ]
    );
    assert_eq!([spy.n("tunnel1"), spy.n("tunnel2")], [100_001.0, 200_001.0]);
}

#[test]
fn reset_missing_maximum_fails_after_seven_adds_without_inferred_defaults() {
    let mut spy = reset_input();
    spy.raw.as_mut().unwrap().remove("tunnel1Max");
    assert_eq!(
        reset::run(&mut spy),
        Err(reset::Error::Capability(Fault("missing-or-wrong-kind")))
    );
    assert_eq!(spy.calls.len(), 9);
    assert_eq!(spy.calls.last(), Some(&Call::Get("tunnel1Max")));
    assert_eq!(
        spy.table[..7],
        [
            9_000_000.0,
            8_000_000.0,
            7_000_000.0,
            6_000_000.0,
            5_000_000.0,
            4_000_000.0,
            3_000_000.0
        ]
    );
    assert_eq!([spy.n("tunnel1"), spy.n("tunnel2")], [12.0, 34.0]);
    assert!(spy.scores().is_empty());
}

#[test]
fn reset_unsupported_maximum_or_final_add_retains_exact_argument_prefix() {
    for (raw, maximum_error) in [(100_000.5, true), (f64::from(i32::MAX), false)] {
        let mut spy = reset_input();
        spy.raw
            .as_mut()
            .unwrap()
            .insert("tunnel1Max", Value::Number(raw));
        let error = reset::run(&mut spy).unwrap_err();
        if maximum_error {
            assert_eq!(
                error,
                reset::Error::UnsupportedMaximum {
                    name: "tunnel1Max",
                    value: raw
                }
            );
        } else {
            assert_eq!(
                error,
                reset::Error::UnsupportedAdd {
                    name: "TUNNEL1",
                    value: f64::from(i32::MAX) + 2_000_000.0 + 1.0
                }
            );
        }
        assert_eq!(spy.calls.last(), Some(&Call::Get("tunnel1Max")));
        assert_eq!(spy.n("tunnel1Max"), raw, "no rounding or clamping");
        assert_eq!(spy.n("tunnel1"), 12.0);
        assert!(spy.scores().is_empty());
    }
}

#[test]
fn reset_add_failure_preserves_completed_table_prefix_before_any_maximum_get() {
    let mut spy = reset_input();
    spy.fail = Some("SAVES");
    assert_eq!(
        reset::run(&mut spy),
        Err(reset::Error::Capability(Fault("SAVES")))
    );
    assert_eq!(spy.table[..4], [9_000_000.0, 8_000_000.0, 7_000_000.0, 0.0]);
    assert_eq!(
        spy.calls.last(),
        Some(&Call::ScoreAdd("SAVES", 6_000_000.0))
    );
    assert!(!spy.calls.iter().any(|call| matches!(call, Call::Get(_))));
    assert_eq!([spy.n("tunnel1"), spy.n("tunnel2")], [12.0, 34.0]);
}

#[test]
fn reset_score9_error_keeps_score8_tunnel_write_before_returning_to_globals_create() {
    let mut spy = Spy::fresh();
    spy.fail_score_call = Some(13); // boot8/9, threshold1..9, reset8/9
    assert_eq!(
        create(&mut spy),
        Err(Error::Reset(reset::Error::Capability(Fault("score"))))
    );
    assert_eq!(spy.n("tunnel1"), 100_001.0);
    assert_eq!(spy.n("tunnel2"), -1_000_000.0);
    assert_eq!(spy.n("firstTime"), 0.0);
    assert_eq!(
        &spy.calls[spy.calls.len() - 3..],
        [
            Call::Score(8),
            Call::Number("tunnel1", 100_001.0),
            Call::Score(9)
        ]
    );
    assert!(!spy.calls.contains(&Call::Number("firstTime", 1.0)));
}

#[test]
fn error_after_concrete_clear_retains_the_ten_zero_slots_instead_of_old_scores() {
    let mut spy = reset_input();
    spy.fail = Some("reset-after-clear");
    assert_eq!(
        reset::run(&mut spy),
        Err(reset::Error::Capability(Fault("reset-after-clear")))
    );
    assert_eq!(spy.calls, [Call::ScoreClear]);
    assert_eq!(spy.table, vec![0.0; 10]);
    assert_eq!([spy.n("tunnel1"), spy.n("tunnel2")], [12.0, 34.0]);
}
