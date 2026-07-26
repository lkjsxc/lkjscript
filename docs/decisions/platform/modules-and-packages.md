# Modules And Reproducible Local Packages

## Status

**Current** for exact source modules, private-by-default declarations, explicit
imports/exports, the strict local manifest, canonical lock generation and
verification, source/module/package hashes, and run/disassembly lock checks.
Network and registry resolution are **Rejected** for the Current platform.
Components and remote distribution remain **Accepted Targets**.

## Module identity

A module has no author-chosen alias. Its identity is its normalized UTF-8
`.lkjscript` path relative to the package source root. Absolute paths, `..`,
dot-relative imports, legacy suffixes, symlinks, duplicate module IDs, and
ambient root search are rejected. Absolute host paths never enter an identity.

Declarations are private unless their declaration carries the explicit
`public` visibility field:

```text
def/
name/
parse
/name
public
fn/
...
/fn
/def
```

Imports name one exact module and a sorted, nonempty declaration list:

```text
imports/
import/
src/parser/token.lkjscript#Token,parse
/import
/imports
```

Wildcards, transitive visibility, private names, duplicate names, collisions,
unknown modules, and undeclared names are errors. Imported names enter only the
importing module scope. Equal private or public spellings may coexist in
distinct modules. The compiler uses deterministic internal qualified names but
preserves source-visible names in diagnostics and runtime metrics.

## Manifest

`lkjscript.package.json` is a strict `lkjscript.package` object containing:

- the full package-contract digest;
- package name and contained source root;
- sorted exact module and public-root lists;
- sorted local path dependencies with full expected package hashes;
- a sorted subset of the eight closed provider capability kinds;
- sorted named targets; and
- an optional exact resource-profile name.

Unknown fields, noncanonical paths, missing modules, unsorted/duplicate values,
undeclared public roots or targets, unknown capabilities, path escapes, and symlink
components fail. Each selected target receives only its exact typed main requirements;
a missing package grant fails before execution and extra declarations are not injected.
Only dependencies contained beneath the root package are Current. No network,
registry, home-directory, environment-variable, or current-directory search is
performed.

## Lock and identities

`lkjscript.lock.json` has stable schema `lkjscript.package-lock` and the full
lock-contract digest. It records a sorted package DAG, local relative origins,
dependency content hashes, canonical manifest hashes, and each module's exact
source hash, interface hash, and sorted exports. It also records full current
digests for the language, source, module-interface, manifest, and lock
contracts.

Identity encoding is length-framed and domain separated:

- source identity hashes exact bytes;
- module identity hashes the module-interface contract, module path, source
  hash, and sorted exports;
- package identity hashes the package-manifest contract, canonical manifest,
  and sorted module identities.

`lkjscript package lock` writes one canonical lock atomically.
`lkjscript package check`, `run`, and `disasm` rebuild the graph and reject a
missing, noncanonical, stale, or mismatched lock. Creation order and working
directory do not affect bytes.

## Pipeline rule

Module qualification occurs before HIR construction. HIR, verified SSA,
bytecode, VM, JITs, diagnostics, transactions, and metrics consume that one
resolved program; no backend or agent endpoint repeats source namespace logic.
Semantic Source rename transactions update declarations, exports, exact import
lists, and scoped references atomically.

## Rejected alternatives

- program-global declaration lookup;
- wildcard or implicit prelude imports;
- transitive re-export by accident;
- nearest `src/std` discovery or `LKJSCRIPT_ROOT` fallback;
- source-declared module aliases;
- lock prefixes or generation numbers;
- package registries, network fetches, and mutable external resolution.

Historical package-import experiments are retained under `docs/history/` and
have no Current acceptance role.
