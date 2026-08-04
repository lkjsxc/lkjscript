# Bounded Repository Topology

## Purpose

Define the repository structure contract that must exist before automated source
reorganization, repository graph generation, or multi-agent work publication.

## Status

**Current.** `lkjscript-xtask structure` implements the bounded topology checker,
strict capsule validation, generated audit, graph, and query foundations.
Existing source limits remain Current and unchanged. The implemented slice is the checker
and its focused fixtures; it must not rewrite the repository.

## Provenance Classes

Every checked path has exactly one provenance class:

- `authored`: human- or agent-maintained source, documentation, configuration,
  and tests whose reviewed bytes are repository authority;
- `generated`: reproducible output with a declared generator and inputs;
- `vendored`: third-party bytes retained with origin, version, license, and
  integrity identity;
- `immutable-evidence`: retained benchmark, experiment, or conformance evidence
  whose bytes must not be reformatted or silently regenerated; or
- `build-artifact`: disposable compiler/tool output that is never tracked.

Classification is explicit in the strict manifest or follows a closed built-in
rule. Ambiguous, unknown, or overlapping classification fails. Generated and
build outputs belong under `target/`; generated material needed by a check is
recreated there and compared with tracked authority rather than written into an
authored directory. There are no permanent exemptions. A temporary deviation
must have an owner, reason, creation revision, expiry condition, and bounded
scope, and the checker still reports it.

## Authored Bounds

The accepted repository-wide bounds are:

| Rule | Bound |
| --- | --- |
| authored text file physical lines | at most 200 |
| authored text file UTF-8 bytes | at most 32 KiB |
| ordinary line width | at most 120 Unicode scalar values |
| immediate tracked entries in an authored directory | at most 16 |
| authored directory depth | warning above 8; hard failure above 12 |

These rules cover Rust, lkjscript, Markdown, scripts, schemas, manifests, test
source, configuration, and CI files. Physical lines include comments, blank
lines, module declarations, tests, and fenced examples. The width rule excludes
only closed, executable classifications: escaped multiline fixture literals in
Rust test paths; isolated integrity records and unavoidable external URLs;
Markdown table/sequence records under `docs/current-state/` and
`docs/vision/experiments/`; and other Markdown data records carrying the literal
suffix `<!-- LKJ-EXACT-DATA -->`. That suffix is valid only for an exact
measured, command, protocol-signature, integrity, or decision-matrix record; it
is not a prose escape. Those lines still count toward file and byte bounds. Audit file records expose
the maximum physical width, maximum ordinary width, and exact-data line count,
so excluded data remains visible rather than disappearing from evidence.

A directory warns above 12 entries. A single-child directory warns unless it is
a stable namespace, target/platform, generated/evidence, edition/schema, trust,
or capability boundary. Top-level semantic items warn above 16 while supported
counting matures. Capsule dependencies and principal public concepts warn above
12 and fail above 16. Cross-capsule dependency cycles fail.

These repository limits do not change the semantics of canonical source. In
particular, source depth 8, form children 16, 384 tokens per file, 8 top-level
forms, 15 product fields, and 16 combined immediate source-directory entries
remain Current until their separate aggregate-budget migration gate passes.

## Semantic Capsules And Manifest

A split unit is a semantic capsule, not a numbered overflow fragment. Every
architecturally significant capsule has one strict `capsule.json`, a stable ID,
purpose, layer, owned concepts, deliberate facade, allowed and forbidden
internal dependencies, tests, decisions, unsafe/capability/provenance status,
verification commands, and context card. Unknown fields fail. Actual extracted
dependencies must be a subset of declared allowances.

A nontrivial capsule has a bounded `README.md` describing authority, invariants,
dependency direction, common agent tasks, verification, and status. Trivial
leaves inherit their capsule. Moves update every repository-local link in the
same change; aliases are rejected. Arbitrary buckets, vague dumping grounds,
wrapper chains, hidden fan-out, statement packing, minification, embedded source
payloads, compression, and macros used only to evade review do not comply.

## Checker Interface

The accepted command family is:

```text
lkjscript-xtask structure audit [--json]
lkjscript-xtask structure check
lkjscript-xtask structure explain <rule-or-path>
lkjscript-xtask structure graph
lkjscript-xtask structure context <target>
lkjscript-xtask structure impact <target>
lkjscript-xtask structure tests <target>
```

Audit succeeds even when findings exist; check fails. Diagnostics sort by rule
ID, path, and evidence. Canonical JSON and full generated projections stay under
`target/lkjscript/`; normal stdout remains human-oriented.

Initial stable IDs include `LKJ-REPO-FILE-LINES`, `LKJ-REPO-FILE-BYTES`,
`LKJ-REPO-LINE-WIDTH`, `LKJ-REPO-DIR-WIDTH`, `LKJ-REPO-DIR-DEPTH`,
`LKJ-REPO-TOPLEVEL-ITEMS`, `LKJ-REPO-CAPSULE-CYCLE`,
`LKJ-REPO-CAPSULE-FANOUT`, `LKJ-REPO-UNCLASSIFIED`,
`LKJ-REPO-GENERATED-PROVENANCE`, and `LKJ-REPO-VAGUE-MODULE`.

The stable `lkjscript.repository-audit` identity and exact graph contract digest contain repository and
policy revisions, every tracked file/directory measurement, item counts where
supported, capsule membership/dependencies, classifications, findings,
provenance, deterministic sort keys, and explicit unsupported analyses. Unknown
contract identities or fields fail at consumers.

## Migration And Policy Coverage

The completed migration used an exact machine-readable ratchet with no new or
worsened finding, monotonic removal, and stale-entry failure. The ratchet is now
deleted because authored hard violations reached zero. Current checks reject
all hard findings directly; no permanent exemption ledger remains.

The checker covers every tracked path, all authored text, containing
directories, local documentation links, strict manifests, provenance, generated
locations, and supported code analyses. An unsupported analysis is reported,
not omitted. Counting uses sorted Git-tracked UTF-8 paths, checked arithmetic,
bounded reads, and explicit symlink handling; filesystem order and wall clock
cannot affect it.

## Acceptance Gates

The contract becomes Current only after focused pass/fail fixtures cover every
rule and boundary, repository links resolve after real moves, audit output is
byte-deterministic, immutable evidence is not modified, generated output stays
under `target/`, and the complete baseline repository passes. The gate records
the exact commit and command.

## Deferred And Rejected

Automatic reorganization and policy-selected rewrites are **Deferred**.
Permanent allowlists, aliases at moved paths, arbitrary numbered buckets,
untracked generated authority, silent truncation, and claiming this contract as
Current before the checker lands are **Rejected**.
