# Source lookup windows and admitted namespace parents

Date: 2026-10-21. Scope: performance within the restricted E1 profile.
Candidate acceptance requires the finalized tree and fresh verification receipts.

## Evidence and decision

The clean starting checkout and remote main were
`d9166adba66192ae02b0fd7ec0d2a46e8597e1e0`.
Inspection confirmed temporary owner-key list/slice/concat construction in
`source-table::owner`, temporary digest slices in `source-type-table::lookup`,
and repeated namespace-parent lookup after containment admission.

Those operations are redundant, but the earlier whole-workload allocation
measurements do not attribute their total cost to these helpers. Measure the
complete `read-nominal` path before and after on the same source carrier.
Keep VM instructions, reference expressions, cumulative modeled allocation,
elapsed time, physical copying, live memory and RSS distinct.

The candidate removes the temporary keys without changing the carrier encoding,
runtime primitives, source eligibility, diagnostic order or proof-work accounting.
Owner lookup compares the explicit domain first, then UUID bytes in order.
Type lookup compares the 32 digest bytes at their original offset. The existing
generic lookup retains its interface and bytewise ordering; its private search
takes a key offset instead of its unused count argument. A separate owner search
keeps the domain projection out of ordinary digest lookup.

Both specialized entrypoints validate directory bounds and source-reference
ranges before offset arithmetic. Owner rows remain 50 bytes and type rows 32.
The serialized reference marker is excluded from the key, as before; preceding
source decoding owns marker validation. Invalid helper ranges return -2 and
missing keys -1. No little-endian numeric comparison replaces lexicographic order.

Namespace collection reads parent indexes from the same carrier's already
admitted containment rows. Module and Target retain the owner-count root sentinel.
The public wrappers still prepare and validate those rows, establish value scope,
then check namespaces before contextual types and nominal flow. Derived rows
remain local to the invocation and never become trusted external input.

## Reproducible measurement

The baseline carrier contains 26,704 owners, 26 types and 6,846 retirements:
5,879,667 bytes, SHA-256
`24d58b35f0dd218740f6bda77e4a275f7481d7dd2580bd2be7c4b4168148bd3b`.
Its semantic-state digest is
`semantic_state_5a833e3702e61b0311063519c35aecf411ceb53a2a5cdf07d4e040c1f1f10809`.
These are separate identities because semantic state does not bind unused types.

The opt-in `selfhost_source_measure_nominal_on_pinned_carrier` test captures a
carrier only when `LKJSCRIPT_SELFHOST_CAPTURE` names an absent file. Subsequent
runs use `LKJSCRIPT_SELFHOST_CARRIER` to read those exact bytes. Both executions
use the production VM and independent canonical reference interpreter and require
the same accepted semantic-state result. Run under the shared target lease:

```sh
LKJSCRIPT_SELFHOST_CAPTURE="$PWD/.artifacts/selfhost/accepted-e-source-lookup/baseline.lkjs" \
  cargo test --release --locked --lib selfhost_source_measure_nominal_on_pinned_carrier \
  -- --ignored --test-threads=1 --nocapture
LKJSCRIPT_SELFHOST_CARRIER="$PWD/.artifacts/selfhost/accepted-e-source-lookup/baseline.lkjs" \
  cargo test --release --locked --lib selfhost_source_measure_nominal_on_pinned_carrier \
  -- --ignored --test-threads=1 --nocapture
```

Both measurements used the same test executable (SHA-256
`77e504921026a2a2e57aebe6e9bfe12f973b3474f451a1396f1f13ec71283fb8`),
which reads the authoritative compiler graph at invocation. The baseline graph
was `rev_7058c5d00c8bfd480485c8b49ece4e06266a26f386ac5a521056f754f01e3fa0`;
the candidate was
`rev_eec02cc8d0a6137f8593d2b497a7e06f549fb00870873b06439ccb7682f9c82c`.
The executable hash remained unchanged through the candidate's last consumer.
The measured function is the composed `source-nominal-probe::read-nominal`.

| Quantity | Baseline | Candidate | Reduction |
| --- | ---: | ---: | ---: |
| VM instructions | 736,020,917 | 720,775,877 | 2.07% |
| VM cumulative modeled allocation bytes | 68,676,380,011 | 67,656,769,949 | 1.48% |
| VM elapsed milliseconds | 93,532 | 90,428 | 3.32% |
| Reference expressions | 388,764,234 | 381,570,481 | 1.85% |
| Reference cumulative modeled allocation bytes | 78,627,257,001 | 76,939,836,637 | 2.15% |
| Reference elapsed milliseconds | 464,860 | 442,776 | 4.75% |

Each run passed VM/reference equality and independent Rust-native execution on
the identical carrier. Timing is one sequential observation per implementation,
not a statistical performance guarantee. The deterministic operation counts and
modeled charges support retaining this bounded change; they do not measure live
heap, actual copied bytes, arena capacity or RSS. Native reclamation is unclaimed.
Current-carrier identities, representation headroom and all final acceptance
outcomes belong to the milestone receipt.

## Current capacity observation

The candidate's complete raw carrier contains 26,815 owners, 26 types and 7,393
retirements in 6,015,449 bytes. Its SHA-256 is
`bb1c35510f8dda9a4162537571b0510f4d3bd969fc705cff8eba0cd5c0405a42`;
its semantic state is
`semantic_state_2e0cb032c7842a40bc73e99e04342c19d0dde975f16e4a6e6609f707868bc8c1`.
The carrier leaves 2,373,159 bytes within the 8 MiB bound. Its 34,234 directory
rows require 805,037,184 source allowance units under the existing formula.
This allowance is separate from the minimum composed-validator work boundary.

The complete Tier 1 artifact projection occupies 7,311,970 bytes, leaving
1,076,638 bytes within its separate 8 MiB bound. The maintained inventory has
446 functions; canonical unit emission observes 448 units, below the 700-unit
limit. The resulting artifact is 7,727,799 bytes, SHA-256
`86f3eb7934b65a5f9ef6d647f42ac79fb68293fc7caf13fbf06490ec7e6403a8`.
These are Tier 1 projection/container measurements, not source-driven Tier 2
artifact production.

The normalized `read-nominal` target contains 222 reachable functions, 11,919
instructions and 12 structural record types (19 total type rows), in 561,019
bytes. The `compile-elf` backend target contains 244 functions, 16,111
instructions and four structural record types, in 794,236 bytes.
Reachable closure size is distinct from total compiler inventory size.

Public `status` revalidated the candidate graph. Its full kernel validation
consumed 8,629,356 of 10,000,000 work units. This is aggregate kernel semantic
proof work, not an attributed count for one callable helper, an evaluator
operation count, or the source carrier allowance. The remaining 1,370,644 units
and both carrier bounds must be monitored before the next compiler expansion.

## Storage decision

Preflight found 595 GiB free, 55 GiB available RAM, a 6.4 GiB Cargo target,
7.6 MiB of retained artifacts, 416 project packs occupying 246 MiB, and 553 MiB
of Git objects. Public updates preserve repository/package/module identities and history.
The existing private search parameter was explicitly renamed without changing its
identity. Ordinary contract edits require new positional children to append, while
affine buffer parameters must remain a suffix. The two namespace helpers therefore
received new declaration identities with the required signatures; their superseded
owned closures were explicitly retired after callers changed. This is ordinary
graph evolution, not a project reseed. The final working pack directory contains
501 files occupying approximately 267 MiB, about 21 MiB above preflight.
Physical compaction is not a capacity prerequisite for this slice.
Defer rewriting the consolidated pack while storage remains ample; retain the
small transaction packs and measure their incremental cost. No history horizon,
project reseed, retirement pruning or Git history rewrite is performed.

## Verification obligations and remaining work

Independent directory controls cover byte ordering, domain order, shared prefixes,
first/last/missing keys, nonzero offsets, and invalid ranges. Compare namespace
parents against the independent kernel projection for every admitted class.
Whole-carrier positive, layer-reaching negative and exact resource-boundary
controls continue on VM, reference, Rust-native and selfhost-native paths.

The selfhost-native validator is still compiled through Rust-normalized Tier 1
input. This slice changes no feature eligibility and establishes neither E1-final
nor Tier 2. The next extension admits the remaining namespace owners and their
contextual binders; E2/E3, the full declared region/witness profile, source-driven
ELF/artifact fixed points and same-process native reclamation remain required.
