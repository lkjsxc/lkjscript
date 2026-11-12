# lkjscript, generation two

Read `docs/status.md`, inspect Git and live jobs, then implement the next dependency-driven
result. Use corpus/census evidence; do not resume a file-by-file source campaign. Language quality comes first, reliable agent development second. Own the
compiler, runtime, library and tools, eventually without Rust or Node. Applications
are probes; historical architecture and backward compatibility are not goals.

Rust-compatible source, explicit checked meaning and memory64 WebAssembly are
current strategies. Reconsider them when measured obstacles justify an executable
transition. Preserve the current runnable path while replacing its parts. The
first self-compilation target is the complete compiler core with declared source,
library and host-buffer inputs, not a syntax-only file count or a second toy core.

Use one authoritative contract at each semantic boundary. Resolve declarations
once in their lexical modules; check generic definitions under their declared
constraints; verify explicit concrete K before emission. Dynamic bounds, overflow
and artifact validation remain necessary. Unsupported syntax gets a diagnostic.
Keep the trusted implementation and outstanding obligations explicit.

No arbitrary acceptance ceilings, work accounting or policy plumbing. Choose
growing representations and suitable algorithms; recursion and explicit work stacks
are both valid when the supported depth remains usable; keep checked arithmetic and ownership
sound. Record actual representation and engine constraints in `docs/limits.md`.
Measure the stages that actually ran, including engine loading and cleanup.

Work in this checkout only. Preserve the read-only `.generation-one` worktree for
specific reference. Do not modify deployments or neighboring checkouts. Use
English, no Python, ordinary compiler words, and focused command output. Coordinate
parallel edits by file ownership. Use unique MCP operation IDs and recover unknown
operations before repeating mutations.

Run `bash tools/check.sh` after coherent changes and before publishing them. A
sub-minute routine gate is a performance objective, never a coverage or language
acceptance limit. Keep expensive scale/fixed-point probes separate when useful.
Commit small, recheck upstream, push main directly and inspect CI. Do not force
push, rewrite shared history, publish releases, or change credentials/protections.
All ordinary technical decisions are delegated; continue without routine approvals.

`docs/language.md` specifies implemented semantics. `docs/status.md` records current
behavior, remaining dependencies, evidence and the next action. `docs/direction.md`
records the selected strategy. Git preserves history; avoid session diaries,
duplicate specifications and permanent superseded production implementations.

Choose width by trust cost. Surface forms lower to existing K where possible;
validate sugars against independently generated expansions as well as rustc.
Treat the existing stack bridge as finished. Separate checked-K lowering, layout,
explicit glue and source runtime before adding an owned native encoder; do not
assemble another runtime by hand. Prefer printing conformance programs and direct
K fixtures over large Rust harnesses. Keep seeds and reduced failures in Git,
commands beside measurements in status, and detailed run output under target/.
