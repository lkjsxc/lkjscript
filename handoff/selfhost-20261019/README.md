# Semantic self-hosting handoff

The owner requested a pause at a clean checkpoint and a GitHub handoff to another
agent. This is a handoff branch, not a claim that Milestone E or F is complete.
The accepted implementation is on `main`; the files here preserve evidence and
unadmitted next-slice drafts. No hosted release, deployment, or artifact promotion
was requested or performed.

## Accepted checkpoint

- Git commit: `13771737a4a9bc49179927fdc0eaaea68003eee1`.
- Git tree: `8f6d26f14248f97deb2cf97d0b5810a6ac0de01f`.
- Authoritative compiler revision:
  `rev_0bd74eef27ed085a6c9fb29be9af37de0d67f649413247e425b66d12687220db`.
- Semantic state:
  `semantic_state_5be96f934c6465dce05b9489aed5f6aa72a10f0d5fd73457c354a4a174894368`.
- Complete source inventory: 20,001 owners, 18 types, 6,125 retirements,
  no dependencies; 307 canonical units containing 305 functions.
- Official acceptance: passed — changed 2/2 fresh gates, full 29/29 fresh gates
  with stable input (1,317.34 seconds), release-target policy passed. All eight
  workspace harnesses completed; the library passed 2,604 tests, including all
  48 selfhost tests and ten native runtime tests (five library tests ignored).
  CLI passed 347 tests (one ignored, one filtered)..

Read `artifacts/accepted-e-source-bindings/receipt.json` and its adjacent bootstrap,
changed, full, release-target and harness receipts for exact provenance. Those
receipts bind the accepted implementation commit above, not the later handoff
commit. The tracked status file was updated before acceptance. Historical
`running` or failed scratch receipts elsewhere in the archive are not acceptance.

## What is implemented

Milestone D remains accepted at `108906b51259f00fae4c4fd1c53d16a05b3df1b5`.
Its evidence is in `artifacts/accepted-d-artifact/`. E1 source-record decoding was
accepted at `c282cfcabcdc4ad41bae1e7ff9652eb63e5d5f6f`; see
`artifacts/accepted-e-source-records/`.

The new E2 slice publicly authored 18 functions in four modules under
`tools/selfhost/`: `source-map`, `source-state`, `source-history` and
`source-object`. They recompute current logical map roots and state/revision
identities, decode dependency/retirement records, and bind canonical objects to
exact directory identities. Independent current-codec/root oracles and
VM/reference/native comparisons cover the full compiler inventory, nominal
fixtures, malformed input and exact capacity boundaries. These checks do not
establish reference existence, acyclicity, typing, custody, effects or witnesses.

A measured host admission bottleneck was fixed in
`src/platform/kernel/owned_associated.rs`: the first traversal context and pending
node now use fixed inline storage. On the same source, cumulative affine metadata
fell from 66,060,451 to 18,832,593 bytes, with the 67,108,864-byte limit unchanged.
Total validation work remained 1,777,039. All spilled and visited entries remain
metered; authority is reread rather than cached. This is not lkjscript Region
reclamation or an RSS measurement. The source test wrappers were also factored to
fit the existing reference callable-flow limit, preserving all assertions and
complete source closure.

## Current fixed points are Tier 1

The entrypoint still consumes Rust-admitted normalized profile-4 input.
No Tier 2 source-graph compilation or semantic fixed point has been demonstrated.

| Observation | Bytes | SHA-256 |
| --- | ---: | --- |
| C2 = C3 ELF, 244 functions / 16,111 instructions | 2,519,420 | `b1c4371af385bf91421e66dee2a5a7a481c1a2747f34ce786a161354ba0623ce` |
| A1 = A2 = A3 `.lkja`, complete 305-function source | 5,846,556 | `c91054d5ee06904fb06678d5bc9539577c90213208da43b273117a5c2b359dc7` |
| Mutated-source `.lkja` | 5,847,955 | `3358bab74489a0d5697dde0f68b5cef44ba6892a3b74a1612a32f229b39cd6c6` |

The expanded artifact input was 5,538,441 bytes, SHA-256
`4364f7d2d1a979b2f93fffa88f7a83018adcf6b11ccce967b9334c74504ac976`.
VM and reference cumulative modeled allocations were 42,985,302,554 and
48,883,743,596 bytes. This strengthens the need for bounded phase-local lexical
Region tables; it does not measure peak memory. Both ELF and artifact mutated
compiler behavior passed. See `artifacts/source-binding-stage-observations.log.gz` (or the restored log).

## Resume the original objective

[Copyable next-agent prompt](NEXT_AGENT_PROMPT.txt).

Read [the owner's complete execution contract](objective.md), root `AGENTS.md`,
`docs/status.md`, `docs/spec/selfhosting.md` and the actual current repository
state. Do not reset to an old SHA. Preserve the Tier 1 / Tier 2 distinction.

The next dependency is a raw source-graph input and semantic frontend. The
existing artifact source-object table is not the full authoritative source:
its reference-owner map excludes owners such as modules. Simply deleting its
compiled-unit section would silently narrow S.

Unadmitted drafts preserved in `artifacts/`:

- `source-index-draft.lkjc`: six bounded sorted-table validation/binary-lookup
  functions. Lookup assumes ordering was separately established and grants no
  semantic authority. Keys are 1–32 bytes, strides at most 128, at most 100,000
  rows, and input at most 8 MiB.
- `source-input-draft.lkjc`: six raw profile-5 framing/identity inspection
  functions, generated by `draft-source-input.cjs`. It joins complete logical
  owner/dependency/retirement tables, a canonical object directory and revision
  metadata; it accepts only Owner/Type inspection initially. Imports and other
  source domains reject explicitly. It returns a state digest, not an artifact.
- `selfhost_source_input-draft.rs`: test-only complete-snapshot serializer for
  that draft. It has no normalized-program, compiled-unit or flow-certificate
  input and does not call full semantic admission or linking. Its canonical
  object encoders still perform local record-shape checks. This is an uncompiled
  proposal, not an accepted exporter or a semantic-admission proof.
- `source-region-table-draft.lkjc`: four unadmitted functions exploring bounded
  scalar tables through existing `region-offset`/`region-resolve`, rather than
  requiring new array syntax. Native Region instructions are still unsupported.
- `source-graph-next-research.json` and `source-region-next-research.json`:
  historical research notes, including superseded proposals. Inspect actual code
  before relying on them.

The JavaScript drafts passed syntax checking and the generated proposals passed
AST-preserving formatting. They have not been publicly authored, compiled,
executed, integrated into S or accepted. No reference-resolution implementation
was added before this handoff. The profile-5 layout is provisional; reconcile it
with the full required source-object closure before adopting it.

Recommended next work:

1. Test and publicly author the raw source input/index slice. Use the full
   authoritative source snapshot, independent smaller programs, malformed
   directories and rehashed object/identity failures. Preserve unsupported
   boundaries explicitly. Do not claim state/hash agreement is semantic admission.
2. Resolve every source reference, including unused owners and untaken syntax;
   then implement cycle/parent/scope checks, type inference, affine custody,
   borrows, region brands, effects and exact witnesses. Reuse one canonical
   decoder where practical. Normalization and flow certificates must be built in
   lkjscript. They must not enter through the exporter.
3. Implement measured bounded lexical Region tables and the native runtime
   support they need, retaining exact custody, provenance, bounds and cleanup.
   Cover all of P_self and complete S; intermediate unsupported constructs remain
   explicit limitations.
4. Complete Milestone F from raw S and mutated S': native C1 → C2 → C3 without
   Rust semantic preprocessing or an external VM; prove C2=C3 ELF and A2=A3
   artifacts, strict loading, positive execution and negative semantic rejection.

## Evidence and local restoration

`artifacts/` is a frozen copy of this task's `.artifacts/selfhost/` contents.
`manifest.json` records original and stored file sizes and SHA-256 values.
Text logs use lossless gzip storage so their original whitespace and digest
identity are preserved; the restore script reconstructs the exact original paths. The archive deliberately
retains failed attempts as failures; only the accepted receipt directories and
terminal official results establish acceptance. Original paths inside receipts
are retained so their bytes and hashes remain unchanged. There are no gate
executables in this handoff.

With this handoff branch checked out in the canonical repository root, run:

```sh
node handoff/selfhost-20261019/restore-artifacts.cjs
```

This verifies every archived file and restores absent files under
`.artifacts/selfhost/`; identical files are left alone and conflicting files
cause a refusal. Return to current `main` before new implementation work. Keep
one checkout and one shared `$PWD/target`, below the owner's 8 GiB total bound.
Do not rerun historical authoring scripts against main without reviewing their
hardcoded source guards and intended revisions.

Candidate/verifier processes must not inherit publication credentials. Use
Cargo jobs 8 and checker jobs 4 independently. For each accepted slice, update
tracked status before committing, then bootstrap the clean exact commit and run
changed, fresh full and release-target checks. Discover actual gate counts and
join the owning process; never infer completion from a timeout or lock file.
Retain complete harness evidence before the default checker removes scratch.
The existing acceptance runner/watcher are archived, but the receipt generator
contains E2-specific test counts and must be updated for later slices.

This goal is paused at the owner's request, not complete. The next agent should
continue Milestones E and F without redefining success around the accepted E2
foundation. No active task build or validation job remains at handoff.
