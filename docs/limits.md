# Representation and host constraints

Successful sizes are observations, not acceptance limits. There is no source-depth,
execution-fuel or operation-count quota. Operator process budgets are independent
of the accepted language and can be changed explicitly.

| Stage | Actual constraint |
| --- | --- |
| Representation | Indices, extents and pointers use the supported 64-bit host representation. Checked arithmetic detects overflow. ELF and virtual-address formats remain finite. |
| Rust bootstrap | Recursive compiler paths use a native thread stack provisioned from host resources. Rust/Cargo still build the compiler; source closure is unfinished. |
| Native code | Linux x86-64 static ELF; absolute call/branch addresses avoid a signed-32-bit code-distance ceiling. No external engine imposes function/local counts. |
| Native stack/heap | Startup reads physical memory, excluding swap, and RLIMIT_AS. Reservations are execution inputs and never build inputs. Stack exhaustion is R0008; heap allocation/growth/refusal is R0006. Both report a source line. Linux can still kill an overcommitted process under physical or cgroup pressure. |
| K execution | A separate worker bounds the Rust process. Fallible explicit frame/heap paths report R0008/R0006. Other Rust collection allocations may abort; the parent reports a host-budget failure without a source location. |
| Layout/allocation | Scalars and elements have natural widths. Slice/text references are 16-byte data/count pairs; sized references are one word. Blocks round payload plus a 16-byte header to power-of-two classes, minimum32 bytes. Released blocks are reused within their class; there is no coalescing. Growth tries geometric capacity, then exact need. |

## Operator controls

Native children default to 256 MiB RLIMIT_AS; K workers to 512 MiB. Tighter inherited
limits are preserved. `LKJ_RUN_MEMORY_BYTES=<bytes>` selects a different extent;
`LKJ_RUN_MEMORY_BYTES=unlimited` delegates to inherited Linux controls. Standalone
ELFs use their caller's controls directly, for example:

```sh
prlimit --as=1073741824 target/program.elf
LKJ_RUN_MEMORY_BYTES=1073741824 target/release/lkjscript run program.rs
```

Generated test executables default to 2 GiB; rustc compilation/linking defaults
to 8 GiB. LKJ_TEST_MEMORY_BYTES overrides either budget. Node orchestration
is excluded from this address-space setting because V8 reserves a large virtual
cage. Node no longer executes lkjscript programs. Timeout and output envelopes
cancel test process groups; they never define language semantics.

`bash tools/host-boundary.sh target/release/lkjscript` checks build-resource byte
identity, K-text byte identity and located native/K resource failures. Large
capacity probes request their budget explicitly and remain separate from the gate.
Virtual reservation, committed memory, RSS and logical heap size are different
measurements; multiple processes compete for physical memory even after mappings
succeed. Historical capacity records remain in Git and are not current guarantees.
