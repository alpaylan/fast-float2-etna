//! ETNA benchmark harness.
//!
//! This module defines the framework-neutral `PropertyResult` enum plus one
//! `property_*` function per mined bug. Every framework adapter in
//! `src/bin/etna.rs` and every witness test calls into these functions.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

/// Fast-float's `parse::<f64>` must agree with the Rust standard library's
/// `str::parse::<f64>` for every ASCII decimal string that both accept.
/// Bug `decimal_trailing_zeros_56ac048_1` (fix #4) violated this for decimal
/// strings that contain many trailing zeros after the decimal point or
/// after the significant digits, because the decimal parser updated
/// `num_digits`/`decimal_point` without trimming trailing zeros before
/// applying the `MAX_DIGITS` truncation. The result was off-by-many-ulps
/// errors for inputs like `"9007199254740993.0"` plus trailing zeros.
pub fn property_decimal_roundtrip_f64(input: String) -> PropertyResult {
    // Bound input — we only care about short ASCII decimals with trailing
    // zeros. Outright reject anything the fast-float parser or std parser
    // can't both evaluate.
    if input.is_empty() || input.len() > 4096 {
        return PropertyResult::Discard;
    }
    if !input.is_ascii() {
        return PropertyResult::Discard;
    }
    let ff: Result<f64, _> = crate::parse(&input);
    let std: Result<f64, _> = input.parse::<f64>();
    match (ff, std) {
        (Ok(a), Ok(b)) => {
            if a.is_finite() && b.is_finite() {
                if a.to_bits() == b.to_bits() {
                    PropertyResult::Pass
                } else {
                    PropertyResult::Fail(format!(
                        "ff=0x{:016x} std=0x{:016x} input={:?}",
                        a.to_bits(),
                        b.to_bits(),
                        input
                    ))
                }
            } else if a.is_nan() && b.is_nan() {
                PropertyResult::Pass
            } else if a == b {
                PropertyResult::Pass
            } else {
                PropertyResult::Fail(format!(
                    "non-finite mismatch ff={} std={} input={:?}",
                    a, b, input
                ))
            }
        }
        // Both parsers reject: fine.
        (Err(_), Err(_)) => PropertyResult::Discard,
        // Mismatched accept/reject: outside the scope of this property.
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => PropertyResult::Discard,
    }
}
