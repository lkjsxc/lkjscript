# Directory Fan-out

## Context

Deep trees of small files help weak AI authors; flat directories grow opaque.

## Decision

Cap visible children per directory at a tunable `max_dir_children` (default 8),
enforced by `check-tree`. Hidden entries, `target/`, and root `LICENSE` do not
count.

## Consequences

Package layout nests under `meta/`, `examples/*/`, `src/std/`, and `src/lib/`.

## Rejected

Unlimited directory width; counting hidden build artifacts.
