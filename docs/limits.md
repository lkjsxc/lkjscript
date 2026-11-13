# Representation and host constraints

Successful sizes are observations, not acceptance limits. There is no source-depth,
execution-fuel or operation-count quota. Operator process budgets are independent
of the accepted language and can be changed explicitly.

| Stage | Actual constraint |
| --- | --- |
| Representation | Indices, extents and pointers use the supported 64-bit host representation. Checked arithmetic detects overflow. ELF and virtual-address formats remain finite. |
| Rust bootstrap | Recursive compiler paths use a scoped native thread stack provisioned from host resources. Compiler threads join before execution or stream capture. Rust/Cargo still build the source front end. The production K compiler source closure is complete. |
| Native code | Linux x86-64 static ELF; absolute call/branch addresses avoid a signed-32-bit code-distance ceiling. No external engine imposes function/local counts. |
| Native stack/heap | Startup reads physical memory, excluding swap, RLIMIT_AS and RLIMIT_STACK. It reserves the stack first and gives the heap the remaining extent after the image and actual startup mappings. Stack exhaustion is R0008; heap refusal/growth/exhaustion is R0006. Linux can still kill an overcommitted process under physical or cgroup pressure. Execution readings never affect artifact bytes. |
| K execution and batch capture | A separate worker inherits the caller's address-space limit or a tighter explicitly selected limit. The model receives a separate stack-byte extent, charging live frames, local location tables and root cells. Array/aggregate/container payloads remain heap allocations. Fallible paths report R0008/R0006; other Rust collection allocations may abort, reported by the parent as a host failure. |
| Layout/allocation | Scalars and elements have natural widths. Slice/text references are 16-byte data/count pairs; sized references are one word. Payload capacity rounds to a power of two, minimum 16 bytes; a separate 16-byte header records the size-class index and a free-list link. Capacity is 16 shifted by the class; release uses the stored class directly. Tip blocks grow in place. Other growth allocates, copies, then releases the old block. Freed blocks are reused within their class; there is no coalescing. |

## Operator controls

With no setting, native children and K execution/batch workers inherit the
caller's Linux address-space limit. There is no additional runner memory quota.
`LKJ_RUN_MEMORY_BYTES=<bytes>` selects a tighter extent; `unlimited` delegates
to inherited controls. An explicit setting never raises an inherited soft limit.

`LKJ_RUN_STACK_BYTES=<bytes>` independently tightens the inherited stack extent.
A finite inherited RLIMIT_STACK soft limit is always authoritative. If it is
unlimited and no finite setting is requested, native startup and the model use
a 64 MiB provisioning fallback. Native reservations round down to 4 KiB pages.
To increase an inherited finite limit, the caller raises it explicitly before
launching the runner. A small K model extent does not shrink the Rust worker's
own OS stack.

Rust compiler work uses scoped threads, joined before model execution or native
stream capture. The temporary compiler stack comes from physical/cgroup memory;
under a finite address-space limit it reserves half the remaining address space.
Execution therefore does not retain that compiler stack as live work. Libc can
cache some thread mappings, so the actual reservation is measured separately.

The Rust execution worker defaults to one glibc allocation arena, honoring an
explicit MALLOC_ARENA_MAX. It has one active compiler or execution thread at a
time; sharing the process arena avoids secondary aligned arena reservations
under a tight address-space budget. This changes allocation overhead, not the
heap capacity or the operator budget. Static model error messages are borrowed,
so constructing an R0006/R0008 message needs no allocation. Infallible Rust
allocations can still abort and remain classified as host failures.

Standalone ELFs read the caller's Linux limits directly; they do not read the
runner's environment settings. For example:

```sh
prlimit --as=1073741824 --stack=67108864 target/program.elf
LKJ_RUN_MEMORY_BYTES=1073741824 target/release/lkjscript run program.rs
prlimit --stack=67108864 env LKJ_RUN_STACK_BYTES=33554432 \
    target/release/lkjscript run program.rs
```

Native startup chooses the smaller of physical memory and RLIMIT_AS. It subtracts
the selected stack and page-rounded ELF mappings, then reserves the stack first.
The heap attempts the remaining extent, rounded to 64 KiB. On ENOMEM only, it
retries one heap page smaller until the required initial heap cannot fit. This
lets the kernel account for variable argument/environment, initial Linux stack
and kernel mappings, without reserving a percentage of every budget.

Raw heap commitment tries geometric growth, then the exact required extent.
Interleaved live allocations can prevent tip growth; released size classes do
not coalesce. These are allocator costs, not source rejection rules.

Generated test executables default to 2 GiB; rustc compilation/linking defaults
to 8 GiB. LKJ_TEST_MEMORY_BYTES overrides either budget. Deliberately deep
recursion tests select their stack extent explicitly. Node orchestration is
excluded from the address-space setting because V8 reserves a large virtual
cage. Node no longer executes lkjscript programs. Timeout and output envelopes
cancel test process groups; they never define language semantics.

`bash tools/host-boundary.sh target/release/lkjscript` checks build-resource byte
identity, K-text byte identity, inherited/clamped controls, located native/K
resource failures, joined compiler threads and bounded batch capture. Resource exhaustion can occur at
different source calls in the two representations; resource codes are compared
separately from their recorded locations.

The 2026-11-13e observation has plain run succeeding on a 20 MB repeated vector,
100 MB of byte pushes, and 200 MB of stdin. The latter two also succeed with
explicit 256 MiB and 512 MiB address-space settings respectively. These are
observations, not acceptance thresholds. See
observations/2026-11-13e-memory.json for inputs, commands, values and measured RSS.

The follow-up CI investigation is recorded in
observations/2026-11-13e-host-resources.json. With the same 256 MiB process and
1 MiB model-stack budgets, 24 direct/mutual recursion cases report located R0008.
The idle model observation drops from 200,576 to 17,096 KiB mapped address space.

Virtual reservation, committed memory, RSS and logical heap size are different
measurements. Multiple processes compete for physical memory even after mappings
succeed. Historical capacity records remain in Git and are not guarantees.

## File and stream boundary

World supplies arguments as owned byte vectors and explicitly derives stream and
directory capabilities. Directory opens use Linux openat2 with RESOLVE_BENEATH
and RESOLVE_NO_MAGICLINKS; kernels without that operation return ENOSYS. No path
resolver fallback weakens containment. Errors are positive Linux errno values in
source Results; raw read/write leaves use negative errno.

Native artifacts preserve inherited descriptor state. A Rust bootstrap adapter
records genuinely closed standard descriptors before Rust initializes them and
restores that state once. Program stderr, requested exit status and trap control
are separate observations in native/K batch and the controlled Rust oracle.
`node tools/world-io.mjs` compares bytes, status, file effects, directory escapes,
handle cleanup and closed/broken streams across the four execution hosts.
