# Runtime code admission: the process form

The first specimen is a running compiler producer that receives lkjscript IR,
checks an empty authority ceiling, emits native code and calls it in an attached
child. It is not a JIT, an in-image loader or a compiler for another language.
The request is delivered only after the producer announces readiness.

## Authority and trust

`src/programs_admission_main.lkj` carries the same parser, IR checker and native
backend as the build-time compiler. Its entry honestly declares **World**,
including broad Process. The startup line names the exact producer SHA-256,
that host authority, the empty admitted authority and process execution.

This is an explicit trusted compiler producer, like the ordinary compiler, not
a narrow Admission primitive that an arbitrary untrusted frontend can certify.
An ordinary private source struct cannot establish executable admission at the
IR boundary. The current specimen does not provide a general File-to-Image cast.
The later caller-facing Image/Channel API must separately bind the exact trusted
admitting producer and its reply provenance before any public loader is claimed.

Each request uses `ir_text::read_located` and `check::check`, exactly as build-time
IR admission. Every supplied body is checked. The selected `probe` must have
no entry slots and no reachable effect, including owning cleanup and destructors.
The native child manifest is derived from that checked program. A requested
Stdout slot refuses even when the body never writes it.

The producer emits a transfer-only child, writes one temporary kernel object,
marks it executable and applies all file seals. The same owned descriptor reaches
exec; no image path or mutable alias is reopened after validation. The child
startup closes ambient streams and undeclared descriptors and receives only the
private result descriptor. No writable executable pages enter the producer.

## Call, end and unload

Input is a stream of little-endian u64 lengths followed by exact UTF-8 IR bytes.
Zero length or clean EOF ends the stream; a truncated frame ends the producer.
There is no fixed language input-size cap. Allocation refusal remains a located
resource failure, and deployment policy belongs to the eventual caller.

Each request returns one JSON-quoted outcome line: refusal, scalar result bits
with result kind, located trap, deadline, or abnormal process termination.
The caller supplies a nonzero per-invocation deadline as an explicit argument.
The producer finishes and reaps one child before consuming the next request.
Only then are executable/capture descriptors dropped. There is no borrowed code
pointer or live stack frame to invalidate, and no in-image unwind is claimed.

A trap record is decoded against the same checked request and runtime source
maps; raw child text is not accepted as a located runtime diagnosis. A refused
request, trapped child or timed-out child does not end the producer. Its next
valid request is admitted and executed normally.

## Retained proof

`bash tools/programs-admission-test.sh [COMPILER]` creates fresh FIFOs/state,
starts the producer, waits for readiness and only then supplies IR. It proves
value 42, refusal of a requested output capability, a trap at
`runtime-trap.lkjir:41`, nontermination under a deadline, and subsequent recovery.
Six direct IR mutants are separately refused by the build-time checker and the
already-running producer. All test-owned processes are reaped and files removed.

The initial focused run took 0.735 s (load 33.28 -> 33.28), excluding producer
compilation. Its producer image was 11,838,568 bytes; changes to the compiler or
runtime can change that size. These numbers describe this specimen, not a
per-request admission benchmark or a promised bound.
