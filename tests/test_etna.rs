//! Witness tests for ETNA mutations in this workload.
//!
//! Each `witness_*_case_*` test passes on base HEAD and fails when the
//! corresponding variant is active (via `M_<variant>=active` for marauders
//! mutations, or on the `etna/<variant>` branch for patch mutations).

use fast_float::etna::{property_decimal_roundtrip_f64, PropertyResult};

fn assert_pass(r: PropertyResult) {
    match r {
        PropertyResult::Pass => {}
        PropertyResult::Discard => panic!("witness discarded — inputs outside domain"),
        PropertyResult::Fail(msg) => panic!("property failed: {}", msg),
    }
}

fn trailing_dot_zeros(base: &str, n_zeros: usize) -> String {
    let mut s = String::from(base);
    s.push('.');
    for _ in 0..n_zeros {
        s.push('0');
    }
    s
}

// The fix (`decimal_trailing_zeros_56ac048_1`) trims trailing zeros from
// `num_digits` before the `MAX_DIGITS = 768` truncation. The buggy code
// leaves `num_digits` inflated, so the truncation chops off the
// *significant* leading digits and the resulting float is off by 1 ulp
// from the stdlib value. Each witness uses enough trailing zeros that the
// total digit count exceeds 768 and then tickles the rounding decision
// made at the boundary.

#[test]
fn witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros() {
    assert_pass(property_decimal_roundtrip_f64(trailing_dot_zeros(
        "9007199254740993",
        800,
    )));
}

#[test]
fn witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros() {
    assert_pass(property_decimal_roundtrip_f64(trailing_dot_zeros(
        "9007199254740993",
        1000,
    )));
}

#[test]
fn witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros() {
    assert_pass(property_decimal_roundtrip_f64(trailing_dot_zeros(
        "9007199254740997",
        1000,
    )));
}
