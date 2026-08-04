# Canonical Numeric Conversions

[Authority](semantic-core.md)

## Status
<!-- LKJ-F numeric-conversions current 13DpH4F5yXKjmImAvd9g9H5y1B-eLK9c9AWplj7LTMo -->


**Current.** Four explicit conversion operations and rejection of implicit
mixed `i64`/`f64` arithmetic and ordering are implemented through Semantic
Source, HIR, verified SSA, evaluator, validated bytecode/VM, forced baseline
JIT, and forced proof JIT.

## Exact Operations

- `convert-i64-to-f64-rounded` has signature `i64 -> f64` in mathematical
  notation. It accepts every `i64` and uses binary64 round-to-nearest,
  ties-to-even.
- `convert-i64-to-f64-exact` returns `result f64 numeric-error` and reports
  `inexact` exactly when round-trip integer interpretation differs.
- `convert-f64-to-i64-exact` returns `result i64 numeric-error`; it reports
  `non-finite`, `out-of-range`, or `fractional` as appropriate.
- `convert-f64-to-i64-truncating` rejects non-finite values, truncates toward
  zero, and then rejects an out-of-range result.

Source signatures use structured `inputs/` and `output/` children; the arrow
above is explanatory notation, not accepted source syntax.

`numeric-error` is the compiler-recognized prelude enum with `non-finite`,
`out-of-range`, `fractional`, and `inexact` variants.

## Boundaries

Binary64 represents every integer from `-2^53` through `2^53` exactly; outside
that interval representability depends on spacing. `i64` is exactly
`[-2^63, 2^63 - 1]`. Since `2^63 - 1` rounds to binary64 `2^63`, range checks
use mathematical bounds rather than a host-cast maximum. Signed zero,
subnormals, NaNs, and infinities have explicit tested behavior. Undefined host
casts are not used.

## Evidence Boundary

Focused tables cover signed zero, subnormals, both fractional signs,
infinities, multiple NaN payloads, values around `2^53`, exact `-2^63`, and the
positive boundary. Four-engine tests require native baseline/proof entries and
zero VM fallback. Rounded conversion uses the native scalar conversion plan;
fallible conversions construct deterministic structural `result` and
`numeric-error` owners through generated structural calls with zero tracing-heap
dispatch.
