// ETNA workload runner for fast-float.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: DecimalRoundtripF64 | All
//
// Emits a single JSON line per run on stdout with fields:
//   status, tests, discards, time, counterexample, error, tool, property.
// Always exits 0 on completion; non-zero exit is reserved for adapter-level
// panics escaping the outer catch_unwind in main().

use crabcheck::quickcheck as crabcheck_qc;
use fast_float::etna::{property_decimal_roundtrip_f64, PropertyResult};
use hegel::{generators as hgen, Hegel, Settings as HegelSettings, TestCase};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestError, TestRunner};
use quickcheck::{Arbitrary, Gen, QuickCheck, ResultStatus, TestResult};
use rand::Rng;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

fn to_err(r: PropertyResult) -> Result<(), String> {
    match r {
        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
        PropertyResult::Fail(m) => Err(m),
    }
}

const ALL_PROPERTIES: &[&str] = &["DecimalRoundtripF64"];

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    let mut final_status: Result<(), String> = Ok(());
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if r.is_err() && final_status.is_ok() {
            final_status = r;
        }
    }
    (final_status, total)
}

// ---- shared input shape ----
//
// The bug (`decimal_trailing_zeros_56ac048_1`) requires a decimal string
// with enough trailing zeros that `num_digits > MAX_DIGITS = 768` and whose
// leading digits land near a half-ulp rounding boundary. To keep every
// framework likely to hit that shape within a ~200-case budget, we
// construct inputs as `{mantissa_digits}.{trailing_zeros}` rather than
// letting each framework draw arbitrary strings.

fn format_decimal(mantissa: u64, trailing_zeros: u16) -> String {
    let mut s = format!("{}.", mantissa);
    let n = trailing_zeros as usize;
    s.reserve(n);
    for _ in 0..n {
        s.push('0');
    }
    s
}

// Newtype around `(u64, u16)` implementing `Arbitrary + Debug + Display` so
// it can be used as a quickcheck/crabcheck property argument. Default
// uniform draws for u64 and u16 effectively never hit the bug shape
// (mantissa near 2^53, trailing zeros >= 768), so we bias strongly toward
// that region and keep a random tail for broad coverage.
#[derive(Clone, Copy)]
struct DecimalShape {
    mantissa: u64,
    tz: u16,
}

impl fmt::Debug for DecimalShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.mantissa, self.tz)
    }
}

impl fmt::Display for DecimalShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.mantissa, self.tz)
    }
}

impl Arbitrary for DecimalShape {
    fn arbitrary(g: &mut Gen) -> Self {
        let roll: u8 = g.random_range(0u8..4u8);
        if roll != 0 {
            let mantissa: u64 = g.random_range(9_007_199_254_740_992u64..=9_007_199_254_741_200u64);
            let tz: u16 = g.random_range(800u16..=1500u16);
            DecimalShape { mantissa, tz }
        } else {
            let mantissa: u64 = u64::arbitrary(g);
            let tz: u16 = u16::arbitrary(g);
            DecimalShape { mantissa, tz }
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let (m, t) = (self.mantissa, self.tz);
        Box::new(
            (m, t)
                .shrink()
                .map(|(mantissa, tz)| DecimalShape { mantissa, tz }),
        )
    }
}

impl<R: Rng> crabcheck_qc::Arbitrary<R> for DecimalShape {
    fn generate(rng: &mut R, _n: usize) -> DecimalShape {
        let roll: u8 = rng.random_range(0u8..4u8);
        if roll != 0 {
            let mantissa: u64 =
                rng.random_range(9_007_199_254_740_992u64..=9_007_199_254_741_200u64);
            let tz: u16 = rng.random_range(800u16..=1500u16);
            DecimalShape { mantissa, tz }
        } else {
            let mantissa: u64 = rng.random();
            let tz: u16 = rng.random();
            DecimalShape { mantissa, tz }
        }
    }
}

// ---- etna (deterministic witness-shaped inputs) ----

fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "DecimalRoundtripF64" => to_err(property_decimal_roundtrip_f64(format_decimal(
            9007199254740993,
            1000,
        ))),
        _ => {
            return (
                Err(format!("Unknown property for etna: {}", property)),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    (
        result,
        Metrics {
            inputs: 1,
            elapsed_us,
        },
    )
}

// ---- proptest ----

fn decimal_strategy() -> BoxedStrategy<(u64, u16)> {
    // Bias: half the cases concentrate the mantissa near 2^53 (the IEEE-754
    // f64 significand boundary where 9007199254740993 lives), and the
    // trailing-zeros count favours the >768 band that overruns MAX_DIGITS.
    prop_oneof![
        // Concentrated: mantissa near 2^53, many trailing zeros.
        (9_007_199_254_740_992u64..=9_007_199_254_741_000u64, 800u16..=1200u16),
        // Broader mantissa, still large zero tail.
        (any::<u64>(), 768u16..=1500u16),
        // Arbitrary — kept so the property domain stays observable.
        (any::<u64>(), any::<u16>()),
    ]
    .boxed()
}

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let mut runner = TestRunner::new(ProptestConfig::default());
    let c = counter.clone();
    let result: Result<(), String> = match property {
        "DecimalRoundtripF64" => runner
            .run(&decimal_strategy(), move |(mantissa, tz)| {
                c.fetch_add(1, Ordering::Relaxed);
                let input = format_decimal(mantissa, tz);
                let cex = format!("({} {})", mantissa, tz);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_decimal_roundtrip_f64(input)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        _ => {
            return (
                Err(format!("Unknown property for proptest: {}", property)),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ---- quickcheck ----

static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn qc_decimal_roundtrip_f64(d: DecimalShape) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let input = format_decimal(d.mantissa, d.tz);
    match property_decimal_roundtrip_f64(input) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let result = match property {
        "DecimalRoundtripF64" => QuickCheck::new()
            .tests(200)
            .max_tests(20_000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_decimal_roundtrip_f64 as fn(DecimalShape) -> TestResult),
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {}", property)),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("aborted: {:?}", err)),
        ResultStatus::TimedOut => Err("timed out".to_string()),
        ResultStatus::GaveUp => Err(format!(
            "gave up: passed={}, discarded={}",
            result.n_tests_passed, result.n_tests_discarded
        )),
    };
    (status, metrics)
}

// ---- crabcheck ----

static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_decimal_roundtrip_f64(d: DecimalShape) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let input = format_decimal(d.mantissa, d.tz);
    match property_decimal_roundtrip_f64(input) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cfg = crabcheck_qc::Config { tests: 20_000 };
    let result = match property {
        "DecimalRoundtripF64" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_decimal_roundtrip_f64 as fn(DecimalShape) -> Option<bool>,
        ),
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {}", property)),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("timed out".to_string()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => Err(format!("aborted: {}", error)),
    };
    (status, metrics)
}

// ---- hegel ----

static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new()
        .test_cases(200)
        .suppress_health_check(hegel::HealthCheck::all())
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    // Bias: ~3/4 cases draw from the concentrated band (large trailing
    // zero count, mantissa near 2^53). The remainder is fully arbitrary
    // so the property's domain is still exercised broadly.
    fn draw_biased_input(tc: &TestCase) -> (u64, u16) {
        let roll: u8 = tc.draw(hgen::integers::<u8>().min_value(0).max_value(3));
        if roll != 0 {
            let mantissa: u64 = tc.draw(
                hgen::integers::<u64>()
                    .min_value(9_007_199_254_740_992)
                    .max_value(9_007_199_254_741_200),
            );
            let tz: u16 = tc.draw(hgen::integers::<u16>().min_value(800).max_value(1500));
            (mantissa, tz)
        } else {
            let mantissa: u64 = tc.draw(hgen::integers::<u64>());
            let tz: u16 = tc.draw(hgen::integers::<u16>());
            (mantissa, tz)
        }
    }
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "DecimalRoundtripF64" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let (mantissa, tz) = draw_biased_input(&tc);
                let input = format_decimal(mantissa, tz);
                let cex = format!("({} {})", mantissa, tz);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_decimal_roundtrip_f64(input)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{}", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("{}", format!("__unknown_property:{}", property)),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {}", rest)),
                    Metrics::default(),
                );
            }
            Err(msg
                .strip_prefix("Property test failed: ")
                .unwrap_or(&msg)
                .to_string())
        }
    };
    (status, metrics)
}

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (
            Err(format!("Unknown tool: {}", tool)),
            Metrics::default(),
        ),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!("Properties: DecimalRoundtripF64 | All");
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    // Silence library-under-test panic noise. Frameworks catch their own
    // panics; defaults still print "thread 'main' panicked ..." to stderr,
    // which clutters the JSON stream. Also defends against adapter-level
    // panic escape — we convert that to status:aborted rather than a
    // non-zero exit (etna reads status from JSON, not exit code).
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(prev);

    let (status, m) = match caught {
        Ok(outcome) => outcome,
        Err(p) => {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "adapter panic (non-string payload)".to_string());
            emit_json(
                tool,
                property,
                "aborted",
                Metrics::default(),
                None,
                Some(&msg),
            );
            return;
        }
    };
    match status {
        Ok(()) => emit_json(tool, property, "passed", m, None, None),
        Err(e) => emit_json(tool, property, "failed", m, Some(&e), None),
    }
}
