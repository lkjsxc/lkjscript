# Linux x86-64 Native Backend

## Purpose

Select one backend for the future callable Linux x86-64 baseline JIT without
presenting the selection as an implemented JIT.

## Status

**Accepted Decision.** The future production baseline tier uses a
repository-owned x86-64 machine-code emitter. No production emitter, native code
object, executable-memory API, engine mode, or JIT is implemented by this
decision commit. The reference VM and observation-only **PLACEHOLDER** remain
the current runtime boundary.

## Decision

Verified typed SSA will lower through one owned Linux x86-64 encoder to bounded
native code objects. The encoder emits bytes directly; assembly text and an
external assembler are not production paths. It follows the versioned System V
AMD64 and runtime-call ABI defined by the callable-baseline cycle and does not
reinterpret syntax, HIR, or bytecode.

The backend boundary is:

```text
verified typed SSA
  -> backend-independent native lowering and ABI plan
  -> owned x86-64 byte encoder and relocations
  -> uninstalled native code image plus exact metadata
  -> safe lkjscript-sys W^X installation
  -> bounded native code object
```

Pure encoding, relocation descriptions, frame layout, and metadata construction
remain separate from host effects. Executable allocation, permission changes,
instruction-cache handling where needed, lifetime, and native entry remain
unsafe implementation details confined behind a safe `lkjscript-sys` API whose
safe inputs cannot install arbitrary bytes. The temporary experiment's raw
pointer calls and executable mappings are not that API.

The initial encoder is deliberately Linux x86-64 and baseline-tier scoped. It
must cover checked I64 arithmetic and division, F64 arithmetic/comparisons,
branches, loops, locals, direct generated calls, versioned runtime calls,
structured outcomes, and exact SysV stack alignment without relying on the red
zone. It is not an optimizing compiler and does not add another semantic IR.

## Selection Rule And Result

The thresholds were fixed in the ignored experiment protocol before timing:
correctness and observed W^X were mandatory; a generated-execution median
difference of at least 10% and more than twice the larger MAD selected the
faster candidate when secondary costs remained acceptable. Otherwise the mature
backend would win if its two-function compile/finalize median was at most 5 ms,
stripped binary at most 20 MiB, clean locked build at most 180 seconds and 2 GiB
peak RSS, Linux normal dependency closure at most 50, licenses and advisories
acceptable, and stack-map/replacement boundaries credible.

Both real spikes generated and called the same scalar kernel and passed exact
oracles. The owned encoder's 31-sample execution median was 48.406374 ms versus
119.422902 ms for Cranelift 0.134.2, so Cranelift took 2.467 times as long and
the owned candidate was 59.47% faster by elapsed-time reduction. The difference
was far above 10% and twice the larger MAD (0.566505 ms). The owned candidate is
therefore selected. Full commands, every retained sample, costs, dependency
review, and limitations are in
[Experiment N1](../vision/experiments.md#n1-linux-x86-64-native-backend-adopted).

The result is narrow rather than a universal compiler claim. The Cranelift
machine code repeatedly spilled and reloaded the loop-carried F64 value in this
kernel; broader language workloads could differ. The predeclared rule still
makes generated execution primary, and this measured material loss is not
overridden by implementation convenience.

## Rejected Production Candidate

Cranelift 0.134.2 is rejected for the production baseline backend selected by
this cycle. It passed correctness, emitted and called code through its real JIT
API, finalized observed `rw-p` mappings to `r-xp`, met every secondary budget,
and offers maintained instruction selection, register allocation, relocations,
and user stack-map machinery. Its measured generated execution was materially
slower under the predeclared rule.

It is conditionally retained only as a future replacement experiment if a
broader completed SSA workload suite reverses the runtime result, the owned
backend cannot satisfy exact semantics or maintenance gates, or an additional
architecture is accepted. Reconsideration requires a new measured decision; it
must not silently add Cranelift as a product dependency.

LLVM, an external assembler, dynasm-style text/macro assembly, and a second
bytecode-to-native semantic compiler remain rejected defaults.

## Dependency And Maintenance Consequences

The selected encoder adds no third-party Rust dependency. Avoiding a dependency
was not the selection reason; generated speed was. Repository ownership accepts
the maintenance cost of x86-64 encodings, register assignment/allocation,
relocations, SysV calls, frame layout, disassembly fixtures, and CPU-feature
policy. Each supported instruction family receives byte-level, disassembly,
ABI, malformed-relocation, and differential execution tests. Unsupported SSA
operations fail explicitly in forced mode.

The rejected locked Cranelift graph had 61 all-target third-party packages; the
current Linux graph had 38 normal runtime packages and five build-only packages.
Because runtime JIT compilation would call Cranelift in the running process,
those 38 are runtime product dependencies, not a build-time backend that can be
removed from the shipped runtime. The experiment found only permissive license
families and zero RustSec advisories, so licensing or a known advisory did not
decide the result.

## Safepoints And Stack Maps

Neither scalar timing kernel contained GC references, so both measured code
objects correctly had zero stack maps. Cranelift exposes user stack maps at
non-tail calls but requires the IR producer to identify and spill live GC
values; loop-backedge safepoints still need an explicit poll/call or later
backend work. The owned backend must instead make its complete register, spill,
outgoing-argument, and frame layout available to repository-owned metadata
construction.

Allocation-free scalar functions may execute first. Allocation-capable code is
forbidden until differential tests prove exact maps at every allocation,
collecting runtime call, GC poll, tier transition, deadline/cancellation poll,
and required loop backedge. A missing map is a compilation rejection, never a
conservative scan or unchecked execution. This cost is accepted explicitly and
is not claimed complete by the spike.

## Integration And Replacement Boundary

The backend consumes only verified typed SSA plus target/ABI configuration and
returns an uninstalled code image containing bytes, typed relocations, call
references, frame facts, safepoints/stack maps, source/trap/outcome mappings,
and size/work accounting. It does not own executable memory, tier state, the VM,
GC, host services, or CLI policy.

A replacement backend must implement that same narrow input/output contract and
pass the same differential, ABI, W^X, resource, and forced-native gates. Native
ABI and runtime-call identities must not expose owned-encoder internals. This
keeps later removal possible without preserving obsolete aliases or allowing two
production backends to reinterpret semantics.

## Evidence Boundary

This decision selects future implementation work only. The experiment used
ignored standalone crates, temporary third-party dependencies, raw FFI, and
unsafe native calls. Those artifacts are deleted after compact evidence is
recorded. No product Cargo manifest or lockfile changes, production unsafe API,
backend source, native engine flag, or callable JIT are part of this commit.
