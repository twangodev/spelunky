//! Conditional software-value controls, not an original-engine observation.
//! Native 403498 and 4E434C operation/order contracts apply to both explicit
//! admissions. No profile is inferred from host FPU state or a game seed.
use super::*;

#[derive(Debug, PartialEq, Eq)]
enum Failure {
    Read,
}
struct Rows {
    rows: Vec<(u64, u64, bool, i32)>,
    reads: Vec<&'static str>,
    count: Option<i32>,
    missing: bool,
    fail_x: bool,
}
impl Rows {
    fn at(points: &[(u64, u64)]) -> Self {
        Self {
            rows: points
                .iter()
                .enumerate()
                .map(|(i, &(x, y))| (x, y, false, 100001 + i as i32))
                .collect(),
            reads: vec![],
            count: None,
            missing: false,
            fail_x: false,
        }
    }
}
impl RawCandidates for Rows {
    type Key = usize;
    type Error = Failure;
    fn count(&mut self, _: Route) -> Result<i32, Failure> {
        self.reads.push("count");
        Ok(self.count.unwrap_or(self.rows.len() as i32))
    }
    fn current_slot(&mut self, _: Route, i: i32) -> Result<Option<usize>, Failure> {
        self.reads.push("slot");
        Ok((!self.missing && i >= 0 && (i as usize) < self.rows.len()).then_some(i as usize))
    }
    fn tombstone108(&mut self, k: usize) -> Result<bool, Failure> {
        self.reads.push("108");
        Ok(self.rows[k].2)
    }
    fn x_qword(&mut self, k: usize) -> Result<u64, Failure> {
        self.reads.push("x");
        if self.fail_x {
            Err(Failure::Read)
        } else {
            Ok(self.rows[k].0)
        }
    }
    fn y_qword(&mut self, k: usize) -> Result<u64, Failure> {
        self.reads.push("y");
        Ok(self.rows[k].1)
    }
    fn visible_id_i32(&mut self, k: usize) -> Result<i32, Failure> {
        self.reads.push("id");
        Ok(self.rows[k].3)
    }
}
fn cap() -> Admission {
    admit_pc53_control_word(0x1272).unwrap()
}
fn query(p: &mut Rows) -> Result<Outcome<usize>, Error<Failure>> {
    evaluate(&cap(), 0, 0, (-3f64).to_bits(), p)
}
fn wide(bits: u64) -> f::F128 {
    f::from_f64(f64::from_bits(bits)).unwrap()
}

#[test]
fn pc53_admission_is_explicit_and_old_pc64_constructor_stays_narrow() {
    for word in [0x1232, 0x1272] {
        assert_eq!(
            admit_pc53_control_word(word)
                .unwrap()
                .profile_control_word(),
            word
        );
        assert_eq!(admit_control_word(word), Err(UnsupportedControlWord(word)));
    }
    for word in [0x1332, 0x1372] {
        assert_eq!(
            admit_control_word(word).unwrap().profile_control_word(),
            word
        );
        assert_eq!(
            admit_pc53_control_word(word),
            Err(UnsupportedControlWord(word))
        );
    }
    for word in [
        0, 0x0032, 0x1132, 0x1632, 0x1a32, 0x1e32, 0x1233, 0x1230, 0x123a,
    ] {
        assert_eq!(
            admit_pc53_control_word(word),
            Err(UnsupportedControlWord(word))
        );
    }
}
#[test]
fn pc53_selector_uses_rc_not_arithmetic_precision_and_preserves_low_dword() {
    let pc64 = admit_control_word(0x1332).unwrap();
    for (value, expected) in [
        (0f64, 0i64),
        (-0., 0),
        (0.5, 0),
        (-0.5, 0),
        (1.5, 2),
        (2.5, 2),
        (-1.5, -2),
        (-2.5, -2),
        (4294967295., 4294967295),
        (4294967296., 4294967296),
        (-9223372036854775808., i64::MIN),
    ] {
        let a = selector_rte_i64_low_i32(&cap(), value.to_bits()).unwrap();
        let b = selector_rte_i64_low_i32(&pc64, value.to_bits()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.rounded_i64, expected);
        assert_eq!(a.low_i32, expected as i32);
    }
    for bits in [0x7ff0000000000000, 0xfff0000000000000, 0x7ff8000000000001] {
        assert_eq!(
            selector_rte_i64_low_i32(&cap(), bits),
            Err(SelectorError::NonFinite)
        );
    }
    for value in [9223372036854775808f64, -18446744073709551616.] {
        assert_eq!(
            selector_rte_i64_low_i32(&cap(), value.to_bits()),
            Err(SelectorError::OutsideSuccessfulI64Conversion)
        );
    }
}
#[test]
fn pc53_identical_point_tie_differs_from_pc64_wide_then_qword_store() {
    // Exact Fraction proof: native PC64 square of binary64 0.1 lies strictly
    // below its upward-rounded QWORD store. PC53 square already equals that
    // QWORD, so second identical candidate ties and leaves the first selected.
    let mut rows = Rows::at(&[(0x3fb999999999999a, 0), (0x3fb999999999999a, 0)]);
    let a = query(&mut rows).unwrap();
    assert_eq!(a.selected_native_id, Some(100001));
    assert_eq!(a.final_best_qword, 0x3f847ae147ae147c);
    let b = evaluate(
        &admit_control_word(0x1332).unwrap(),
        0,
        0,
        (-3f64).to_bits(),
        &mut rows,
    )
    .unwrap();
    assert_eq!(b.selected_native_id, Some(100002));
    assert_eq!(b.final_best_qword, a.final_best_qword);
}
#[test]
fn pc53_retains_extended_exponent_below_binary64_underflow() {
    let tiny = wide(1);
    let square = cap().mul(tiny, tiny).unwrap();
    assert!(!square.is_zero());
    assert_eq!(
        f::to_ext80_nearest(square).unwrap(),
        f::Ext80 {
            sign_exp: 0x379b,
            significand: 0x8000000000000000
        }
    );
    assert_eq!(f::to_f64_nearest(square).unwrap().to_bits(), 0);
    let out = query(&mut Rows::at(&[(1, 0), (0, 0)])).unwrap();
    assert_eq!(out.selected_native_id, Some(100001));
    assert_eq!(out.final_best_qword, 0);
}
#[test]
fn pc53_retains_extended_exponent_above_binary64_overflow() {
    let huge = wide(0x7fefffffffffffff);
    let difference = cap().sub(huge, huge.neg()).unwrap();
    let square = cap().mul(difference, difference).unwrap();
    assert!(!square.is_zero());
    assert_eq!(f::to_f64_nearest(square), Err(NumericError::Overflow));
    let out = evaluate(
        &cap(),
        0xffefffffffffffff,
        0,
        (-3f64).to_bits(),
        &mut Rows::at(&[(0x7fefffffffffffff, 0)]),
    )
    .unwrap();
    assert_eq!(out.selected_native_id, None);
    assert_eq!(out.final_best_qword, INITIAL_BEST_QWORD);
}
#[test]
fn pc53_native_reloads_and_recomputation_remain_literal() {
    let mut rows = Rows::at(&[(3f64.to_bits(), 4f64.to_bits())]);
    let out = query(&mut rows).unwrap();
    assert_eq!(out.final_best_qword, 25f64.to_bits());
    assert_eq!(
        rows.reads,
        vec![
            "count", "slot", "108", "slot", "x", "slot", "y", "slot", "x", "slot", "y", "slot",
            "id"
        ]
    );
}
#[test]
fn pc53_exact_id_and_marked_paths_do_not_inspect_nonfinite_coordinates() {
    let mut rows = Rows::at(&[(f64::NAN.to_bits(), f64::INFINITY.to_bits())]);
    let out = evaluate(
        &cap(),
        f64::NAN.to_bits(),
        0,
        100000f64.to_bits(),
        &mut rows,
    )
    .unwrap();
    assert_eq!(out.source_result_qword, NOONE_QWORD);
    assert!(rows.reads.is_empty());
    rows.rows[0].2 = true;
    let out = query(&mut rows).unwrap();
    assert_eq!(out.source_result_qword, NOONE_QWORD);
    assert_eq!(rows.reads, vec!["count", "slot", "108"]);
    rows.rows[0].2 = false;
    assert_eq!(
        query(&mut rows),
        Err(Error::Arithmetic(NumericError::NonFinite))
    );
}
#[test]
fn pc53_unsupported_backing_is_typed_without_empty_or_coordinate_fallback() {
    let mut rows = Rows::at(&[(0, 0)]);
    rows.count = Some(-1);
    assert_eq!(query(&mut rows), Err(Error::UnsupportedNegativeCount(-1)));
    rows.count = None;
    rows.missing = true;
    assert_eq!(
        query(&mut rows),
        Err(Error::UnsupportedMissingSlot {
            route: Route::All,
            index: 0
        })
    );
    rows.missing = false;
    rows.fail_x = true;
    assert_eq!(query(&mut rows), Err(Error::Provider(Failure::Read)));
}

#[path = "nearest_numeric_pc53_vectors.rs"]
mod independent_vectors;
