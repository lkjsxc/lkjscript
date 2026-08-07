# lkjscript language semantics

**Status: normative intended semantics.** This specification defines language meaning independently
of any physical source notation, compiler IR, bytecode encoding, or runtime engine. The current
implementation is summarized in [`../status.md`](../status.md); a difference there is an
implementation gap, not an implicit change to this document.

This document specifies complete executable programs. Editing-time incomplete states and semantic
transactions belong to the separate [workspace specification](workspace.md).

## 1. Programs and resolution

A program is a finite set of modules containing declarations. A declaration has semantic identity,
a kind, a name, and typed contents; file location, formatting, source order where semantically
irrelevant, and physical notation are presentation attributes.

Imports identify modules and declarations exactly. Resolution is static and deterministic. A
complete executable program must reject duplicate declarations, unresolved or ambiguous names,
invalid imports, and dependency cycles that cannot be assigned the required static meaning.

Functions have typed parameters and one result type. Calls are resolved before execution, including
any generic or trait obligations. An implementation may specialize a call, but failure to specialize
must use a generic correct path rather than make an otherwise valid program invalid.

## 2. Types and values

Every executable expression has a static type. Foundational semantic families include:

- unit, Boolean, signed 64-bit integer, and 64-bit IEEE-754 floating-point values;
- symbols, text, immutable bytes, mutable byte vectors, and borrowed byte views;
- lists;
- nominal products and nominal enums;
- capabilities and typed host resources; and
- function, generic, and trait-constrained values admitted by the static type system.

Products and enums are nominal: equal field shapes do not make separately declared types
interchangeable. Enum construction names a declared variant and supplies exactly its typed fields.
Pattern matching is type-correct, deterministic, and exhaustive. A statically useless or subsumed
arm is invalid. `never` is the result type of control that does not return normally.

Equality is type-directed and complete for values for which equality is defined. Integer operations
and conversions that are defined as checked must report arithmetic failure rather than wrap.
Floating-point operations preserve IEEE-754 behavior, including bit-significant cases where an
operation's contract requires it.

## 3. Bindings and control

Bindings are immutable unless mutation is explicitly part of their declaration. Evaluation order is
deterministic where effects, movement, cleanup, failure, or control flow can observe it.

The language supports conditional selection, ordered blocks, loops, `while`, `break`, `continue`,
early return, function calls, and exhaustive enum matching. Control must not read an uninitialized
value, use an unavailable moved value, or enter an invalid control-flow state.

## 4. Effects and capabilities

Host effects require explicit typed capabilities supplied by the package and host. Source code has
no ambient filesystem, network, database, terminal, process, clock, argument, or standard-I/O
authority. A capability authorizes only operations in its declared domain; host adapters must reject
kind or authority mismatches before performing the effect.

Given the same complete semantic program, explicit inputs, capabilities, target, and options, every
completed execution has the same language meaning. Runtime tier, scheduling, caching, or diagnostic
mode must not reinterpret the program.

## 5. Ownership and memory safety

Ordinary execution is collector-free and non-tracing. Ordinary programs do not expose raw pointers,
retain/release, a general `free`, named implementation lifetimes, or runtime-engine-specific memory
controls.

Copy values may be duplicated. Affine or unique values are moved exactly once unless a valid borrow
is active. A borrow cannot outlive its authority or conflict with movement or mutation. Cleanup is
deterministic where promised by the value or host-resource contract and is attempted on normal,
error, and cancellation paths without double release. Memory safety, cancellation safety, and
failure atomicity are semantic requirements even when internal storage strategies differ.

## 6. Validity, resources, and failure

Semantic validity is determined by type, effect, capability, ownership, exhaustiveness, resolution,
and control-flow laws. Program size, source bytes, token count, nesting, declarations, parameters,
arguments, locals, fields, variants, CFG nodes, IR nodes, runtime objects, or analysis work are not
semantic laws.

Trusted local compilation and execution continue until success, explicit cancellation, allocation
failure, operating-system or I/O failure, a genuine external representation failure, or another real
host failure. An untrusted host may apply explicit coarse input, memory, output, elapsed-time,
cancellation, or concurrency policy. Exhausting that policy is a typed host-resource result, not a
semantic error: the unchanged program may succeed under a higher or unrestricted policy.

No implementation may silently truncate source, diagnostics, semantic results, generated code,
serialization, output promised as complete, or execution. It must stream, paginate with a stable
continuation, return an explicit partial result, or fail.

Representation overflow must be checked before indexing, allocation, or publication. Compact
encodings and optional native shapes must have a generic wide fallback where they would otherwise
restrict an ordinary valid program.

## 7. Publication and external boundaries

Malformed packages, serialized artifacts, process messages, capability grants, paths, executable
relocations, FFI values, and persisted data fail closed at their trust boundary. Validation failure,
resource exhaustion, cancellation, backend failure, or I/O failure must not publish a partial
artifact or poison an earlier valid program, cache entry, or runtime state.
