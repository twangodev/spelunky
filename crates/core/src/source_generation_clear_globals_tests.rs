use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Number(f64),
    Text(String),
}
#[derive(Debug, Clone, PartialEq)]
enum Call {
    Get(String),
    Number(String, f64),
    Text(String, String),
}
#[derive(Debug, Clone, PartialEq)]
enum Fault {
    Missing(&'static str),
    WrongKind(&'static str),
    Reached(&'static str),
}
struct Raw {
    values: BTreeMap<String, Value>,
    calls: Vec<Call>,
    fail_set: Option<&'static str>,
    fail_after_set: Option<&'static str>,
}
impl Raw {
    fn prior() -> Self {
        Self {
            values: BTreeMap::from([
                ("pickupItem".into(), Value::Text("Rock".into())),
                ("plife".into(), Value::Number(7.0)),
                ("bombs".into(), Value::Number(9.0)),
                ("rope".into(), Value::Number(8.0)),
                ("money".into(), Value::Number(321.0)),
                ("time".into(), Value::Number(777.0)),
                ("currLevel".into(), Value::Number(12.0)),
                ("cleanSolids".into(), Value::Number(37.0)),
                ("isDamsel".into(), Value::Number(0.75)),
                ("untouchedText".into(), Value::Text("kept".into())),
            ]),
            calls: Vec::new(),
            fail_set: None,
            fail_after_set: None,
        }
    }
    fn tunnel(&mut self, raw: f64) {
        self.values.insert("isTunnelMan".into(), Value::Number(raw));
    }
    fn n(&self, name: &str) -> f64 {
        match self.values.get(name) {
            Some(Value::Number(value)) => *value,
            other => panic!("required numeric {name}: {other:?}"),
        }
    }
    fn t(&self, name: &str) -> &str {
        match self.values.get(name) {
            Some(Value::Text(value)) => value,
            other => panic!("required text {name}: {other:?}"),
        }
    }
}
impl Globals for Raw {
    type Error = Fault;
    fn number(&mut self, name: &'static str) -> Result<f64, Fault> {
        self.calls.push(Call::Get(name.into()));
        match self.values.get(name) {
            Some(Value::Number(value)) => Ok(*value),
            Some(Value::Text(_)) => Err(Fault::WrongKind(name)),
            None => Err(Fault::Missing(name)),
        }
    }
    fn set_number(&mut self, name: &'static str, value: f64) -> Result<(), Fault> {
        self.calls.push(Call::Number(name.into(), value));
        if self.fail_set == Some(name) {
            return Err(Fault::Reached(name));
        }
        self.values.insert(name.into(), Value::Number(value));
        if self.fail_after_set == Some(name) {
            return Err(Fault::Reached(name));
        }
        Ok(())
    }
    fn set_text(&mut self, name: &'static str, value: &'static str) -> Result<(), Fault> {
        self.calls.push(Call::Text(name.into(), value.into()));
        if self.fail_set == Some(name) {
            return Err(Fault::Reached(name));
        }
        self.values.insert(name.into(), Value::Text(value.into()));
        if self.fail_after_set == Some(name) {
            return Err(Fault::Reached(name));
        }
        Ok(())
    }
}

// Independent oracle: read the pinned ORIGINAL GML, not the Rust body/table.
// This tiny parser accepts only the script's actual assignment syntax and its
// sole isTunnelMan branch. It is not a generic GML interpreter or native claim.
const ORIGINAL: &str = include_str!("../tests/fixtures/source-clear-globals-original.gml");
fn assignments(source: &str) -> Vec<Call> {
    source
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("global."))
        .map(|line| {
            let (name, value) = line.split_once(" = ").expect("source assignment");
            let value = value.split_once(';').expect("source semicolon").0;
            match value {
                "\"\"" => Call::Text(name.into(), String::new()),
                "true" => Call::Number(name.into(), 1.0),
                "false" => Call::Number(name.into(), 0.0),
                _ => Call::Number(name.into(), value.parse().expect("literal numeric RHS")),
            }
        })
        .collect()
}
fn original_calls(tunnel: bool) -> Vec<Call> {
    let (prefix, branches) = ORIGINAL
        .split_once("if (global.isTunnelMan)")
        .expect("sole original condition");
    let (yes, rest) = branches.split_once("else").expect("source else");
    let (no, tail) = rest.split_once("global.flares").expect("post-branch tail");
    let mut calls = assignments(prefix);
    calls.push(Call::Get("isTunnelMan".into()));
    calls.extend(assignments(if tunnel { yes } else { no }));
    calls.extend(assignments(&format!("global.flares{tail}")));
    calls
}

#[test]
fn complete_original_row_order_and_both_raw_conditional_branches() {
    for (value, tunnel, inventory) in [
        (-4.0, false, [4.0, 4.0, 4.0]),
        (0.499, false, [4.0, 4.0, 4.0]),
        (0.5, true, [2.0, 0.0, 0.0]),
        (7.25, true, [2.0, 0.0, 0.0]),
    ] {
        let mut raw = Raw::prior();
        raw.tunnel(value);
        run(&mut raw).unwrap();
        let expected = original_calls(tunnel);
        assert_eq!(expected.len(), 115, "114 SETs and the reached raw GET");
        assert_eq!(raw.calls, expected, "every original assignment/order");
        assert_eq!([raw.n("plife"), raw.n("bombs"), raw.n("rope")], inventory);
        assert_eq!(raw.n("isTunnelMan"), value);
        assert_eq!(raw.n("cleanSolids"), 37.0, "not reset by this script");
        assert_eq!(raw.n("isDamsel"), 0.75, "selected character retained");
        assert_eq!(raw.t("untouchedText"), "kept");
        assert_eq!(raw.t("pickupItem"), "");
        assert_eq!(raw.n("checkWater"), 0.0);
        assert_eq!(raw.n("marketChance"), 160.0);
        assert_eq!(raw.n("goldChance"), 160.0);
        assert_eq!([raw.n("currLevel"), raw.n("levelType")], [1.0, 0.0]);
    }
}

#[test]
fn missing_actual_tunnel_read_retains_the_exact_completed_prefix() {
    let mut raw = Raw::prior();
    assert_eq!(
        run(&mut raw),
        Err(Error::Capability(Fault::Missing("isTunnelMan")))
    );
    let expected = original_calls(false);
    let get = expected
        .iter()
        .position(|c| matches!(c, Call::Get(_)))
        .unwrap();
    assert_eq!(get, 62);
    assert_eq!(raw.calls, expected[..=get]);
    assert_eq!(raw.n("marketChance"), 160.0);
    assert_eq!(raw.n("hasSpectacles"), 0.0);
    assert_eq!(raw.t("pickupItem"), "");
    assert_eq!(
        [raw.n("plife"), raw.n("bombs"), raw.n("rope")],
        [7.0, 9.0, 8.0]
    );
    assert_eq!(raw.n("money"), 321.0);
    assert_eq!(raw.n("time"), 777.0);
    assert_eq!(raw.n("currLevel"), 12.0);
    assert!(!raw.values.contains_key("flares"));
}

#[test]
fn text_error_stops_after_camera_water_prefix_before_probability_assignments() {
    let mut raw = Raw::prior();
    raw.tunnel(1.0);
    raw.fail_set = Some("pickupItem");
    assert_eq!(
        run(&mut raw),
        Err(Error::Capability(Fault::Reached("pickupItem")))
    );
    let expected = original_calls(true);
    assert_eq!(raw.calls, expected[..=14]);
    assert_eq!(raw.t("pickupItem"), "Rock");
    assert_eq!(raw.n("checkWater"), 0.0);
    assert_eq!(raw.n("waterCounter"), 0.0);
    assert!(!raw.values.contains_key("crapsPoint"));
    assert!(!raw.values.contains_key("probDarkLevel"));
    assert!(!raw.calls.iter().any(|c| matches!(c, Call::Get(_))));
    assert_eq!(raw.n("money"), 321.0);
}

#[test]
fn branch_numeric_error_keeps_earlier_inventory_set_but_not_later_tail() {
    let mut raw = Raw::prior();
    raw.tunnel(0.5);
    raw.fail_set = Some("rope");
    assert_eq!(
        run(&mut raw),
        Err(Error::Capability(Fault::Reached("rope")))
    );
    let expected = original_calls(true);
    assert_eq!(raw.calls, expected[..=65]);
    assert_eq!(
        [raw.n("plife"), raw.n("bombs"), raw.n("rope")],
        [2.0, 0.0, 8.0]
    );
    assert!(!raw.values.contains_key("flares"));
    assert_eq!(raw.n("currLevel"), 12.0);
}

#[test]
fn wrong_kind_tunnel_value_is_not_boolean_false_or_character_default() {
    let mut raw = Raw::prior();
    raw.values
        .insert("isTunnelMan".into(), Value::Text("1".into()));
    assert_eq!(
        run(&mut raw),
        Err(Error::Capability(Fault::WrongKind("isTunnelMan")))
    );
    let expected = original_calls(false);
    assert_eq!(raw.calls, expected[..=62]);
    assert_eq!(raw.t("isTunnelMan"), "1");
    assert_eq!(raw.n("plife"), 7.0);
    assert_eq!(raw.n("currLevel"), 12.0);
}

#[test]
fn error_after_a_raw_set_keeps_that_store_without_rollback_or_tail() {
    let mut raw = Raw::prior();
    raw.tunnel(0.0);
    raw.fail_after_set = Some("marketChance");
    assert_eq!(
        run(&mut raw),
        Err(Error::Capability(Fault::Reached("marketChance")))
    );
    let expected = original_calls(false);
    let at = expected
        .iter()
        .position(|call| matches!(call, Call::Number(name, _) if name == "marketChance"))
        .unwrap();
    assert_eq!(raw.calls, expected[..=at]);
    assert_eq!(raw.n("marketChance"), 160.0);
    assert_eq!(raw.n("madeMarketEntrance"), 0.0);
    assert!(!raw.values.contains_key("marketEntrance"));
    assert!(!raw.values.contains_key("goldChance"));
    assert!(!raw.calls.iter().any(|call| matches!(call, Call::Get(_))));
    assert_eq!(raw.n("plife"), 7.0);
    assert_eq!(raw.n("money"), 321.0);
}
