# Public-Fact And Documentation-Authority Cut Evidence

## Status

**Historical evidence.** This record describes the completed repository cut at
the named commit. It grants no Current capability, alias, decoder, fallback, or
freshness beyond that commit and environment.

## Identity And Scope

- Starting repository commit: `00194ba8cb8decb562e27b5b8831e83310912379`.
- Durable Agent Guide commit: `5edd9ed8220f1e812ebaffc84fb61716d0c8c4f7`.
- Accepted authority contract commit: `4803fed82408d35d033b98d10a10dac921289ef5`.
- Implementation commit: `9f72aeb4cbe6587be10aa0be741dd9c6d82994b4`.
- Platform revision moved monotonically from 18 to 19.
- Repository-graph contract digest became
  `1543bef254b336a5f7e507744b9bc5e2277b0ce4760f0cfc092379fca6645e00`.
- Public-facts contract digest was
  `c0b1013dfe8d5f80cd4ec6f4311322d72e8010dedff301a0ece298f6f12cfae1`.

The cut removed `meta/config/capability-status.json` and its tooling. It added one
strict manifest plus canonical shards containing 52 facts: 36 Current, 10
Accepted Contract, three Accepted Target, one Accepted Selection, and two
Superseded. The generated inventory expected 106 exact projection markers. No
generic outside-interface exclusion remained.

## Implemented Boundary

Each public fact bound stable identity, kind, closed status, scope, positive
interface, explicit exclusions, authority, implementation anchors, typed
evidence, projections, dependencies, invalidations, platform cut, and contract
digests. Projection identity covered the public-facts contract, shard, complete
fact record, and exact unique authority/anchor/evidence content with only an
exact compact projection-marker line normalized out.

The loader rejected unknown and duplicate fields, unlisted shards, duplicate
facts, noncanonical IDs and digests, path and symlink escape, missing Current
evidence, missing exclusions, invalid references, dependency/invalidation
cycles, excessive files, collections, aggregate members, JSON bytes, and unique
content bytes. Reads used the remaining aggregate allowance. Reports used one
bounded atomic JSON publication and were emitted by `check-docs` only after the
whole documentation check succeeded; `check-docs --expected` remained a
separate generation interface.

The repository graph retained fact, status, interface, exclusion, authority,
implementation, typed evidence, test, projection, dependency, and invalidation
relations. Graph results carried a base Git revision and exact emitted-closure
identity. Explain, context, impact, and tests output had exact serialized byte
checks. Focused impact reached registered projections without unrelated capsule
closure, and implementation-test evidence was queryable through `structure
tests`.

Current-facing entry and operations documents were repaired for collector-free
runtime wording, removed tracing commands, borrowed text, persistent lists,
resource-token scope, daemon/standalone scope, malformed migration wording, and
numbered-generation residue. Historical records gained explicit envelopes or
recorded-baseline language. The broader documentation-authority contract stayed
Accepted because complete example execution, evidence reachability/freshness,
architecture derivation, and Historical-scope linting were not implemented.

## Environment

Recorded at `2026-08-04T11:29:02Z` on Linux 7.0.0-27-generic x86-64 with
`rustc 1.96.0` and `cargo 1.96.0`.

## Exact Positive Evidence

At implementation commit `9f72aeb4cbe6587be10aa0be741dd9c6d82994b4`:

- `cargo run --locked -p lkjscript-xtask -- quiet verify` passed twice. The
  measured run took 120.208 seconds and reported 76 successful test-result
  groups, below the fixed 130-second threshold.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- `cargo test --release --locked -p lkjscript-xtask public_facts` passed 21
  focused tests, and release `check-docs` passed.
- AddressSanitizer passed the same 21 focused tests under nightly Rust. Nightly
  emitted unrelated deprecation warnings for existing `fetch_update` uses.
- `check-docs` passed twice in 2.031 and 2.076 seconds. Both runs produced
  byte-identical inventory digest
  `b33fe551546cc0ed2f7815dab7898a2565187f23e1a5466eafced5d1605d430d`.
- `structure graph --json` passed twice in 1.334 and 1.400 seconds. Both runs
  produced 4,096 nodes, 9,707 edges, 4,523,037 bytes, input identity
  `3106ae6e47aa6e90d362cd068dda2ad18bd4df269a6aecaa55eb086376003356`,
  and byte-identical output digest
  `47420be15e5c6845fd2b3f4397cc0031747f4c406f208127194b87b0ee84dbf0`.
- `structure tests fact:public-fact-foundation` returned both registered public-
  fact test files and typed `tests` edges.
- `structure tests fact:repository-graph-context` returned the graph test file
  and typed `tests` edges.
- A transient broken link in an unbound documentation fixture made `check-docs`
  fail while the prior generated report digest remained unchanged; the fixture
  was restored byte-for-byte.

## Negative And Unavailable Evidence

The full graph reached its configured node limit and reported `truncated: true`;
focused fact queries also preserved that inherited truncation flag. Peak RSS was
unavailable because `/usr/bin/time` was absent. `cargo-deny`, `cargo-llvm-cov`,
and Miri were unavailable. Docker was unavailable as an acceptance gate because
the repository had no Dockerfile. No fuzz harness existed. Privileged install,
external process, and non-Linux acceptance were out of scope.

Residual generic codecs remained authority-selected after the cut, but no codec
contract or implementation was added. Persistent structural lists, indirect
generics, general pools, and complete owner-home selection remained incomplete.
