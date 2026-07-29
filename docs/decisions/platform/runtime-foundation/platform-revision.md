# Global Platform Revision

## Purpose

Define the sole lkjscript-owned public compatibility number and its relation to
exact content-addressed contracts.

## Status

**Accepted Contract.** `LKJ-PLATFORM-REVISION` makes the file, monotonicity,
public-change, fixed Cargo metadata, and forbidden subsystem-number checks
Current when its focused and canonical gates pass. Envelope migration remains
Current only for each named producer and consumer covered by tests.

## One Number

`meta/platform-revision` contains one canonical nonzero unsigned decimal `u64`
and one newline. It is the only monotonically increasing lkjscript product
number. The initial accepted value is `1`.

The revision increases whenever an accepted public platform contract changes,
including source, Semantic Source, daemon control, application manifests,
bytecode artifacts, packages, database formats, GUI semantics, or host provider
contracts. Several coherent implementation commits may retain a revision only
when none changes an accepted public contract.

Language, schema, protocol, ABI, package, database, GUI, runtime, and format
subsystems never own independent increasing versions. Cargo package versions
are fixed private tooling metadata `0.0.0`; they are not product identity.
External standards and dependency releases retain their factual names.

## Exact Identity

Revision orders coherent platform states. A full `ContractDigest` proves exact
structural and semantic equality. Every Current public envelope carries the
current `platform-revision` and the exact digest for its stable schema. A
revision match never authorizes bytes with a digest mismatch.

A stale revision or digest fails closed with exact update or rebuild guidance.
No old protocol revision is negotiated and no compatibility parser, alias, or
translation branch remains Current.

Repository commits and retained immutable evidence preserve prior revisions;
historical bytes are never rewritten to satisfy the Current rule.

## Operational Counters

Application incarnations, source/repository/state revisions, transaction
sequences, epochs, leases, dimensions, and internal reusable-slot generations
are operational identities or ordinary data. They do not order the platform.
Public stale application identity uses `incarnation`, never product-facing
`generation` or `version`.

## Machine Rule

`LKJ-PLATFORM-REVISION` verifies offline from the working tree and first
available Git parent:

- the canonical file exists, parses as nonzero `u64`, and has exact bytes;
- the value never decreases and a registered public-contract change strictly
  increases it;
- private workspace crates all inherit fixed Cargo metadata `0.0.0`;
- forbidden lkjscript-owned subsystem compatibility-number fields do not enter
  Current authored contracts; and
- registered Current public envelope producers retain both the revision and an
  exact contract digest.

A root repository validates shape without requiring a parent. Missing local Git
parent objects fail closed for a non-root commit. Ordinary verification never
requires network access.
