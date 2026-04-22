# fast-float — Injected Bugs

Fast floating-point number parser — ETNA workload mining fast-float-rust's git history for bug fixes.

Total mutations: 1

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `decimal_trailing_zeros_56ac048_1` | `decimal_trailing_zeros` | `src/decimal.rs` | `marauders` | `56ac048a96c10c014d5398bae9e548b929616228` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `decimal_trailing_zeros_56ac048_1` | `DecimalRoundtripF64` | `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `DecimalRoundtripF64` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. decimal_trailing_zeros

- **Variant**: `decimal_trailing_zeros_56ac048_1`
- **Location**: `src/decimal.rs`
- **Property**: `DecimalRoundtripF64`
- **Witness(es)**:
  - `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros`
  - `witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros`
  - `witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros`
- **Source**: Ignore trailing 0s when parsing decimals (fix #4)
  > `parse_decimal` filled a fixed 768-byte digit buffer but never trimmed trailing `'0'` bytes before the MAX_DIGITS clamp fired. For inputs like `9007199254740993.` followed by >752 zeros, the clamp truncated genuine digits instead of zero padding, inflating `decimal_point` and rounding the result off by 1 ulp at the `2^53` boundary.
- **Fix commit**: `56ac048a96c10c014d5398bae9e548b929616228` — Ignore trailing 0s when parsing decimals (fix #4)
- **Invariant violated**: `fast_float::parse::<f64>(s).unwrap()` must be bit-equal to `s.parse::<f64>().unwrap()` for every ASCII decimal string both parsers accept.
- **How the mutation triggers**: `parse_decimal` fills `Decimal::digits` (a fixed `MAX_DIGITS = 768`-byte buffer) with the raw digit characters it scans. After the loop that reads the fractional part, the fixed version records the pre-fractional start pointer and, once `num_digits` is known to be non-zero, walks the last `num_digits` slice backwards and decrements `num_digits` for every trailing `'0'` byte. The mutation omits that trim. For inputs like `"9007199254740993." + '0' * 1000`, `num_digits` sits at exactly 1000 + 16 = 1016 when the MAX_DIGITS clamp fires; the clamp discards the *last* 248 digit slots — but since all of them are trailing zeros in the fixed version, the clamp would be a no-op, whereas in the buggy version `num_digits` is still 1016 and `d.decimal_point += d.num_digits as i32` inflates the decimal exponent by the wrong offset. The resulting `Decimal` rounds off by 1 ulp from the stdlib reference (`fast_float` yields `0x4340000000000001`, stdlib yields `0x4340000000000000`). The witnesses pick mantissas at the `2^53` boundary where a 1-ulp swing flips the LSB visibly.
