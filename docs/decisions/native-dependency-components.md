# Native dependency-component analysis

Selected 2026-10-08. Implementation and acceptance remain status-owned; this
selection alone does not establish a working pass or compiler integration.

## Decision

Extend the native provisional dependency-graph witness from reachability to a
complete strongly connected component partition. Author the algorithm through
ordinary lkjscript declarations, using explicit owned worklist implementations.
Do not add a privileged graph writer, a new intrinsic or a Rust semantic generator.
The first production-compiler substitution remains a separate authority boundary.

Use two iterative graph traversals: finish order on the original graph, followed
by traversal of reversed edges in decreasing finish order. Explicit owned frames
replace recursion on graph depth. Mark first-pass vertices on entry, not while
scheduling all siblings; otherwise an edge between pending siblings can invalidate
finish order. Reuse the emptied worklist between trees and components.

Complete existing proposal validation precedes either pass, including unreachable
nodes, duplicate identities and missing references. Keep its capacity, structural
invalidity and runtime failures distinct. All graph identities remain signed I64;
frame tags use bounded node positions, never negation or arithmetic on identities.

Return every component, not merely the reachable subgraph. Order components by
the first authored member and members by authored node order. Distinguish a
singleton self-loop from an acyclic singleton. Retain the existing reachability
partition alongside the components. In either maintained LIFO storage
implementation, valid DFS traversal order must not affect the complete observable
result. The Worklist signature alone does not prove stack laws for an arbitrary
implementation; keep that behavioral precondition separate from type and ownership
admission.

## Acceptance obligations

Compare flat and chunked worklists with independently computed complete outputs.
Include empty graphs, disconnected cycles, self-loops, repeated edges/roots,
signed extremes, deep chains, cross-edges between pending siblings and graphs with
multiple possible traversal orders. Invalid unreachable edges must still reject as
ordinary invalid-proposal outcomes before traversal. Capacity precedence remains
nodes, roots, edges.

Keep native literal inputs and require public plan/apply/check/build, unchanged
canonical drafts, detached execution after source removal and joined owned cleanup.
A test-only host oracle may compute expected partitions but must not compute any
program output. Preserve failures, workload inputs and executable/source identity.

The algorithm makes a bounded number of graph visits; this does not establish
linear wall time or allocation. Ordinary persistent lists/maps, preparation and
runtime bookkeeping have separate costs. Do not claim zero-copy behavior, a speed
improvement, complete self-hosting or production compiler adoption from this pass.

## Why this increment

The current language already has enough ownership and generic structure to express
a nontrivial compiler-analysis workload. Exercise that composition before adding
associated view families solely from a syntax inventory. Retained working-set and
preparation evidence can then justify borrowed adjacency views, representation-
dependent cursors or region custody. Kernel method-map construction restrictions
remain unchanged; this native witness is not a claim that they were relaxed.
