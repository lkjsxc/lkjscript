# Typed Capabilities

[Authority](../../operations/status-authority.md)

## Status

**Current.** Host-provider authority is represented by closed, unforgeable,
first-class capability values. A package declares the exact capability union it
may grant to its targets. A target receives only the capability parameters in
its `main` declaration.

## Closed Capability Set

The Current capability kinds are:

- `Arguments`
- `Clock`
- `Entropy`
- `FileSystem`
- `Network`
- `Sqlite`
- `Stdio`
- `Terminal`

Their stable source type is `Capability/ Kind /Capability`. Capability kinds are
sorted by the spelling above wherever order is canonical.

## Value Semantics

A capability value is immutable, copyable, and unforgeable. It has no source
constructor, literal, integer conversion, equality operation, object identity,
or `drop` operation. It may be named, passed, returned, or stored like another
copy value. Possession delegates exactly one provider authority.

A capability-bearing `main` declares `sig/`, then `params/`, then its body. The
signature and parameter types must agree exactly. Capability kinds must be
unique and sorted. A pure zero-parameter `main` omits `params/`; no omitted
parameter is inferred.

```text
main/
sig/
Capability/
Arguments
/Capability
Capability/
Stdio
/Capability
->
Unit
/sig
params/
arguments
Capability/
Arguments
/Capability
stdio
Capability/
Stdio
/Capability
/params
...
/main
```

Ordinary functions use the existing exact `sig/` and `params/` representation.
There is no ambient capability lookup and no implicit parameter insertion.

## Authority-Bearing Operations

Provider acquisition and ambient host services require an exact capability as
argument zero:

- `Arguments`: `argc`, `arg`
- `Clock`: `sys-now-ms`, `sys-wait-ms`
- `Entropy`: `sys-random-fill`
- `FileSystem`: `sys-open-read`, `sys-open-write`, `sys-open-append`,
  `sys-open-create-new`, `sys-open-dir`, `sys-path-exists`, `sys-rename`
- `Network`: `sys-socket`
- `Sqlite`: `sys-sqlite-open`, `sys-sqlite-backup`
- `Stdio`: `print`, `flush`, `read-byte`, `write-byte`, `write-str`,
  `stdin-handle`
- `Terminal`: `sys-tty-guard-save`, `sys-tty-guard-clear`

Operations on an already acquired `Handle` do not redundantly require a
provider capability. `sys-sha256` is deterministic computation, and structured
`exit` is language control; neither receives a capability.

## Package And Execution Contract

`lkjscript.package.json` contains a sorted, unique subset of the closed kinds.
Compilation computes every target's exact `main` requirements. A requirement
absent from the package is a package error. Extra package declarations are not
injected into the target.

Bytecode records the sorted exact requirements and main arity. VM and native
entry validate supplied values before any source effect. Missing, duplicate,
extra, or wrong-kind grants fail closed. Capability values are process-local
and are never serialized into source, package locks, native images, or caches.

All language, HIR, verified-SSA, bytecode, package, runtime-call, and native
artifact identities include this contract. Stale identities are rejected; no
numbered schema, zero-argument alias, or compatibility decoder remains.

## Resource And Backend Rules

Capability checking is allocation-free and bounded by the closed eight-kind
set. It does not weaken fuel, deadline, heap, handle, root, executable-memory,
or proof limits. A forced tier either validates and enters generated code with
zero fallback or rejects before source effects. Auto tiering may execute a
capability-bearing function only in a tier that implements the exact type and
operation contract.

## Rejected

- ambient process authority;
- string capability names in source types;
- forgeable integer tokens;
- inferred or silently injected parameters;
- wildcard package grants;
- granting package declarations not requested by the selected target;
- retaining old host-operation arities as aliases.
