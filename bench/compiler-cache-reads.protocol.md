# No-create cache-read observation protocol

Declared before constructing or timing either trial producer. This is a cache-read
contract repair and a modest cost observation, not an optimization milestone or
an explanation of the historical indexed 100k store cost.

Both producers are built with the same immutable emitter, SHA-256
71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7,
at the same compiler source path with explicit library/runtime selectors,
--inline-limit 64 and --no-store. Baseline source is
71e821dfcaaf6a7940ed67bb7d2d3a721c169922. The candidate changes only
store::cache_file and its explanatory comment. Construction wall/CPU/RSS and
paired three-component loads are recorded separately from observations.

The fixed specimen is one source file with main calling answer, whose scalar
result is 42. Both producers independently build/index it into distinct empty
stores at the same physical source path. Setup commands and all outputs are
retained; their cost is not a warm-read observation. Ordinary emitted output
uses --inline-limit 0 and must be byte-identical across producers.

Run exactly baseline,candidate,candidate,baseline for each of these three cases,
in this case order:

1. Unchanged ordinary build using that producer's already populated store.
2. Pinned outline query of main against the baseline snapshot/store, using the
   same historical producer identity for both executables.
3. Pinned meta query with a fixed valid 64-hex identity absent from a fresh empty
   store for each observation. Exit 1 and no stdout are required; files left in
   the owned store are counted after each invocation.

There are twelve observations, no retries, no adaptive order or repeat count,
and no file-cache reset. Every command uses a 4 GiB address-space limit and a
900 s watchdog; wall/CPU/RSS and loads are retained for setup and observations.
The caller source, runtime and options stay fixed. Warm images and query stdout
must match exactly. An unexpected result stops remaining observations and stays
in the evidence. No-store/specimen proof controls and focused contract tests are
separate from these timings.

One observation pair cannot isolate a causal syscall or instrumentation cost.
Producer-size/hash work and host/order variation remain in command time. No
10% improvement threshold or speedup claim is attached to this repair.
