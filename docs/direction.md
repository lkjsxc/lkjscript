# Direction — 2026-11-14

lkjscript is a language an agent can own completely and trust without a sandbox:
one small self-reproducing binary, a checked IR that admits native code by proof
instead of by confinement, and capabilities for every effect.

## Semantic boundary

The lkjscript IR is the identity; the Rust-shaped surface is one producer.
The checker independently verifies monomorphic, structured IR: ownership, loans,
initialization, types and reachable authority. Nothing below it depends on source
language concepts. Document the contract in docs/ir.md as reference entries.
Keep the independent interpreter and direct IR mutation testing.
Optimization is untrusted IR-to-IR work: recheck it, discard rejected output,
compare optimized native behavior with interpretation of the original IR.
There is no unsafe surface, FFI, ambient authority or unwinding.
Views may occupy locals, parameters, results and view aggregates, not heap containers.
Apply the two-way rule: add everyday forms through existing IR/library contracts;
rewrite consumers when a rare form would enlarge trusted rules.

## Direct native execution

Replace the operand-stack lowering and linear-memory emulator with x86-64
instruction selection directly from checked IR. Keep encoding separate.
Scalars use registers and frame slots. Structs, enums, tuples and arrays are
inline, including in parent aggregates and vector elements. An enum has a tag
and its largest payload. Only Box, Vec and String payloads use heap blocks.
References are machine addresses; slice/text references are address/count pairs.
Use the machine stack, never heap frames. Retain language-defined bounds,
arithmetic, division, UTF-8, tag and stack-exhaustion checks.
Calls and branches use rel32; an unencodable program receives a located error.
The allocator is lkjscript source over a sealed raw kernel-memory interface.
Keep this trusted backend plain; larger optimizations belong in rechecked IR.
Build beside the old backend, compare native routes and interpreter, switch,
re-establish the fixed points, then delete the old lowering/encoder in this stretch.
Do not tune compiler-source bookkeeping before removing structural backend costs.

## One executable and bootstrap

One static executable named lkjscript contains frontend, checker, backend,
interpreter, CLI, source library and runtime. Stages exchange IR in memory,
checked once. Text IR remains for ir output, .lkjir input, testing and fuzzing.
Delete the multi-process launcher, installer/rebuild scripts, manifest, hash ledger,
result reader and packet protocol when this executable replaces them.
Keep source bundles only while an actual tool consumes them.
A self-check rebuilds the compiler and compares the complete executable bytes.

Store a previous compressed executable in seed/. Compiler sources must build
with that seed. Refresh it only when needed, before using a newly supported form
inside the compiler. While the witness lives it must regenerate the seed bytes.
A fresh clone without Rust or Node must build and pass the gate from that seed.
Do not create releases or tags: the generation-one release line serves live apps.
Keep an eventual checked-IR seed possible; do not implement it now.

Rust becomes an optional witness under witness/: Cargo manifest, bridge and shim.
The root stops being a Cargo project. Keep compiler sources valid Rust while
cheap, so the witness can regenerate identical output and compare programs.
Programs both compilers accept must behave the same; lkjscript may accept
programs Rust rejects. Keep .rs while the witness lives. Delete the witness
when it blocks needed source forms or ceases to find useful differences.

## Consumers

1. Repository tools: test runner, differential generators, IR mutator/reducer,
   benchmark reporter. Derive process, monotonic clock, directory listing,
   create/rename/remove capabilities from World, following the two-way rule.
2. A persistent HTTP server serves docs/ beneath a Directory capability.
   Add Listener and time; test a fresh local port, never deploy or touch a service.
   Exercise reuse/fragmentation for an hour with generated requests.
3. Capability-typed entries declare authority, for example main(Stdout, Directory,
   Listener); the CLI grants each explicitly and refuses missing grants before
   execution. World remains all authority. Report reachable authority from IR alone.
Concurrency and a second target wait for a consumer.

## Resources and measurement

No resource policy exists except facts of representation.
Ask the kernel for memory when needed; do not size extents from RAM or rlimits.
Use a lazily reserved stack with a guard and located exhaustion traps.
Tools have no default memory envelope, output cap or stack override. The runner
has one generous hang watchdog. Keep one heap and one stack exhaustion test;
do not compare exhaustion locations across different representations.
Recursion is fine; replace it only when a real program fails.
Use at most eight workers, leave no background jobs, clean temporary directories.
Add --stats for phase time, peak RSS, IR/code sizes.
The Rust witness uses standard-library collections; the self-built compiler uses
lkjscript's indexed AVL map. Keep the twin as a yardstick, but its full timing
ratio includes library implementation differences (verified in code on 2026-11-14).
Keep small bench/ programs and a table against the Rust-built twin and prior run.
Investigate structural costs before source tuning.

## Milestones, in order

0. Rename and reset: ir.rs, ir_text.rs, interpreter.rs, backend*.rs;
   IrProgram/IrFunction, source_to_ir, .lkjir, lkjscript ir/runtime-ir,
   --unchecked-ir, --engine interpreter, E_IR/E_IR_TEXT, LKJSCRIPT_*.
   Text begins lkjscript-ir 1. Rename files, tests, tools and local IR variables;
   remove obsolete names without aliases. Delete docs/observations/ and limit
   disclaimers. Finish with the gate green and both compiler fixed points.
1. Direct backend: corpus/generators/mutations agree with the interpreter,
   old lowering and encoder deleted; self-built fixed point matches witness bytes.
   Record aggregate-call, scalar-loop, vector-read, hello-build and complete
   rebuild timings, peak RSS and executable sizes beside the witness.
2. One binary and stored seed: check/build/run/test/ir and interpreter execution,
   process/clock/directory operations, owned-values/index-based interpreter port.
   One documented bootstrap command works without Rust/Node and reproduces itself.
   Delete the three-process pipeline and its scaffolding.
3. Gate without Rust/Node: each case carries its expectation in a header;
   generate repetitive integer cases from a small table. Port pure seeded
   generators, mutator and reducer; run both routes in parallel subprocesses.
   Print one ok line per suite and failure details. Convert language/IR Cargo
   tests into data, drop host tests. Move witness files; CI has a bare seed gate
   and a nonblocking Rust comparison job. Inspect witness failures.
4. Consumers: one-hour HTTP request test with flat memory, capability entries,
   missing-grant refusal and reachable-authority reporting from .lkjir.
5. Later: rechecked inlining, verified range facts, content-addressed reuse,
   IR queries and machine-applicable edits, IR seed, second target, concurrency.
   Do not begin these before the earlier consumers.

## Deletion and working policy

Delete census machinery if the completed ports were its only consumers.
Delete old budget variables, reservation/arena/stack-host scaffolding,
host-boundary tests, and launcher tests as their replacements land.
Delete trusted command/trust tool; one paragraph in docs/ir.md names the trust base.
Replace native/frontend/limits documents with help and short reference sections.
Delete Node tools after their replacements pass. Clean unused bulk only after
checking jobs. Never touch .generation-one, other projects, services or their data.
Work on main; run the gate, commit coherent changes, fetch/recheck upstream,
push directly and inspect CI. No force pushes, history rewrites, releases or tags.
Use gateway operation IDs for writes and recover unknown results before retrying.
Keep concise references and status; Git is the history, tests enforce properties.
Any reversal of the decisions above needs a concrete reason recorded here.
