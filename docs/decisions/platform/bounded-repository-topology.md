# Bounded Repository Topology

## Purpose

Define the repository structure contract that must exist before automated source
reorganization, repository graph generation, or multi-agent work publication.

## Status

**Accepted Implementation Contract.** No topology checker, manifest validator,
or audit JSON described here is Current. Existing Edition 1 source limits remain
Current and unchanged. The first implementation slice is the checker and its
focused fixtures; it must not rewrite the repository.

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

The accepted normal bounds are:

| Rule | Bound |
| --- | --- |
| authored Markdown physical lines | at most 200 |
| authored Markdown UTF-8 bytes | at most 32 KiB |
| normal prose/code-comment columns | at most 120 Unicode scalar columns |
| immediate tracked authored entries in one directory | at most 16 |
| authored directory depth | warning above 8; hard failure above 12 |

Physical lines include blank lines and fenced-block contents. The column rule
excludes integrity strings, unavoidable external URLs, generated snapshots, and
immutable evidence rows, but those lines still count toward line and byte
bounds. Exclusion is rule-defined, not an exemption.

The checker also warns on more than 12 top-level repository entries, more than
12 immediate authored child directories, or more than 64 outgoing repository
relationships from one manifest node. Warnings require review but do not change
the hard 16-entry width or depth-12 limits.

These repository limits do not change Edition 1 language semantics. In
particular, source depth 8, form children 16, 384 tokens per file, 8 top-level
forms, 15 product fields, and 16 combined immediate source-directory entries
remain Current until their separate aggregate-budget migration gate passes.

## Semantic Capsules And Manifest

A split document is a semantic capsule, not a numbered overflow fragment. Its
authority page is a clearly named record or `README.md` with status vocabulary
and a strict capsule manifest. Each manifest entry has a unique stable key,
relative path, purpose, provenance, authority, status, and ordered relationship
to sibling capsules. Unlisted capsules and duplicate paths fail. Moves update
all repository-local links in the same change; compatibility aliases are
rejected.

Directory names express product semantics such as `platform`, `execution`, or
`capabilities`. Arbitrary buckets, hidden fan-out, line packing, and one-line
link farms do not satisfy the bound.

## Checker Interface

The accepted commands are:

```text
cargo run --locked -p lkjscript-xtask -- check-structure
cargo run --locked -p lkjscript-xtask -- check-structure --audit-json target/repository-audit.json
```

The first command emits deterministic diagnostics sorted by rule ID and path.
The second additionally writes canonical UTF-8 JSON under `target/`; normal
stdout remains suitable for humans and no tracked file is generated.

Initial rule IDs are closed and stable:

- `LKJ-REPO-PROVENANCE`, `LKJ-REPO-MANIFEST`, and `LKJ-REPO-LINK`;
- `LKJ-REPO-LINES`, `LKJ-REPO-BYTES`, and `LKJ-REPO-COLUMNS`;
- `LKJ-REPO-WIDTH`, `LKJ-REPO-DEPTH`, `LKJ-REPO-TOPLEVEL`, and
  `LKJ-REPO-FANOUT`; and
- `LKJ-REPO-GENERATED-LOCATION` and `LKJ-REPO-TEMPORARY-DEVIATION`.

Audit JSON uses identity `lkjscript.repository-audit`, version `1`, and contains
repository revision, manifest identity, policy identity, sorted findings,
counts by provenance/rule/severity, checked paths, and deterministic limits.
Unknown versions and fields fail at consumers.

## Policy Coverage

The checker covers all tracked authored Markdown, every directory containing a
tracked authored entry, repository-local Markdown links, the strict manifest,
and generated-output locations. Later language/source and code-specific rules
may join the same audit only with explicit rule IDs and provenance semantics.
Symlinks cannot evade containment or depth accounting.

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
