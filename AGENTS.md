# lkjscript

Read docs/status.md first, then inspect Git and running jobs.
lkjscript is an AI-native general-purpose language: agents directly understand,
query and change a program's semantic structure at the scale of software such as
Chromium. One self-reproducing binary, checked IR and capabilities for every effect
make that foundation independently verifiable; applications are test specimens.

The checked lkjscript IR is the single trust boundary. Producers are untrusted.
Keep the IR checker independent of elaboration; keep the interpreter independent
of the checker and backend. Recheck optimizer output and discard it on rejection.
Native emission assumes only checked IR and the sealed runtime interface.
Views live in locals, parameters, results and view aggregates, never heap containers. Name the ecosystem lkjscript, its representation lkjscript IR, its source files .lkj and IR files .lkjir.
Use ir/interpreter/backend in code, lkjscript for commands, and LKJSCRIPT_* variables.
IR text starts with lkjscript-ir 1. No compatibility aliases or obsolete paths.
Compiler owns recovery: retain refused owners and poison uncertain bindings/coherence. Replay each selected glob slot, absent pattern/block slot and lexical ancestor. Synthetic probes read only their owner and keep authentic refusal causes; refuse import/local precedence beyond ordinary lowering. Partial facts require exact source/dependency replay and never authorize whole-source IR, execution or checked edits. Compiler owns provider frontiers: keep explicit external aliases distinct from macro_use and ordinary prelude names; compare exact checked identities, recovered owners and actual invocation diagnostics before claiming progress (measured: docs/real-programs.md#explicit-dom-provider-inputs).
No resource policy exists except representation facts.
Request memory when needed, adapt to refusal; run under limits that fit actual use.
Report located allocation/stack traps; never size from RAM or rlimits.
No default tool budgets, output caps, stack settings or address-space envelopes.
Use a generous test watchdog; mutants get a short nontermination allowance. Keep one heap/one stack exhaustion test. Use four workers per stream on this shared host; clean up your own processes.

Text remains the stored projection; storing structure instead needs the owner's decision.
The surface has Rust shape; programs both compilers accept behave the same.
The optional Rust witness must not constrain lkjscript's design; no Clippy gate.
Apply the two-way rule: add ordinary forms using existing IR/library rules, or
rewrite a consumer when a rare form would enlarge trusted semantics.
English everywhere; no Python. Other technical decisions are delegated.
Keep proof receipts, evidence files, and Perl out of this repository.
Work on main inside /home/coder/workspace/lkjscript. Preserve .generation-one,
other projects, running services and their data. No GitHub releases or tags by hand:
`.github/workflows/release.yml` publishes the gated compiler from main daily.
Never restore WebAssembly, a Node execution host, or backward compatibility.
Before the full gate, compare lookup changes with prior behavior and Rust where shared. Compiler owns call inference: complete unresolved source-call substitutions without overriding contextual coercions or inventing reborrow sites. Compiler owns primitive method lookup: rank receiver forms and keep inference, source providers and generic selections consistent. Cover name collisions, malformed qualifiers and namespace precedence. Compiler owns token identity: preserve lexemes through matching and replay, normalize only semantic names, and compare raw/keyword transcription with Rust before narrowing recovery. Compiler owns transparent macro fragments: share repetition/capture classification and verify unchanged token provenance and ordinary lifetime checks. Compiler owns pattern-flow coverage: exhaustively name AST variants, retain binding reference terms, and test capture arena identities independently of runtime values. Module-path strings follow expansion destinations; test generated modules and preserve checked-rename meaning refusals. Compiler owns erased borrowed-call contracts: validate environment loans, transitive effects and descriptor metadata independently, and keep generated helper identities stable under checked edits.
Run bash tools/check.sh with LKJSCRIPT_GATE_JOBS=4 after coherent changes and before pushing. Keep fixed points. Compiler owns driver-root registration: check native main/admission parity before bootstrap and bridge/wrapper parity only in optional witness preflight. Compiler owns gate wrappers: close inherited lock and timer descriptors in children before descriptor-limit controls. Compiler owns source configuration: bind explicit values through replay, caches, units, workers and edits; verify both modes. Update generated callers and independent golden vectors when signatures or fingerprint schemas change.
Prefix compiler gateway IDs with compiler-; recover uncertain operations before retrying.
Compiler owns main; programs owns its worktree and status Programs section. Recheck
upstream, push main directly, inspect CI; never force-push or rewrite shared history.
Keep docs/status.md within 60 lines, docs/direction.md within 150, this file within 40.
Record decisions in direction, current results and next actions in status, and contracts in docs/ir.md and docs/language.md. Record every optimization approach in docs/optimization-ledger.md. Git holds other history; do not create other observation ledgers.
