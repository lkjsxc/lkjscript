# Capability Status Authority

## Purpose

Define the deterministic status graph that prevents one public capability from
being Current in one authority and Accepted, Deferred, Rejected, superseded, or
historical in another without an explicitly different capability identity.

## Status

**Current** for `LKJ-DOC-STATUS` validation and the version 1 registry in
`meta/config/capability-status.json`.

## Registry

The registry identity is `lkjscript.capability-status` version `1`. Each closed
record contains:

- a stable capability ID with its public schema or contract version;
- one closed status;
- the exact interface, schema, or command being classified;
- one authority document;
- one evidence or acceptance-gate document; and
- a sorted set of public claimant documents.

Capability IDs separate bounded slices from broader targets. For example,
`agent-foundation/1` and `semantic-source-schema/1` are not aliases. An internal
file move does not create a new capability or status.

## Claim Directives

Every registered claimant repeats the exact status inside its `## Status`
section:

```text
<!-- LKJ-STATUS id=agent-foundation/1 status=current -->
```

The checker reads only this exact machine form. Natural-language words,
historical evidence, conditional acceptance gates, code examples, and scoped
Edition statements do not become status claims.

The closed statuses are:

- `current`;
- `accepted-target`;
- `accepted-contract`;
- `accepted-selection`;
- `experimental`;
- `deferred`;
- `rejected`;
- `superseded`; and
- `historical`.

## Validation

`check-docs` and therefore `quiet verify` fail on:

- unknown schema or registry version;
- unknown, malformed, duplicate, or unsorted capability records;
- unknown or duplicate claimant paths;
- a missing authority, evidence path, or claimant;
- a directive outside the claimant's Status section;
- an unknown, missing, duplicate, or mismatched directive; or
- a Current capability without an evidence path.

Diagnostics sort by path and capability ID. The registry classifies status; it
does not promote a capability or replace the named semantic authority.

## Change Rule

Update the semantic authority and acceptance evidence before changing a
registered status. Change the registry and every claimant in the same coherent
revision. Historical records retain their original scoped claims and do not
become claimant documents merely because they contain status vocabulary.
