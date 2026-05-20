use std::fmt;

use crabcheck::profiling::quickcheck;
use crabcheck::quickcheck::{Arbitrary, Mutate};
use fast_float::etna::{property_decimal_roundtrip_f64, PropertyResult};
use rand::Rng;

// Mirror src/bin/etna.rs DecimalShape: 3/4 chance of mantissa near
// 2^53 and tz in 800..=1500 — the exact region where the trailing-zeros
// bug fires. 1/4 full-random u64/u16.
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

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        return;
    }
    let result = match (args[1].as_str(), args[2].as_str()) {
        ("crabcheck", "DecimalRoundtripF64") => quickcheck(|d: DecimalShape| {
            let input = format_decimal(d.mantissa, d.tz);
            to_opt(property_decimal_roundtrip_f64(input))
        }),
        (a, b) => panic!("Unknown: {a} {b}"),
    };
    println!("Result: {:?}", result);
}
