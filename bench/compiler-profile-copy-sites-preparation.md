# Copy-site measurement preparation and review

This records the initial preparation committed at 6fc9099c. Root reviewed the
complete launcher and accounting helper, including the helper's decimal-string
correction, before installing those bench files. The later first invocation
stopped in preflight before any compiler phase, as recorded in the
[cohort-1 refusal](compiler-profile-copy-sites-cohort-1-refusal.md). Its narrowly
corrected launcher still requires a distinct exact committed release. Neither
preparation nor that correction changes compiler or optimizer policy.

## Initial frozen preparation at 6fc9099c

| File | SHA-256 |
| --- | --- |
| compiler-profile-copy-sites-measurement-plan.md | 6bd73c58ce10a36cdc5fed7ba4fd5b1bf96f83e6550e7a572852b2884b955e37 |
| compiler-profile-copy-sites.sh | 47d085bc0cf1b0a48be56fcfda8efcec5cba85162dcdf9031f5da5ba7e6140e6 |
| compiler-profile-copy-account.pl | 0f938e92e961d94581f4424804f798c67cb168269af3de0734f4e7703d8fcb57 |
| compiler-profile-copy-sites-release.example.json | 1bca2f0720bda0b5bb9b8ebbe379e9fb23f9b8007a656595c1e5c002896c05a8 |

The launcher passes Bash syntax and all eight embedded Perl syntax checks.
It implements the declared seven foreground phases, fixed 4 GiB/900 s/10 s
bounds, fresh once-only directories, exact source and executable pins, raw
captures before validation, output-byte oracles and content-addressed retention.
It has no retry, resume or cleanup mode. The release example is intentionally
unfilled; it is not permission to substitute a current or ungated binary.
The [launcher review](compiler-profile-copy-sites-preparation-data/launcher-preparation.md)
states the remaining execution/ownership assumptions.

## Accounting fixture and correction

The separately authorized fixture reads the retained 28,522-byte role profile
`1f091222…b27a1` and 59,965-byte companion `b323c640…0154b`, using the gated
`6cd52308…e7bdacc` official reader. It executes no sampled program. The fixture
has 58 copy rows, 39 selected-function slots, 69 symbols and 1,092 kind intervals.
A derived counter-only state deliberately sets S=10, T=9, K=5, mapped=6,
unmapped=0 and D=6. Its one detail-leading in-flight sample is explicit and
unlocated; these synthetic counts are not execution observations.

The first validation captures one fixture-setup command, four official reads,
two helper acceptances and six expected helper refusals. Official reads take
0.00–0.01 s and helper cases 0.02–0.03 s, with load 2.58/1.69/2.01 unchanged.
The six refusals cover a reserved header, counter overflow, impossible copy
offset, wrong companion, false detail accounting and false summary marginal.

Offline inspection found that small Math::BigInt::bstr results sometimes
retained numeric flags, contradicting the helper's JSON string-only schema.
The original helper and validation remain intact. Final helper `0f938e92`
forces fresh decimal strings at those JSON boundaries. The second validation
runs only the helper against already captured official outputs: two acceptances
and the same six refusals. Recursive JSON string-type checks pass; all six TSVs
per positive state remain byte-identical. The accepted reads take 0.03 s each
at load 2.39/1.93/2.05 unchanged. Refusals take 0.02–0.03 s, with individual
load pairs retained from 2.39/1.93/2.05 to 2.36/1.94/2.05. Every prior validation
file rehashes unchanged. Bounds for these fixture commands are 4 GiB/60 s/10 s.

The helper validates the raw binary framing/counters and correspondence with
recorded official output; it does not replace the official IR check or attest
that a command executed. The fixture does not exercise a nonterminal helper
page or positive unmapped count, and does not exhaust all malformed inputs.
These limitations remain explicit before launch. Full details are in the
[accounting validation](compiler-profile-copy-sites-preparation-data/accounting-validation.md).

## Retained bytes and independent review

Root verifies the helper's 321-record manifest
`4146ef453a3ed60e2b846827fb454ebeb7a29128357dce87bf785a5d1120a96d`.
The deterministic [proposal archive](compiler-profile-copy-sites-preparation-data/accounting-proposal.tar.gz)
contains all 322 regular files, including that manifest, and 41 directories:
630,813 logical bytes, compressed to 125,867 bytes, SHA-256
`ff8e03d6bdbc49d6c97f7fdb79f223e71e9bc619885df273d9e072db328e8e90`.
Root independently compares every archived file hash with the manifest and
live file bytes; all agree. The archive retains both earlier draft versions,
the first schema-defective results, the final correction, raw inputs, exact
commands, resource/load captures and all expected refusals.

[Raw phase extraction](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-profile-copy-sites-preparation-data/accounting-validation-phases.json)
contains all 21 captures, including fixture setup. The first root extraction
mistakenly expected 20 and stopped before emitting a report; its empty output
and error remain beside the corrected extraction. No input or command was
rerun to correct that offline count assumption.

There is no new compiler construction, attribution measurement, speedup,
optimization eligibility result or Rust-O comparison in this preparation.

## Subsequent launcher correction

The original launcher hash in the table identifies the initially reviewed
snapshot, retained verbatim as
[launcher.sh](compiler-profile-copy-sites-cohort-1-refusal-data/launcher.sh).
The first runtime preflight exposed a Perl list-context defect that syntax
checking did not detect. The current launcher is
`abf52e1d5f22370a999e6e7765c4b28fe34671ff16430aa93da8d7112fd71bd6`:
only its predicate helper gains a two-scalar prototype and explicit arity
validation. The refusal report retains the original preparation manifest,
all diagnostics, twelve pure controls and the single corrected read-only
preflight. No workload phase was rerun or repaired within cohort 1.
