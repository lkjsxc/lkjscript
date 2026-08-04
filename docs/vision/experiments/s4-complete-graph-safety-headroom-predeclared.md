# S4 Complete Graph Safety Headroom

## Status

**Experimental, predeclared before measuring the replacement maxima.** S3 rejected 16,384 graph nodes after observing
9,492 canonical nodes, which violated its below-50% headroom rule.

## Question

Can the current complete in-memory graph publish within 32,768 nodes and 65,536 edges while remaining below half of
each maximum, deterministic, endpoint-closed, and faster than the retained 2.35-second acceptance ceiling?

## Baseline And Candidate

- Implementation baseline: `006693f4cb8d5d7fb15f4f7fe54688df89c2a382` plus the mission worktree.
- Platform revision before promotion: 19.
- Rejected S3 candidate: 16,384 nodes; observed 9,492; no publication.
- Replacement implementation safety maxima: 32,768 nodes and 65,536 edges.
- Existing work and retained-byte maxima remain 1,048,576 work units and 64 MiB.
- Query work and output bytes remain separate view budgets.

The replacement maxima are not preallocation sizes, source semantics, or successful-output caps. Construction must
return typed exhaustion and publish nothing at plus one.

## Method And Metrics

Build the graph twice from the same clean declared input. Record canonical node and edge counts, charged work and bytes,
identity, serialized bytes, wall time, endpoint closure, and byte equality. Then run strong Current State and Agent
Handoff contexts and record structured completion, frontier, and output bytes.

## Acceptance And Falsification

Accept only if:

- nodes are below 16,384 and edges are below 32,768;
- every edge endpoint is retained;
- repeated canonical JSON is byte-identical with equal identity;
- identity changes when accepted canonical input changes;
- exact node, edge, work, and byte plus-one fixtures return typed failure without output;
- build wall time remains at most 2.35 seconds on the retained baseline environment; and
- bounded queries expose exact structured completion rather than graph truncation.

Reject on any partial publication, collision, dangling endpoint, nondeterminism, threshold breach, or unclassified
resource failure. A rejected result grants no authority to increase another number without a new record.

## Result

**Selected for the immediate correctness cut.** Two repeated builds each produced 9,492 nodes, 17,356 edges,
6,296 charged work units, 8,939,109 charged bytes, and graph identity
`d3232d851db1cf68ee77457de3ffb26c38047f6758242ace4ff3ce430df8c308` before the contract/public-fact cut.
The 10,075,234-byte canonical JSON outputs had equal SHA-256
`8c52f082d8485545f7c07ee5a421f942ce9252779ea91e067cf3b39c62db5cb5`; endpoint validation found zero dangling
edges. Warm command wall times were 1.642 and 1.656 seconds. Counts are below half of both selected maxima and time is
below 2.35 seconds.

The integrated contract, public-fact, capsule, and marker input was measured separately at staged tree
`f5cc75e3806125e0223c0be2d92d52f787cc19f7` over HEAD `0b0611b9b0c6e1819f26ac8a6a1e58dc4859c053`, before
this evidence paragraph changed the tree. Two builds each produced 9,541 nodes, 17,470 edges, 6,367 charged work units,
6,494,181 retained node/edge field bytes, and identity
`d61eb80bf86a23cb1b6aa6c3b9558c209ec890271bf8398f7be71738a33526a6`.
The 10,132,466-byte outputs were byte-identical at SHA-256
`5aae7022fb43af97d40cac317d93fcd2572f9e5009648fea5bf220e63cc01000`; warm wall times were 1.620 and 1.574 seconds.
The complete in-memory vector build is Current only as the immediate form, not the final incremental or sharded
large-repository design.

## Not Yet Measured

Peak RSS and query frontier sizes remain unavailable. Incremental clean-build equivalence is Deferred.
