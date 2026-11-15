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

Keep direct x86-64 instruction selection plain. Materialize source locations on
cold failure paths; allocation/stack failures may use a static function location.
Allocate scalar locals/temporaries to registers; aggregates and address-taken
locals occupy frames. Forward straight-line values. No register-allocation
project, cross-block propagation or peephole collection in this milestone.
Emit nothing for scalar drops, unit values or copies onto themselves.
Aggregates/vector elements are inline; views are addresses or address/count pairs.
Frames use the machine stack. Retain bounds, arithmetic, division, UTF-8, tag
and stack checks. Reject unencodable rel32 calls/branches with a located error.
The allocator grows regions geometrically, reuses size classes and mremaps
dedicated large blocks. Reserve the stack above the mapping arena; halve refused
requests, return unused reservation on heap refusal and regrow when needed.
Stack/heap pressure shares the address space without moving live stack addresses.
Operations are enums with stable IR spellings. Preserve byte reproduction and
hello under a 4 GiB address-space limit.

## Profile first, then remove structural costs

build --profile embeds a function-address map and opts into OUTPUT.samples.PID.
A Linux CPU timer samples instruction pointers and real frame chains without a
depth cap. The shared samples file survives exit/signals; profile reports self
and inclusive source function counts in at most 40 lines, without DWARF or perf.
Instrumentation file/timer authority is granted by this explicit build option.
The 43cc8ae9 profile found type-sized boolean initialization and cloning first,
then called aggregate copies. Use sparse query visits and packed snapshot facts;
emit known-size overlap-safe copies inline. This follows the profile rather
than assuming aggregate copying alone explains the gap. Stop after these two
causes or about 3x the witness. Trap stubs and a new register allocator can wait.
Put self-build first in bench/, with 64-byte call chains and Vec construction/walks.

## One executable and bootstrap (6.2, complete)

One static executable contains frontend, checker, backend, independent interpreter,
CLI and embedded source library/runtime. Stages exchange checked IR in memory;
text IR remains for output, .lkjir inputs, tests and fuzzing.
include_str!/include_bytes! read beneath the source Directory capability.
World derives process, clock and directory operations; process arguments and
streams are explicit. Owned Files carry streams and executable authority.
Interpreter values use owned cells/indices; self-execution emits checked native
code only on demand. Unchecked mutation input cannot request native emission.
A private descriptor returns eight scalar bytes or trap diagnostics, preserving
program stderr. Standalone traps use stderr. There is no packet/reader layer.
Raw negation preserves NaN payloads and flips the sign; binary arithmetic
canonicalizes NaNs. All user bodies are checked; library bodies follow demand
with reusable indexes. check-library verifies every body during bootstrap.
Store a previous compressed executable in seed/; refresh after emitter changes or
before adopting new source forms, checking two native generations and the witness. sh tools/bootstrap.sh builds target/lkjscript, checks
the library and verifies a second build byte for byte, without Rust or Node.
The optional Rust witness must emit identical compiler bytes.
The component launcher, installers, source bundles, manifests and reader are gone.
No releases/tags: generation-one releases still serve live applications.

## Gate and optional witness (6.3, complete)

Cases are single files with expectation headers: value, output bytes, diagnostic,
trap code and line. Headers replace cases.json; repeated integer cases come from a
small table. lkjscript test discovers and runs both routes in native
worker processes, one ok line per suite and details only on failure.
Seeded generators, direct IR mutation and disagreement reduction are lkjscript programs.
Keep native/interpreter comparisons in the gate; rustc comparisons are optional.
Run a fixed mutation slice every gate and a long campaign after each milestone;
report count and mutants/minute, turning every finding into a permanent case.
Extract reference programs into cases and remove prose those cases already state.
Keep the language reference below 500 lines: form, meaning, lowering, rejection,
rewrite. Port language/IR Cargo assertions; drop Rust-host tests.
Retain the three scoped-concurrency cases and borrowed_closure_escape as intended
pending specifications, each naming the missing implementation in one line.
Holder liveness is a conservative rule, with a rejection and lexical-scope rewrite;
it is not a pending promise to implement automatic lifetime shortening.
Cargo, bridge and rustc shim live in witness/; the root is not a Cargo project.
CI gates using only the stored seed, without cargo/rustc/node, with a nonblocking
witness job. Inspect witness failures; never let Rust constrain language design.
Programs accepted by both compilers behave the same. Keep .rs while the witness
lives; choose a native extension when deleting it. Delete the witness when it
blocks necessary source forms or stops finding useful differences.

## Real consumers (programs stream)

Programs work in /home/coder/workspace/lkjscript-programs-20261115 on programs;
land through fetch/rebase, bootstrap, four-worker gate and push HEAD:main.
The HTTP consumer grants Directory, Listener, Clock and output explicitly.
Connector grants one numeric IPv4:port; connect yields an owned File stream.
No DNS or substitute destination follows from that grant. Both executors validate
entry descriptors; authority follows checked-IR effects and owning cleanup.
HTTP/1.1 persistence and pipelining retain exact framing. Functional and load
clients are lkjscript; measure verified requests on persistent connections.
Profile before removing at most two costs. Fixed-size owned byte chunks remove
per-byte file assembly without adding an intrinsic or caching changing files.
Keep compiler source within the stored seed's forms; never refresh seed here.
The interpreter's independent syscall helper preserves ordinary File transport;
normal completion drains and reaps it, abnormal cleanup kills and reaps it.
Next prove readiness/deadlines, crash-durable journal state, stop and sessions.
A documentation candidate may come earlier; never modify the live unit or /opt.
The compiler is one consumer among ordinary programs; no deployment here.

## Resources and measurement

No resource policy exists except representation facts. Ask the kernel when
needed; never read RAM or rlimits to plan extents. Adapt to refusal and run under
any external limit leaving room for actual use. Report located allocation/stack
traps when requests cannot be satisfied. No default tool memory envelopes,
output caps or stack overrides. Use a generous normal-program watchdog and a
separate short allowance for generated/mutated nontermination, not a language rule.
Keep one heap and one stack exhaustion test; do not compare their route locations.
Recursion is fine until a real program fails. Use at most eight workers and leave
no background jobs or temporary files. Report --stats/bench timings, RSS, IR/code
sizes in status and commits. Measurements do not gate. Measure remaining gaps.

## Horizon and working policy

After consumers: content-addressed IR-closure/code/test reuse before inlining;
then rechecked inlining measured on bench/, verified range facts, checked-IR
queries and diagnostics with machine-applicable edits, formatting, an eventual
checked-IR seed, a second target and structured concurrency. Do not start early.
Work on main inside /home/coder/workspace/lkjscript; preserve .generation-one,
other projects, services and data. No Python, WebAssembly or Node execution host.
No backward compatibility. Delete replaced components in the same change.
No observation files, hash ledgers or manifests: the gate checks properties.
Run the eight-worker gate, keep fixed points, commit coherent changes, recheck
upstream, push main directly and inspect CI. Never force-push or rewrite history.
Use unique gateway IDs; recover uncertain operations before retrying.
Keep status within 60 lines, direction within 150. Git holds history.
Record a reason here for any reversal of the semantic boundary, course or resources.
