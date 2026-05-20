//! Fault-localization integration tests for fast-float.
//!
//! One `#[test]` per property in src/bin/etna-faultloc.rs's dispatch.
//! Each test runs `crabcheck::quickcheck_with_locate!` on the property,
//! prints the report, and emits a single `@@LOCATE@@ {<json>}` line on
//! stdout. Tests never panic — the driver classifies success/failure
//! from the JSON.

use std::fmt;

use crabcheck::quickcheck::{Arbitrary, Mutate};
use fast_float::etna::{property_decimal_roundtrip_f64, PropertyResult};
use rand::Rng;

#[derive(Clone, Copy)]
struct DecimalShape {
    mantissa: u64,
    tz: u16,
}
impl fmt::Debug for DecimalShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(m={} tz={})", self.mantissa, self.tz)
    }
}

impl<R: Rng> Arbitrary<R> for DecimalShape {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let roll: u8 = rng.random_range(0u8..4u8);
        if roll != 0 {
            DecimalShape {
                mantissa: rng.random_range(9_007_199_254_740_992u64..=9_007_199_254_741_200u64),
                tz: rng.random_range(800u16..=1500u16),
            }
        } else {
            DecimalShape {
                mantissa: rng.random(),
                tz: rng.random(),
            }
        }
    }
}

impl<R: Rng> Mutate<R> for DecimalShape {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = *self;
        if rng.random_bool(0.5) {
            let bit = rng.random_range(0u32..64);
            out.mantissa ^= 1u64 << bit;
        } else {
            let bit = rng.random_range(0u32..16);
            out.tz ^= 1u16 << bit;
        }
        out
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn format_decimal(mantissa: u64, trailing_zeros: u16) -> String {
    let mut s = format!("{}.", mantissa);
    let n = trailing_zeros as usize;
    s.reserve(n);
    for _ in 0..n {
        s.push('0');
    }
    s
}

fn property_decimal_roundtrip_f64_test(d: DecimalShape) -> Option<bool> {
    let input = format_decimal(d.mantissa, d.tz);
    to_opt(property_decimal_roundtrip_f64(input))
}

fn emit_locate_json(r: &crabcheck::profiling::LocateResult) {
    use crabcheck::quickcheck::ResultStatus;
    let status = match &r.run.status {
        ResultStatus::Failed { .. } => "Failed",
        ResultStatus::Finished => "Finished",
        ResultStatus::GaveUp => "GaveUp",
        ResultStatus::TimedOut => "TimedOut",
        ResultStatus::Aborted { .. } => "Aborted",
    };
    let top = if let Some(s) = r.top() {
        serde_json::json!({
            "rank": s.rank,
            "file": s.region.file,
            "function": s.region.function,
            "start_line": s.region.start_line,
            "end_line": s.region.end_line,
            "ochiai": s.region.suspiciousness.ochiai,
            "delta": s.region.delta,
            "panic_overlap": s.panic_overlap,
            "confidence": format!("{}", s.confidence),
            "confidence_rule": s.confidence_rule,
        })
    } else {
        serde_json::Value::Null
    };
    let top_5: Vec<_> = r
        .suspects
        .iter()
        .take(5)
        .map(|s| {
            serde_json::json!({
                "rank": s.rank,
                "file": s.region.file,
                "function": s.region.function,
                "start_line": s.region.start_line,
                "end_line": s.region.end_line,
                "confidence": format!("{}", s.confidence),
                "confidence_rule": s.confidence_rule,
                "panic_overlap": s.panic_overlap,
            })
        })
        .collect();
    let diags: Vec<_> = r.diagnostics.iter().map(|d| d.tag()).collect();
    let out = serde_json::json!({
        "status": status,
        "passed": r.run.passed,
        "discarded": r.run.discarded,
        "n_panics": r.n_panics,
        "n_suspects": r.suspects.len(),
        "top": top,
        "top_5": top_5,
        "diagnostics": diags,
    });
    println!("@@LOCATE@@ {}", out);
}

#[test]
fn locate_decimal_roundtrip_f64() {
    let report =
        crabcheck::quickcheck_with_locate!(property_decimal_roundtrip_f64_test, "fast_float");
    eprintln!("{report}");
    emit_locate_json(&report);
}
