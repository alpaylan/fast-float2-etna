# fast-float — Injected Bugs

Total mutations: 1

Each `etna/<variant>` branch is a pre-applied snapshot containing exactly one buggy commit on top of `base_commit`. The variant is also available as a marauders block (comment-toggled injection) in the base source at `src/decimal.rs`.

## Bug Index

| # | Name | Variant | File(s) | Injection | Fix Commit |
|---|------|---------|---------|-----------|------------|
| 1 | `parse_decimal` fails to trim trailing zeros before MAX_DIGITS truncation | `decimal_trailing_zeros_56ac048_1` | `src/decimal.rs` | marauders | `56ac048a96c10c014d5398bae9e548b929616228` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `decimal_trailing_zeros_56ac048_1` | `property_decimal_roundtrip_f64` | `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros` |

## Framework Coverage

| Property | etna | proptest | quickcheck | crabcheck | hegel |
|----------|:----:|:--------:|:----------:|:---------:|:-----:|
| `property_decimal_roundtrip_f64` | ✓ | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. `parse_decimal` fails to trim trailing zeros before MAX_DIGITS truncation

- **Variant**: `decimal_trailing_zeros_56ac048_1`
- **Location**: `src/decimal.rs` — body of `parse_decimal`
- **Property**: `property_decimal_roundtrip_f64`
- **Witnesses**: `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros`
- **Fix commit**: `56ac048a96c10c014d5398bae9e548b929616228` — *"Fix parse_decimal: trim trailing zeros before MAX_DIGITS clamp"*
- **Invariant violated**: `fast_float::parse::<f64>(s).unwrap()` must be bit-equal to `s.parse::<f64>().unwrap()` for every ASCII decimal string both parsers accept.
- **How the mutation triggers**: `parse_decimal` fills `Decimal::digits` (a fixed `MAX_DIGITS = 768`-byte buffer) with the raw digit characters it scans. After the loop that reads the fractional part, the fixed version records the pre-fractional start pointer and, once `num_digits` is known to be non-zero, walks the last `num_digits` slice backwards and decrements `num_digits` for every trailing `'0'` byte. The mutation omits that trim. For inputs like `"9007199254740993." + '0' * 1000`, `num_digits` sits at exactly 1000 + 16 = 1016 when the MAX_DIGITS clamp fires; the clamp discards the *last* 248 digit slots — but since all of them are trailing zeros in the fixed version, the clamp would be a no-op, whereas in the buggy version `num_digits` is still 1016 and `d.decimal_point += d.num_digits as i32` inflates the decimal exponent by the wrong offset. The resulting `Decimal` rounds off by 1 ulp from the stdlib reference (`fast_float` yields `0x4340000000000001`, stdlib yields `0x4340000000000000`). The witnesses pick mantissas at the `2^53` boundary where a 1-ulp swing flips the LSB visibly.

## Notes

- `marauders` injection: on the `master` branch, the base source has a comment-toggled marauders block (marker `decimal_trailing_zeros`, tags `etna`) wrapping the body of `parse_decimal`. `marauders list` prints the variant; `marauders convert --path src/decimal.rs --to functional` rewrites it into a runtime `match` on `std::env::var("M_<variant>")`, so `M_decimal_trailing_zeros_56ac048_1=active` flips the injected code at run time.
- The `etna/decimal_trailing_zeros_56ac048_1` branch is a pre-materialized version of the mutation: identical to base except the `parse_decimal` body reverts the trim-trailing-zeros fix and reorders the `decimal_point` adjustment, matching the pre-56ac048 code path.
- One candidate commit was dropped with a terminal reason: `check_len_ptr_overflow_5d086d1_1` (fix for issue #28 in commit `5d086d15fce43e4ecc2b5e8f7d61147580630c25`) swaps `unsafe { self.ptr.add(n) <= self.end }` for a safe `usize` subtraction in `AsciiStr::check_len`. The pre-fix code is UB if pointer addition overflows the allocated object, but `check_len` is only called with small constant `n = 8`, so no public API input can observably trigger the difference. Skipped with reason "no observable public invariant".
