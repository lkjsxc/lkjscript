# Semantic development repository campaign ledger

This ledger records reproduced checkout facts and selected campaign decisions. It is evidence and
handoff state, not semantic authority. Measurements and verification results are updated only after
the named command has completed.

## Starting checkout and instructions

- Audit date: 2026-08-19 UTC.
- Branch: `main`.
- Starting commit: `d9f2993d7335c9e177b5f0ed34247bf6a49595ea` (`Finalize lkjwork campaign provenance`).
- Preceding implementation commit: `2af914ac8d338879264b96378fe84630848390b9`.
- Relation to campaign baseline: exact match.
- Initial worktree changes: modified `AGENTS.md`; untracked `prompts/202608191427.md`.
- Root-policy SHA-256: `7e9c40d1bc29b4528ef7c8fe67ee5b70c2c941016f9bbed6434a276ba0309733`.
- Root-policy size: 41,634 UTF-8 bytes, 1,030 lines.
- Campaign-prompt SHA-256: `1f88a6da6723eb6dcb9e5562834b6ad5028a0b12706cc6d2f41f6fec0a55dc2a`.
- Campaign-prompt size: 241,205 UTF-8 bytes, 5,793 lines.
- No deeper `AGENTS.md` exists in this repository.
- The modified root policy and untracked campaign prompt are user-delivered in-scope inputs. They
  are preserved and are not attributed to this implementation.
- Cargo metadata succeeds for one stable Rust 2024 package with library, `lkjscript`, `lkjwork`, and
  nine integration-test targets. Existing locked dependencies are `base64`, `blake3`, `fs2`,
  `getrandom`, `serde`, `serde_json`, and the dev-only `tempfile`.

## Reproduced active baseline

| Boundary | Active identity | Existing direct predecessor rule |
|---|---|---|
| workspace protocol / machine schema | 11 / `lkjscript-machine-schema-v11` | 10 and older reject |
| workbench / editable document | 2 / 1 | packet 1 and `plan` reject |
| workspace artifact | 7 / `LKJTSM\0\x07` / `lkjscript-tsm007` | 6 and older reject |
| workspace HEAD | `LKJHEAD9` | `LKJHEAD8` and older reject |
| reusable release | 2 / `LKJREL\0\x02` | 1 and older reject |
| application | 5 / `LKJAPP\0\x05` | 4 and older reject |
| durable instance | 3 | 2 and older reject |
| runtime session | 2 | every other version rejects |
| lkjwork machine / export | 1 / 1 | every other version rejects |

The checked `applications/lkjwork/lkjwork.lkja` is 163,670 bytes. Strict public inspection reports
application digest `9d5ebe527719aa4c68b471cc10f9113df421385997113a08fbd1a6eae4650c4d`,
root release `7f8fac0efd12aac9562a4498bdc0c3cbb42c68838b09f824623a0d7455194c43`,
one release, no graph edges, 3,232 flattened semantic items, and five passing immutable application
case declarations. Its file SHA-256 is
`52b8f9a5961b7a978832fa32ae28021223162d5acbdaeb660d04e326c69370b6`.

## Reproduced baseline ownership map

This map records the ownership defect found at the starting commit; superseded entries are retained
only as baseline evidence. The selected current ownership is summarized below under **Selected
cutover** and in `docs/architecture.md`.

- `src/schema.rs`: closed semantic types, declarations, operations, and stable tags.
- `src/graph.rs`, `src/validate.rs`, `src/transaction.rs`: snapshot graph, whole-candidate
  validation, proposal normalization, allocation, and validate/apply parity.
- `src/artifact.rs`: canonical workspace snapshots.
- `src/persistence.rs`: immutable workspace revision files, idempotency, lock, and HEAD publication.
- `src/diff.rs`: semantic adjacent-snapshot diff.
- `src/workbench/`: bounded contexts, editable proposals, and review views.
- `src/engine.rs`, `src/protocol.rs`, `src/contract.rs`, `src/machine.rs`: logical workspace owner,
  protocol, executable contract catalogue, and strict JSON projection.
- `src/release/`: reusable release projection, validation, cases, and artifact publication.
- `src/application.rs`: application composition, profile mappings, cases, artifact validation, and
  typed public values.
- `src/instance.rs`, `src/runtime.rs`: durable product state/history and topology-neutral execution.
- `src/bin/lkjscript.rs` plus `src/bin/lkjscript/`: public process adapters.
- `applications/lkjwork/build.py`: practical maintained owner of the lkjwork graph and build facts;
  this is the superseded authority that must be deleted.
- `applications/lkjwork/lkjwork.lkja`: checked immutable distribution application.
- `applications/lkjwork/bindings.json`: derived native-client descriptor.
- `src/bin/lkjwork.rs` and `src/bin/lkjwork/`: native boundary, deployment locator, application-value
  construction/decoding, rendering, and backup transport.

## Baseline facts owned only by `build.py`

The 255,704-byte, 4,869-line recipe creates one workspace and submits one 3,232-item semantic
transaction. It owns:

- package/module declaration creation and every durable declaration/function draft symbol;
- 19 mutation-event variants and seven pure-query variants;
- project, task, lifecycle, hold, note, attachment, activity, paging, filtering, context, decision,
  command, outcome, and helper nominal shapes;
- all function bodies for mutation, resume, queries, deterministic ordering, dependency reachability,
  readiness, pagination, resource-aware context, and attachment reconciliation;
- package entry selection and the 57-item export list;
- release coordinate `applications/lkjwork`, user version `1.0.0`, exact export selection, and the
  `identity_text` release case;
- the complete stateful application profile, all exact type/field/variant mappings, the
  `attachments` immutable-blob requirement and outcome routing, and run policy;
- five application cases: task creation, empty list query, no-pending resume decline, unchanged
  rename, and suspended attachment request;
- test initial state/value construction; and
- generation of the checked application artifact and binding descriptor.

The recipe uses only public commands and a foreground session, but its host-language control flow and
constant tables are still the maintained graph/build source. Renaming, translating, wrapping, or
retaining it as an oracle would preserve the defect.

## Frozen complete workflows

Primary application closure is migrated `lkjwork`:

1. discover and verify its checked semantic development project;
2. orient and request bounded target/function context;
3. validate and apply an exact-base-bound semantic change;
4. observe exactly one automatic revision record and semantic diff;
5. build and test the release/application/product targets;
6. run the public product acceptance and functional/representative workloads; and
7. restart or copy the checkout and reproduce the same authority and artifact.

The dogfood change is the default pure `lkjwork why TASK` query unless checkout evidence proves an
equivalent query already exists. The current artifact has `get_task`, list, next, summary, context,
export, and activity queries but no `why` query, so the default remains selected.

## Candidate architecture and stop rules

The initial selected direction is a direct evolution of the existing workspace rather than a second
project graph:

- reuse `WorkspaceId`, `Snapshot`, transaction validation, full immutable snapshots, and one
  single-writer store;
- add one strict project locator/repository layout around that authority;
- add canonical revision records adjacent to immutable snapshots;
- place durable typed build-target declarations in the accepted snapshot;
- expose one project-oriented public CLI over the existing engine/workbench/release/application
  owners; and
- migrate once through temporary external scratch, retain only accepted repository authority, then
  delete the importer and `build.py`.

Serious alternatives retained for comparison are: a duplicate project layer (reject unless it owns
a distinct necessary authority), journal/object/Merkle persistence (retain only if the 100-revision
large-application workload defeats full snapshots), a closed JSON-only change protocol, the evolved
semantic document, narrow commands, generated bindings versus artifact discovery, and checked
distribution versus build-on-demand.

Do not implement a daemon, database, merge model, conventional source language, broad filesystem or
terminal interface, or derived cache unless the complete migrated workflow crosses a recorded gate.

## Verification state

- Checkout/instruction audit: passed.
- Cargo metadata: passed.
- Root policy and campaign prompt hashes: reproduced.
- Baseline lkjwork artifact strict inspection: passed.
- Full baseline repository gate: passed before implementation.
- Baseline public acceptance, functional corpus, and representative corpus: passed before
  implementation at revisions 30, 85, and 2,700 respectively.
- Semantic project, automatic history, project proposal surface, target graph, migration, builder
  deletion, artifact self-description, and `why` dogfood change: implemented.
- Migrated target reproduction, seven semantic cases, nine product integration tests, public
  acceptance, and pure-query no-write proof: passed.
- One-hundred-change storage workload: passed at revision 107; 21,580,665 retained bytes; shallow
  current open 3.224 seconds; deep reconstruction 332.369 seconds in named warm-host debug samples.
- Final formatting, warning-denied Clippy, full test suite (250 passed, three explicitly ignored),
  and optimized locked workspace build: passed.
- Isolated copied-checkout proof: passed with an initially absent Cargo target directory, deep
  project doctor, seven target cases, two byte-identical target builds, installed-product use,
  acceptance, functional revision 85, and representative revision 2,700 including deep audit.
- Exact current receipts and binary/artifact digests are retained in
  `docs/evidence/20260819-semantic-development.json` and its three raw companion receipts.
- Provider model/token-class/price telemetry: unavailable from repository workflows; no token or
  monetary claim will be inferred from bytes.

## Selected cutover

- Workspace identity is reused as project identity; `.lkjscript/project` is a locator, not a second
  graph.
- Full canonical snapshots plus compact revision records remain authority. Ordinary open lazily
  decodes history; deep doctor is the full reconstruction oracle.
- One project-wide closed JSON change envelope and the existing semantic document normalize through
  the transaction owner; narrow commands are not a second edit model.
- Release, application, and product targets are durable typed graph nodes with exact-ID edges and
  eager candidate validation.
- Application self-description replaces generated native binding constants.
- The checked semantic repository and checked distribution artifact are both retained: one is
  development authority and history, the other is independently validated installable distribution.
- Raw RPC/session remains a distinct conformance/embedding transport. The equivalent `agent`,
  command-local release/application build, procedural examples, and lkjwork builder/bindings paths
  are deleted and reject.
