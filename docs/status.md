# Current status

Updated 2026-10-22 (UTC). Accepted typed meaning is the sole editable authority.
Production bytecode is checked against an independent canonical interpreter.
Verification has no three-minute requirement; elapsed time is an observation.

## Capabilities

One checkout and one leased Cargo target serve admitted packages/programs and
one current schema epoch. Public tx preserves symbolic identities, runs
impact-scoped differential tests and publishes atomically. Safe Rust, denied
lint invariants and the no-Python policy remain intact.

The lkjscript-written Tier 1 backend consumes Rust-admitted LKJCI1 profile 4 and
emits standalone Linux x86_64 ELF or current .lkja. Restricted E1 recognizes
interfaces, operations/parameters, requirements, function R/E binders and
TaskFunction types. Source admission does not replace Rust normalization.
The [self-hosting specification](spec/selfhosting.md) owns the capability matrix.
Native heap allocations remain process-owned until exit; phase reclamation is
still required.

## Current milestone

Both native binary-filter emitters read stdin to EOF using a private growable
mapping, without the former 16 MiB transport quota. Checked geometric growth
may move the mapping only before a language descriptor exists; the final mapping
stays stable through execution and output. A scratch-byte probe detects exact-fit
EOF without unnecessary growth. I/O failure, allocation failure and usage retain
distinct diagnostics, with no compiler output published before successful admission.

This transport capability precedes larger compiler-format inputs. It changes
neither LKJCI1 profile 4 nor LKJSG001. The 8 MiB format bounds, finite widths,
native byte-operation/image limits and semantic checks remain separate work.
The [decision](decisions/20261022-native-input-growth.md) records the bootstrap,
pointer invariant, behavioral controls and complete carrier identity.

Ordinary Rust execution policies retain the removal of default cumulative
instruction, modeled allocation, collection-item and capability-call quotas.
Explicit quotas still use checked precharge; passive observations grant no capacity.
Live frame/stack limits and cancellation remain enforced. The
[execution decision](decisions/20261022-default-cumulative-execution.md) owns those
controls. GC preparation and verification evidence also retain their preceding
removal of default aggregate quotas; physical compaction preserves immutable history.

## Maintained compiler baseline

The authoritative project has 62 modules, 442 pure functions and 23 externals,
28,550 owners, 27 types and 10,202 retirements at
rev_a52428252252cf7cb597e2316da6e1ee1cff948a446323484724b1cc5e146985.
Semantic state:
semantic_state_dd7af6f9c105d7b87f3e47270713bfcda165e78280168e9579fa501289549a98.
The complete LKJSG001 carrier is 6,898,767 bytes, SHA-256
4488fc83fc5ede9865069bab3c5063277cafd58ba458b1b844d24f85aad14955.

Mode-1 input occupies 7,746,716 bytes for 467 units, leaving 641,892 bytes under
its current bound. Public rebuilt current-validation reports 9,173,945 aggregate
semantic-work units across separate local and composed-source validation operations.
That sum does not measure headroom under a single admission ceiling. Direct kernel
work and native allocation/RSS remain distinct observations.
The source transaction preserves existing module, function and parameter identities.
No retirement deletion, history truncation or project reseed occurs.

## Verification and next work

Both emitters pass exact-content inputs through 20 MiB plus 17 bytes, short transfers,
read errors and explicit address-space refusal. An exact 4 MiB input succeeds under
an 8 MiB address-space limit. Supplemental debugger controls force a moving remap
and preserve the pending byte for both emitters. GDB is not a gate dependency.

The maintained Tier 1 compiler reproduces native C2=C3 and identical artifact bytes
through the VM, reference and emitted compilers. These consume Rust-normalized
inputs; they do not establish the source-driven E2/E3 fixed points. The complete
carrier hash and separate normalized-input hashes are recorded in the decision.

Publication requires exact staged-tree bootstrap, tools/selfhost/verify.sh, every
gate in full --fresh and release-target inspection. Source/test changes select
the same full gate set for changed; do not duplicate that complete checker run.
Terminal evidence is under .artifacts/selfhost/native-input-capacity and
.artifacts/lkjscript-dev/latest-receipt.json. Git records normal integration.
No public product release or deployment is implied.

Next close the mode-1 LKJCI1 producer/consumer capacity path: checked offsets and
conversions, finite record progress, explicit allowance semantics and larger-input
controls. Its remaining byte headroom is the closest format constraint. LKJSG001
fuel/count bounds and kernel callable-flow work follow their own complete audits;
they need not form a single format rewrite. Inventory the active compiler profile
and advance E2/E3 without waiting for unrelated horizontal E1 families.

WS1 and Tier 2 remain incomplete. Remaining obligations include authoring/inspection
quotas, recursive depth limits, custody/type encodings, source-driven E2/E3, the
declared region/closure/witness profile, complete-carrier-bound source C2=C3 and
A1=A2=A3, behavioral mutations, independent full-profile programs and measured
same-process native reclamation. [Verification](spec/verification.md) owns final acceptance.
