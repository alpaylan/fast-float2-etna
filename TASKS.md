# fast-float — ETNA Tasks

Total tasks: 4

ETNA tasks are **mutation/property/witness triplets**. Each row below is one runnable task: the command executes the framework-specific adapter against the buggy variant branch and should report a counterexample (or time out).

Run against a variant by first checking out its branch (`git checkout etna/<variant>`) or by activating the marauders block on `master` (`marauders convert --path src/decimal.rs --to functional`, then `M_<variant>=active cargo run --release --bin etna -- <framework> <property>`).

## Task Index

| Task | Variant | Framework | Property | Witness(es) | Command |
|------|---------|-----------|----------|-------------|---------|
| 001 | `decimal_trailing_zeros_56ac048_1` | proptest | `property_decimal_roundtrip_f64` | `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros`, `witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros` | `cargo run --release --bin etna -- proptest DecimalRoundtripF64` |
| 002 | `decimal_trailing_zeros_56ac048_1` | quickcheck | `property_decimal_roundtrip_f64` | (same) | `cargo run --release --bin etna -- quickcheck DecimalRoundtripF64` |
| 003 | `decimal_trailing_zeros_56ac048_1` | crabcheck | `property_decimal_roundtrip_f64` | (same) | `cargo run --release --bin etna -- crabcheck DecimalRoundtripF64` |
| 004 | `decimal_trailing_zeros_56ac048_1` | hegel | `property_decimal_roundtrip_f64` | (same) | `cargo run --release --bin etna -- hegel DecimalRoundtripF64` |

## Witness catalog

Each witness is a deterministic concrete test in `tests/test_etna.rs`. On `base_commit` every witness passes. On the `etna/decimal_trailing_zeros_56ac048_1` branch (or with `M_decimal_trailing_zeros_56ac048_1=active`) every witness fails with a 1-ulp mismatch versus the stdlib `str::parse::<f64>` reference.

### `property_decimal_roundtrip_f64`

- `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros` — `"9007199254740993." + "0" * 800`. The 16-digit mantissa pushes `num_digits` to 816 before the clamp; the dropped trim causes `decimal_point` to be offset by the total instead of the significant count, and the rounded result differs from stdlib by 1 ulp (`0x4340000000000001` vs `0x4340000000000000`).
- `witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros` — `"9007199254740993." + "0" * 1000`. Same mechanism with a wider zero tail to exceed MAX_DIGITS more comfortably.
- `witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros` — `"9007199254740997." + "0" * 1000`. A neighbouring mantissa at the same 2^53 boundary; guards against the fix being tuned to a single magic number.

## Negative controls

There is only one injected variant in this workload, so there is no cross-variant negative control. The base-commit control is enforced by the CI-equivalent validate stage: every framework task reports `status:passed` on `master` (base, no mutation) and `status:failed` on `etna/decimal_trailing_zeros_56ac048_1`. Each witness test is also directly asserted to pass on base and fail under the active mutation.
