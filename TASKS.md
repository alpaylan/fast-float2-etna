# fast-float — ETNA Tasks

Total tasks: 4

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `decimal_trailing_zeros_56ac048_1` | proptest | `DecimalRoundtripF64` | `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros` |
| 002 | `decimal_trailing_zeros_56ac048_1` | quickcheck | `DecimalRoundtripF64` | `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros` |
| 003 | `decimal_trailing_zeros_56ac048_1` | crabcheck | `DecimalRoundtripF64` | `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros` |
| 004 | `decimal_trailing_zeros_56ac048_1` | hegel | `DecimalRoundtripF64` | `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros` |

## Witness Catalog

- `witness_decimal_roundtrip_f64_case_9007199254740993_dot_800_zeros` — base passes, variant fails
- `witness_decimal_roundtrip_f64_case_9007199254740993_dot_1000_zeros` — base passes, variant fails
- `witness_decimal_roundtrip_f64_case_9007199254740997_dot_1000_zeros` — base passes, variant fails
