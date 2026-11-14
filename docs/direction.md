# Direction — 2026-11-14

lkjscript is a language an agent can own completely and trust without a sandbox:
one small self-reproducing binary, a checked IR that admits native code by proof
instead of by confinement, and capabilities for every effect.

## Semantic boundary

The lkjscript IR is the identity; the Rust-shaped surface is one producer.
The frontend is untrusted. The checker independently verifies monomorphic,
structured IR: ownership, loans, initialization, types and reachable authority.
Nothing below the IR knows source-language concepts. Native emission assumes
only checked IR and the sealed runtime interface. Keep encoding separate.
The interpreter shares only the IR data module with checker/backend; test imports.
Attack the checker through direct IR mutation and independent interpretation.
Optimization is untrusted IR-to-IR work: recheck it, discard rejected output,
compare optimized native behavior with interpretation of the original IR.
There is no unsafe surface, FFI, ambient authority or unwinding.
Views may occupy locals, parameters, results and view aggregates, not heap containers.
Apply the two-way rule: add everyday forms through existing IR/library contracts;
rewrite consumers when a rare form would enlarge trusted semantics.

## Direct native execution (6.1, complete)

Keep the direct x86-64 backend plain:
- Materialize locations only on cold failure paths; preserve exact trap lines.
  Allocation/stack failures may use a static function location.
- Allocate registers to scalar locals/temporaries; keep aggregates and
  address-taken locals in frames. Forward straight-line values. No inlining,
  cross-block propagation or peephole collection; those are later IR passes.
- Emit nothing for scalar drops, unit values and copies onto themselves.
- The source allocator carves slabs and medium blocks from geometrically grown
  regions, reuses size classes, and uses mremap for dedicated large allocations.
- Reserve the stack above the ordinary mapping arena; halve failed requests.
  Return unused reservation on heap refusal and regrow stack pages when needed.
  Startup-only retries failed a 32 MiB actual-use probe; pressure must be shared
  by heap and live frames without moving stack addresses.
- Unary, binary and intrinsic operations are enums with stable text spellings.

Aggregates and vector elements are inline; references are machine addresses,
with address/count pairs for slice/text views. Frames use the machine stack.
Retain bounds, arithmetic, division, UTF-8, tag and stack-exhaustion checks.
Calls/branches use rel32; reject an unencodable program with a located error.
Preserve compiler reproduction, witness byte identity, hello under a 4 GiB
address-space limit and measured aggregate/scalar/buffer/compiler behavior.
If the compiler gap exceeds tenfold, identify another structural cause first.

## One executable and bootstrap (6.2)

One static executable named lkjscript contains frontend, checker, backend,
interpreter, CLI, source library and runtime. Stages exchange IR in memory,
checked once. Text IR remains for ir output, .lkjir input, tests and fuzzing.
include_str!/include_bytes! read beneath the source Directory capability;
embedded sources are ordinary source literals, with no generated source files.
World derives processes with explicit argument bytes and streams, a monotonic
clock, and directory listing/create/rename/remove. Owned Files carry child
streams and executable authority. Interpreter values use owned cells and indices.
An artifact returns eight raw scalar bytes on a driver-held descriptor; traps
return diagnostic text there, preserving the program's stderr bytes separately.
Standalone executables report traps on stderr. There is no reader or packet layer.
Interpreter self-execution materializes a checked native artifact only on demand;
unchecked mutation input cannot request native emission.
Raw bit conversions expose NaN payloads: negation preserves them and flips the
sign to match Rust; binary arithmetic retains canonical NaNs.

All user bodies are checked. Library headers/layouts remain global; library body
verification and lowering follow demand, retaining indexes across batches.
check-library checks every library body during bootstrap and toolchain tests.
Store a previous compressed executable in seed/. Compiler sources must build
with it. Refresh rarely, before using a new form in compiler sources.
sh tools/bootstrap.sh unpacks it, builds target/lkjscript, checks the complete
library and compares a second build byte for byte, without Rust or Node.
The gate also compares the Rust witness's output with the self-built binary.
The component launcher, installer, rebuild scripts, manifests, result reader,
packet/bundle formats and their shell suites are deleted in this milestone.
Do not create releases or tags: the generation-one release line serves live apps.

## Gate and optional witness (6.3)

Clippy is removed now; retain cargo fmt --check only until the witness moves.
Cases are single files with expectation headers: value, output bytes, diagnostic,
trap code and line. Replace cases.json and repetitive integer files with generated
cases from a small table. lkjscript test discovers cases and executes both routes
in parallel processes, printing one ok line per suite and details only on failure.
Port pure seeded generators, direct IR mutation and reduction to lkjscript.
Native/interpreter comparisons stay in the gate; rustc comparison moves to witness.
Run a fixed mutation slice every gate and a long campaign after each milestone;
turn every finding into a permanent case. Extract executable programs in
language/IR references and test them. Convert language/IR Cargo tests to cases;
drop Rust-host tests.

Move Cargo.toml, bridge and rustc shim under witness/; the root stops being Cargo.
CI has a gate using only the stored seed, without cargo/rustc/node, and a
nonblocking witness job. The witness rebuilds identical compiler bytes and runs
marked comparable cases. Inspect failures, but never let Rust constrain design.
Programs both compilers accept behave the same; lkjscript may reject what Rust
accepts and accept what Rust rejects. Keep .rs while the witness lives; choose a
native extension when deleting it. Delete the witness when it blocks needed
source forms or stops finding useful differences.

## Real consumers (6.4)

A persistent HTTP server serves docs/ beneath a Directory capability, using a
Listener and time. Test generated requests for an hour on a fresh local port
with flat memory; never deploy it or touch a running service.
Capability-typed entries declare authority, for example main(Stdout, Directory,
Listener). The CLI grants each explicitly and refuses missing grants before
execution. World remains all authority. Report reachable authority from checked
IR alone, including a .lkjir file. The compiler alone is not the only consumer.

## Resources and measurement

No limit exists except facts of representation. Ask the kernel for memory when
needed; do not read RAM or rlimits to plan extents. Adapt to refusal. A program
must start and run under any external limit that leaves room for actual use.
Report located allocation/stack traps when requests cannot be satisfied.
Tools have no default memory envelope, output cap or stack override.
The runner has a generous watchdog for programs expected to finish; generated or
mutated programs that may loop have a separate short allowance and a
"did not terminate" outcome. Neither is a language rule.
Keep one heap and one stack exhaustion test; do not compare their route locations.
Recursion is fine; replace a traversal only when a real program fails.
Use at most eight workers; leave no background jobs and remove temporary files.
Measure phases/RSS/IR/code sizes with --stats and bench/. Report times in status
and commits. Measurements do not gate. Find structural costs before tuning;
measure any explanation for a remaining gap.

## Horizon and working policy

After consumers: rechecked inlining measured on bench/, verified range facts,
content-addressed IR-closure/code/test reuse, checked-IR queries and diagnostics
with machine-applicable edits, a formatter, an eventual checked-IR seed, a second
target and structured concurrency. Do not start these early.

Work on main inside /home/coder/workspace/lkjscript; preserve .generation-one,
other projects, services and data. No Python, WebAssembly or Node execution host.
No backward compatibility. Delete replaced components in the same change.
No observation files, hash ledgers or manifests: the gate checks properties.
Keep references concise: form, meaning, lowering, rejection and rewrite.
Run the eight-worker gate, keep fixed points, commit coherent changes, recheck
upstream, push main directly and inspect CI. Never force-push/rewrite history.
Use unique gateway operation IDs and recover uncertain operations before retrying.
Keep status within 60 lines and this direction within 150. Git is the history.
A reversal of the semantic boundary, course or resource stance needs a reason here.
