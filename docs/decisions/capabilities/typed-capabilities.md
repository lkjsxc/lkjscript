# Typed Capabilities

[Authority](../../operations/status-authority.md)

## Status

**Current.** Host-provider authority is represented by closed, unforgeable,
first-class capability values. Packages declare the exact capability union
available to targets.

## Closed Set

The source kinds are `arguments`, `clock`, `entropy`, `file-system`, `network`,
`sqlite`, `stdio`, and `terminal`. A type is structural:

```text
capability/
file-system
/capability
```

Capability kinds are unique and sorted wherever order is canonical.

## Value Semantics

A capability is immutable, copyable, process-local, and unforgeable. It has no
source constructor, literal, integer conversion, equality, object identity, or
`drop`. Possession delegates exactly one provider authority.

A capability-bearing main uses a structured signature:

```text
main/
sig/
inputs/
capability/
arguments
/capability
capability/
stdio
/capability
/inputs
output/
unit
/output
/sig
params/
arguments
capability/
arguments
/capability
stdio
capability/
stdio
/capability
/params
...
/main
```

No omitted parameter is inferred and no ambient lookup exists.

## Authority-Bearing Operations

- `arguments`: `argument-count`, `argument-at`
- `clock`: `current-time-milliseconds`, `wait-milliseconds`
- `entropy`: `fill-random`
- `file-system`: `open-file-reader`, `open-file-writer`,
  `open-file-appender`, `create-file`, `open-directory`, `does-path-exist`,
  `rename-path`
- `network`: `open-tcp-socket`
- `sqlite`: `open-sqlite`, `backup-sqlite`
- `stdio`: `print`, `flush`, `read-byte`, `write-byte`, `write-string`,
  `standard-input`
- `terminal`: `save-terminal-guard`, `clear-terminal-guard`

Operations on a typed acquired resource do not redundantly accept a provider
capability. `sha256` is deterministic computation; `exit` is language control.

## Package And Execution Contract

The package manifest contains a sorted unique subset of capability kinds.
Compilation computes each target's exact requirements. Missing declarations
are errors; extras are not injected. Bytecode records exact requirements and
main arity. Execution validates supplied kinds before source effects.
Capabilities are never serialized into source, locks, images, or caches.

No numbered schema, zero-argument alias, old host spelling, compatibility
decoder, wildcard grant, or inferred provider remains.
