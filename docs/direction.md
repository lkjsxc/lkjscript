# Direction — 2026-11-15

lkjscript is an AI-native general-purpose language in which agents directly
understand, query and change a program's semantic structure, with a foundation
for software at any scale, including Chromium. One self-reproducing binary,
checked IR as the single trust boundary and capabilities for every effect remain.
Text stays the stored, mergeable projection; storing structure itself requires the
owner's decision. Work on projection operations while that alternative is open.

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
Keep direct x86-64 instruction selection plain. Materialize source paths/lines on
cold failure paths; allocation/stack failures may use a static function location.
Whole-function scalar linear scan extends live intervals across blocks and loops;
aggregates/address-taken locals occupy frames. Volatile registers cross only modeled
non-clobbering instructions; save callee-saved registers only when selected.
Emit nothing for scalar drops, unit values or copies onto themselves. Share aggregate slots only for same-block, unborrowed-source final-use transfers into new identical-type locals; retain separate drop flags and original IR.
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
Measure self-built / witness opt=0 / witness -O and attribute transformations before optimizing; -O milestones are 4x, 3x and 2x, or a measured backend floor.
Discard steps below 10%; the last coalescing trial gave under 1% CPU and was discarded. Structure queries, transactions and scale precede speed unless compiler speed is the measured scale wall.
Scalar Clone lowers to Read. --inline-limit N copies small acyclic bodies and forwards dead temporaries.
Recheck changed bodies; exact original bodies under unchanged shared graphs retain their proof.
Keep the original interpreter/authority oracle and copied source locations; compare complete custom traps and reject the whole bad candidate.
Default zero: measured cold self-build admission costs exceed code gains; inlining stays explicit. The 3x witness target remains unmet.

## One executable and bootstrap
One static executable contains frontend, checker, backend, independent interpreter,
CLI and embedded source library/runtime. Stages exchange checked IR in memory;
text IR remains for output, .lkjir inputs, tests and mutation.
include_str!/include_bytes! read beneath the source Directory capability.
World derives process, clock and directory operations; arguments/streams are explicit.
Owned Files carry streams/executable authority. Interpreter values use cells/indices;
self-execution may emit checked native code, never unchecked mutation input.
Private result descriptors preserve full trap identities; bounded standalone lines use only granted Stderr/World, then granted Stdout.
Raw negation preserves NaN payload/sign; binary arithmetic canonicalizes NaNs.
Check all user bodies and demanded library bodies; check-library verifies every body.
The seed bootstraps without Rust/Node; require two consecutive current-emitter generations byte for byte, bridging stale emitters. Refresh after source/emitter changes.

## Gate and optional witness
Case files carry value/output/diagnostic/trap expectations. The native test driver
runs native and independent interpretation in processes, with details only on failure.
Seeded generators, direct IR mutation and disagreement reduction are lkjscript programs.
Run a fixed mutation slice every gate and a long campaign after each milestone;
report count/throughput and preserve every finding as a permanent case.
Reference examples are executable cases; language.md stays below 500 lines with
form, meaning, lowering, rejection and rewrite. Port language/IR contracts; drop host tests.
Holder liveness is conservative; its rejection names the lexical-scope rewrite.
borrowed_closure_escape remains pending; obsolete prose about three absent scoped-concurrency cases is not a contract.
Cargo, bridge and rustc shim stay in witness/. CI gates from an empty store and seed
without cargo/rustc/node; inspect the nonblocking witness job and never constrain
language design for Rust. Accepted programs agree; delete the witness when it blocks
needed forms or ceases to find useful differences. Source uses .lkj to identify lkjscript before reading; only actual Rust witness code uses .rs. Retain Rust shape; child.lkj/child/mod.lkj and program.lkj have no .rs alias or fallback.

## Program specimens (programs stream)
[Runtime demand table](runtime-demands.md) orders work. Applications are specimens; site, journal and desk remain unchanged gate cases. The compiler is the first consumer.
Programs work in /home/coder/workspace/lkjscript-programs-20261115 on programs; fetch/rebase, bootstrap, four-worker gate, push HEAD:main. Never refresh seed here.
Concurrency is process-isolated parallelism and readiness for idle work; no mutable shared address space or heap views. The [ordered pool library and map/channel specimens](../examples/process-workers.md) share only sealed immutable payloads.
The compiler pays separate heaps and checked-unit serialization; no single-process speedup is claimed. Seeded actor protocol exploration is distinct from a thread-race detector.
Connector grants one numeric IPv4:port, without DNS/retargeting. HTTP framing, indexed deadlines, bounded dispatch and fixed receive/write deadlines remain in the existing suites.
ReadDirectory/ReadFile are affine and cannot make writable/chmod-capable handles. Inspect/read one regular descriptor; atomic replacement preserves its coherent snapshot.
Journal commit/replay/authentication and Stop drain contracts remain in examples/journal; desktop/browser services are not extended or deployed.
Desk is host composition: state root, named board/handoff read roots and loopback listener. File, append, SSE, wake, units and handoff contracts remain in examples/desk.md; Claude owns its switch.
The documented site hardening and route/HEAD/same-port parity remain mandatory. Preserve live services and data.
Keep compiler source within stored-seed forms. The independent interpreter broker operates descriptors, without hidden relay queues.
build-child/image-authority bind exact ELF length/hash, target/entry/result/transfer ABI and checked capability slots; docs/child-image.md owns that wire contract.
A child imports only moved slots, never reopens roots or reacquires World. Report image identity and slots; deadline exit/output values, attached lifetime and parent-death cleanup are required.
Foreign code stays outside checked images: an attached confined worker may serve many requests. Absolute request deadlines poison/reap failed sessions; restart bounds are library.
Foreign grants snapshot the initial executable; read root and NullDevice are explicit. Dynamic runtime files live beneath R; report transformed PT_INTERP bytes honestly.
Landlock/seccomp/no_new_privs permit read/execute R, initial image and explicit streams; deny other writes, sockets, descendants and privilege regain. Reexec keeps confinement; close unpassed descriptors, check parent-death races, fail closed on unsupported hosts.
Outside-path metadata, clocks/entropy and mutable root contents remain observable; no snapshot/information-flow claim. In-image FFI would forfeit checked control flow, memory integrity and trustworthy authority accounting (examples/foreign-child.md).
Runtime admission uses the process form: a dedicated compiler producer carries checker/backend, receives framed IR while running and admits only an empty authority closure.
That producer presently declares World, including broad Process; it is an explicit trusted compiler, not a narrow public Admission value or a proof manufactured by an ordinary library struct.
Check every IR body with the same build-time checker, derive entry authority, emit transfer-only code, seal its memfd, then execute that exact descriptor with no capability slots.
There is no writable executable mapping in the producer. Each invocation returns scalar/trap/deadline evidence; reap before dropping its image. A child trap is contained and located; no in-image unwinding or unloading contract is invented.
The caller-facing authenticated Image/Channel form is separate from this producer's broad authority. Keep that gap visible in [runtime admission](runtime-admission.md), rather than call arbitrary File bytes admitted.
Long-lived object graphs use ordinary owning arenas and typed generational handles; temporary lookup views obey existing loans. Removed/reused/exhausted generations are checked; [specimen](../examples/object_graph/README.md).

## Authenticated reuse

--store grants a disposable Directory; --store-key grants a private 32-byte secret.
Keep the key outside every source/library/program read grant, including include_bytes!.
Key readers and compiler replacers are equally trusted; HMAC binds exact compiler bytes, target, dependency identity and result bytes.
Only admitted originals or authenticated prior proofs skip body checks; revalidate shared structure.
The scale ladder selects separately checked definition/instance bodies and checked dependency-interface identities; recursive type/destructor interfaces form components. The current global graph/pack remains a measured migration wall, not the chosen ten-million-line checking unit (bench/scale.md). Graph packs still bind types, destructors, signatures, contracts and exact bodies; native fragments retain relocations. Multi-entry builds share exact-namespace fragment snapshots and flush once; only ordinary builds retain whole images.
Front checkpoints reuse only stable source/arena spans, declarations, expanded string/byte literals, type spellings, catalogs and IR storage/call skeletons.
Freshly infer/construct changed free nongeneric bodies; other edits miss. Advance successful checkpoints and independently check all resulting IR.
Compact binary IR preserves exact tags and raw bits; decoding gives no checker authority.
One invocation shares actual source reads, including absent candidates; runtime dependencies are the bytes actually sealed.
Closed tests include fixed stdout/stderr captures only when both checked original/native effect closures permit those four operations; other host/resource/model outcomes rerun.
Fresh checked zero-authority ELF execution may reuse an exact stable-runner/image/input proof; compiler verdicts stay generation-bound.
Authenticate a fixed root binding producer/target/domain, length and digest before allocation; read one owned payload, require EOF and hash it once. VerifiedPayload preserves the exact authentication domain.
Default cap: 256 MiB of published compiler-owned bytes per Directory; evict oldest producer generations. Each worker-N gets one quarter of the base budget: 512 MiB for four workers, 768 MiB for eight.
Valid reads do not lock; nonblocking writers enforce the cap and commit accounting at invocation end. Closed runners release miss-cleanup locks before execution and reopen only after success. Corruption/refusal/concurrency/orphans are misses.
Program directories use literal SOURCE/ENTRY/GRANTS/CASES constants in program.lkj; signature authority is unchanged and missing/extra grants reject.
No entries are committed; deleting the store or rotating its key is always safe.

## Resources and measurement

No resource policy exists except representation facts. Ask the kernel when
needed; never read RAM or rlimits to plan extents. Adapt to refusal and run under
any external limit leaving room for actual use. Report located allocation/stack
traps when requests cannot be satisfied. No default tool memory envelopes,
output caps or stack overrides. Use a generous normal-program watchdog and a
separate short allowance for generated/mutated nontermination, not a language rule.
Keep one heap and one stack exhaustion test; do not compare route locations. Recursion is fine until a real program fails. Use four workers per stream and leave
no background jobs or temporary files. Report --stats/bench timings, RSS, IR/code
sizes in status and commits. Measurements do not gate. Measure remaining gaps.

## Horizon and working policy

Make structure directly operable: source claims and checked-instance facts are labelled, queries are bounded, edits are checked transactions, and comparisons use checked identities. Size measurements decide checking units; range facts follow measured use. A checked-IR seed and a second target still follow need. No mandatory formatter or fragment marker: retained diagnostic cases show no need for their frontend cost; delta prints accepted rewrites.
Processes-only parallelism is chosen; a single-address-space compiler pays IPC/unit-serialization costs until separately checked thread/allocator rules exist.
Compiler owns main, seed, emission, driver/store, bench, CI and hash; programs owns
its separate worktree, consumers and new capabilities. Preserve its additions.
Only compiler refreshes seed, in its own commit. Do not move/reformat whole files.
Preserve .generation-one, other projects, services and data. No Python, WebAssembly,
Node execution host, releases, tags or backward compatibility. Delete replacements.
Fetch/rebase, bootstrap, four-worker gate, push, inspect CI; retry refused pushes.
No force pushes, ledgers or background jobs. Prefix gateway IDs with the stream.
Keep status within 60 lines (Programs owns at most 28), direction within 150.
