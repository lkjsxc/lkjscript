# lkjscript

Read docs/status.md first, then inspect Git and running jobs.
lkjscript is a language an agent can own completely and trust without a sandbox:
one small self-reproducing binary, a checked IR that admits native code by proof
instead of by confinement, and capabilities for every effect.

The checked lkjscript IR is the single trust boundary. Producers are untrusted.
Keep the IR checker independent of elaboration; keep the interpreter independent
of the checker and backend. Recheck optimizer output and discard it on rejection.
Native emission assumes only checked IR and the sealed runtime interface.
Views live in locals, parameters, results and view aggregates, never heap containers.
Name the ecosystem lkjscript, its representation lkjscript IR, and its files .lkjir.
Use ir/interpreter/backend in code, lkjscript for commands, and LKJSCRIPT_* variables.
IR text starts with lkjscript-ir 1. No compatibility aliases or obsolete paths.

No resource policy exists except representation facts.
Request memory when needed, adapt to refusal; run under limits that fit actual use.
Report located allocation/stack traps; never size from RAM or rlimits.
No default tool budgets, output caps, stack settings or address-space envelopes.
Use a generous test watchdog; mutants get a short nontermination allowance.
Keep one heap/one stack exhaustion test. Use at most eight workers on this shared host; clean up your own processes.

The surface has Rust shape; programs both compilers accept behave the same.
The optional Rust witness must not constrain lkjscript's design; no Clippy gate.
Apply the two-way rule: add ordinary forms using existing IR/library rules, or
rewrite a consumer when a rare form would enlarge trusted semantics.
English everywhere; no Python. All technical decisions are delegated.
Work on main inside /home/coder/workspace/lkjscript. Preserve .generation-one,
other projects, running services and their data. No GitHub releases or tags.
Never restore WebAssembly, a Node execution host, or backward compatibility.

Run bash tools/check.sh with LKJSCRIPT_GATE_JOBS=8 after coherent changes and before
pushing. Keep fixed points.
Use unique gateway operation IDs; recover uncertain operations before retrying.
Coordinate parallel work by file ownership. Commit coherent changes, recheck
upstream, push main directly, inspect CI; never force-push or rewrite shared history.
Keep docs/status.md within 60 lines, docs/direction.md within 150, this file within 40.
Record decisions in direction, current results/next action in status, contracts in
docs/ir.md and docs/language.md. Git holds history; do not create observation ledgers.
