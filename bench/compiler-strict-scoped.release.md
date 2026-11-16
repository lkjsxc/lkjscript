# Scoped replay release and CI evidence

The root session explicitly released one compute worker on 2026-11-16 after
independently reviewing the frozen preparation and inspecting both successful
jobs of CI run 43831972036 at revision e7afd108.

The authorized experiment is exactly the bound invocation in
compiler-strict-scoped.producer.md, with producer SHA-256
ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504,
31,724,996 bytes, source/gate
830896f51b449532e99c203a6531844fa399fb50, and launch manifest
0a7191669e924d862b031e6f3f389372c3fe9562f0c0df557d2a60f2355f5fe6.

One worker, 4 GiB/900 s per command, one initially cold shared 2 GiB store,
the same fixed six cases and no retries/replacements remain mandatory.
The original 0/6 experiment stays unchanged. All new source archives,
copied producer, key and store must remain until the root accepts the
results and independent archive/proof audits.

Root-reported, independently inspected CI evidence:

- Native gate: 1,156.617 s, load 0.53 to 2.42, 5,448 fresh requests,
  zero reused; bootstrap 59.778 s, load 0.53 to 0.83.
- Optional witness: 276 value/output cases and 344 verdicts.
- Native log SHA-256:
  418f72555ed64d50e1a28cc31425de10255c18cc2853c0f8f70ebbb2d8e1847e.
- Witness log SHA-256:
  ef7c94e56957afa101b02b242f8b1f106221f9406d19aa70579c6b835a404088.
- Native completed 08:51:29 UTC; witness completed 08:34:42 UTC.

This record follows the immutable prelaunch addendum, whose historical
"CI/release pending" statement is preserved. It changes no predeclared
parameter, case, command, acceptance rule or producer identity.
