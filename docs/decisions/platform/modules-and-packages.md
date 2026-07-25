# Modules And Reproducible Packages

## Purpose

Define the accepted replacement for the current program-global imported
namespace before self-hosting, framework, or database ecosystems depend on it.

## Status

Contained `.lkjscript` imports and exact executable source closure are
**Current**. Explicit modules, exports, manifests, lockfiles, package graphs,
and a registry are not implemented. The module/package model in this record is
an **Accepted Target**.

## Decision

The global imported declaration namespace will be replaced, without aliases or
compatibility lookup, by:

- explicit module identities;
- explicit exports and qualified imports;
- one package manifest format;
- exact lockfiles;
- content hashes over canonical package contents;
- reproducible dependency resolution;
- local path dependencies;
- semantic ABI and native ABI identities;
- declared native/host capabilities;
- bounded deterministic graph and conflict diagnostics.

A package graph is resolved before source name resolution. Two packages cannot
be conflated because source names happen to match. Every resolved function,
product, trait, implementation, generic instantiation, runtime capability, and
native artifact retains its package/module identity through HIR and SSA.

## Native Capabilities

Ordinary packages cannot call arbitrary addresses or declare raw native
pointers. Native integration is available only through audited capability
packages whose manifests record a versioned ABI, ownership/lifetime contract,
required platform capability, and safe lkjscript wrapper. Unsafe
implementation remains in the trusted native boundary.

## Resolution And Locking

Resolution is deterministic under explicit bounded work and depth. Lockfiles
record exact package identities, content hashes, dependency edges, selected
features if features are later accepted, semantic ABI, and capability grants.
A lock mismatch is a build error rather than a best-effort fallback.

## Sequence

1. module and export syntax decision;
2. package identity in HIR/SSA and diagnostics;
3. manifest and local-path graph;
4. exact lockfile and content hashing;
5. package-capability enforcement;
6. self-hosted package manager and build driver;
7. registry protocol only after local reproducibility is proven.

## Deferred

Registry publication, remote resolution, signatures/transparency logs, and
binary distribution are **Deferred**.

## Rejected

Ambient global lookup, implicit exports, unlocked network resolution, package
identity inferred only from a path string, arbitrary native FFI, and preserving
the old namespace as a fallback are **Rejected**.
