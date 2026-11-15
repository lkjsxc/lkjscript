# Direction — 2026-11-15

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

build --profile embeds function and operation-kind maps and opts into OUTPUT.samples.PID.
A Linux CPU timer samples instruction pointers and real frame chains without a depth cap.
Reports survive exit/signals and show self/inclusive functions and exclusive kinds
in at most 37 lines, without DWARF or perf; the explicit option grants instrumentation.
Measure self-built / witness opt=0 / witness -O and emitted kinds; stop near 3x -O
or after two measured causes are gone. Keep compiler, call/vector, HTTP and map rows in bench/.
Scalar Clone lowers to Read. --inline-limit N copies small acyclic bodies and forwards dead temporaries.
Recheck changed bodies; exact original bodies under unchanged shared graphs retain their proof.
Keep the original interpreter/authority oracle and copied source locations; reject the whole bad candidate.
Default zero: measured cold self-build admission costs exceed the generated-code gains; inlining stays explicit.

## One executable and bootstrap

One static executable contains frontend, checker, backend, independent interpreter,
CLI and embedded source library/runtime. Stages exchange checked IR in memory;
text IR remains for output, .lkjir inputs, tests and mutation.
include_str!/include_bytes! read beneath the source Directory capability.
World derives process, clock and directory operations; arguments/streams are explicit.
Owned Files carry streams/executable authority. Interpreter values use cells/indices;
self-execution may emit checked native code, never unchecked mutation input.
The private result descriptor preserves stderr; standalone traps report on stderr.
Raw negation preserves NaN payload/sign; binary arithmetic canonicalizes NaNs.
Check all user bodies and demanded library bodies; check-library verifies every body.
The stored compressed seed builds two native generations byte for byte without Rust
or Node; optional Rust witness output must agree. Refresh after emitter/source changes.

## Gate and optional witness

Case files carry value/output/diagnostic/trap expectations. The native test driver
runs native and independent interpretation in processes, with details only on failure.
Seeded generators, direct IR mutation and disagreement reduction are lkjscript programs.
Run a fixed mutation slice every gate and a long campaign after each milestone;
report count/throughput and preserve every finding as a permanent case.
Reference examples are executable cases; language.md stays below 500 lines with
form, meaning, lowering, rejection and rewrite. Port language/IR contracts; drop host tests.
Holder liveness is conservative; its rejection names the lexical-scope rewrite.
borrowed_closure_escape remains pending; programs decides the scoped-concurrency cases.
Cargo, bridge and rustc shim stay in witness/. CI gates from an empty store and seed
without cargo/rustc/node; inspect the nonblocking witness job and never constrain
language design for Rust. Accepted programs agree; delete the witness when it blocks
needed forms or ceases to find useful differences. Keep .rs until that decision.

## Real consumers (programs stream)

Programs work in /home/coder/workspace/lkjscript-programs-20261115 on programs;
land through fetch/rebase, bootstrap, four-worker gate and push HEAD:main.
Connector grants one numeric IPv4:port; connect yields an owned File stream, without DNS or retargeting.
Both executors validate entry descriptors; authority follows checked-IR effects and owning cleanup.
HTTP/1.1 persistence and pipelining retain exact framing; clients are lkjscript.
Profile before removing at most two costs; owned chunks replace per-byte assembly.
Use readiness for concurrency, processes for parallelism.
Readiness temporarily borrows owned handles into a kernel wait set without storing language views.
Keep explicit connection state, indexed deadlines and bounded work per dispatch.
Header and write deadlines do not extend when a hostile peer makes tiny progress.
The three scoped-thread cases are deleted; closure escape stays pending, with no new shared-memory ownership rules.
Keep compiler source within the stored seed's forms; never refresh seed here.
The independent interpreter helper operates actual descriptors without relay queues.
Journal acknowledges only after log fsync, a synced head file and atomic rename plus directory fsync.
Replay validates checksums and the committed boundary before incomplete-tail repair; one lock owns writes.
Stop claims SIGTERM for the execution lifetime; drain already admitted requests to a fixed deadline, sync and exit.
Signal the published target: native program PID or interpreted broker PID; requested consumes signals, readiness reports unread ones.
Entropy supplies kernel bytes; session keys are created private before writing, synced and atomically published.
Write tokens bind expiry and a random nonce through shared HMAC; a separate key-directory grant authorizes issuance.
JSON rejects ambiguous keys; authentication precedes body retention and is checked again before mutation.
Next match the documentation origin through GET-only observation; never modify the live unit or /opt.

## Authenticated reuse

--store grants a disposable Directory; --store-key grants a private 32-byte secret.
Keep the key outside every source/library/program read grant, including include_bytes!.
Key readers and compiler replacers are equally trusted; this is an explicit authority.
HMAC-SHA-256 binds exact compiler bytes, target, dependency identity and result bytes.
Producer-specific slots let different compiler generations coexist without eviction.
Only admitted originals or authenticated prior proofs skip body checks; revalidate shared structure.
Function keys bind types, destructors, signatures, return contracts and exact bodies.
Native fragments retain relocations; runtime/drop stubs remain ordinary emission.
Source-closure reuse records every actual read, including absent module candidates;
runtime dependencies are the bytes actually sealed. Closed tests on the embedded runtime
reuse returns and language traps; resource, host and model failures always rerun.
Authenticate large entries in bounded buffers before allocation, then recheck the reread.
Corruption, refused writes, disappearing stores and concurrent writers are misses.
No entries are committed; deleting the store or rotating its key is always safe.

## Resources and measurement

No resource policy exists except representation facts. Ask the kernel when
needed; never read RAM or rlimits to plan extents. Adapt to refusal and run under
any external limit leaving room for actual use. Report located allocation/stack
traps when requests cannot be satisfied. No default tool memory envelopes,
output caps or stack overrides. Use a generous normal-program watchdog and a
separate short allowance for generated/mutated nontermination, not a language rule.
Keep one heap and one stack exhaustion test; do not compare their route locations.
Recursion is fine until a real program fails. Use four workers per stream and leave
no background jobs or temporary files. Report --stats/bench timings, RSS, IR/code
sizes in status and commits. Measurements do not gate. Measure remaining gaps.

## Horizon and working policy

Range facts, IR queries, formatting, a checked-IR seed and a second target follow measured use.
Programs decides concurrency; do not build on pending scoped-concurrency cases.
Compiler owns main, seed, emission, driver/store, bench, CI and hash; programs owns
its separate worktree, consumers and new capabilities. Preserve its additions.
Only compiler refreshes seed, in its own commit. Do not move/reformat whole files.
Preserve .generation-one, other projects, services and data. No Python, WebAssembly,
Node execution host, releases, tags or backward compatibility. Delete replacements.
Fetch/rebase, bootstrap, four-worker gate, push, inspect CI; retry refused pushes.
No force pushes, ledgers or background jobs. Prefix gateway IDs with the stream.
Keep status within 60 lines (Programs owns at most 28), direction within 150.
