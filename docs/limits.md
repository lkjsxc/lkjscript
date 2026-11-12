# Size, count, depth and time constraints

These are constraints of current routes. Successful probe sizes are observations,
not acceptance limits; the language has no source-depth or execution-fuel quota.
Records are in `tools/measurements/` and `tools/scale/records/`.

| Route / stage | What can stop a valid program |
| --- | --- |
| All routes / host allocation | Address representation, available memory/swap, cgroup limits and OS reservation failures. The supported host uses 64-bit indices/extents; arithmetic is checked. |
| Rust bootstrap / compile | Recursive paths still use a finite native stack. The public `compiler::compile` adapter reserves a thread stack from host resources and retries rejected reservations before running the pipeline; existing explicit traversals remain. Complete independence from the native stack is not established. |
| K machine / execution | Heap cells and explicit frame/control/copy/drop stacks consume Rust process memory and retain some peak capacity. Some Rust collection allocation failures still abort instead of reporting R0006; exhaustion behavior is unmeasured. Absence of Node's function/body/16 GiB constraints does not establish acceptance bounded only by available memory. |
| WebAssembly / emit | Function/local indices, vector counts and section lengths are u32 even with memory64. Encoding overflow is diagnosed; addresses obey memory64 representation. |
| Node v24.18.1 / engine load | The engine accepts at most 1,000,000 total functions in one module. A successfully built million-function source chain emits 1,000,073 functions and is refused at `engine_load`. The outside kit also measured a 7,654,321-byte function-body limit; that body probe has not been repeated here. |
| Node v24.18.1 / memory growth | Linear memory stops at 16 GiB. The 50m-node boxed list constructs and completes its recursive sum, then traps R0006 during explicit drop at that extent, with about 21.07 GB process RSS high-water. The 35m-node whole program finishes. More free host memory does not remove this engine constraint. |
| Node / execution stack | Calls, copy helpers, generated recursive Clone/equality and recursive user destructors use the worker's finite native stack reservation. Structural destruction uses an explicit growing stack. No unbounded-depth claim follows from a large reservation. |
| Current allocation/layout / memory | Scalars still occupy eight-byte slots, including Vec<u8> elements. Released blocks are reused without splitting/coalescing. Growing cleanup buffers retain old capacities in that pool; fragmentation and byte expansion can exhaust memory early. Linear-memory capacity grows geometrically, with exact-required fallback if the preferred grow fails. |

The measured Node scalar probe reaches 10m non-tail calls. The largest complete
list program reaches 35m recursive calls; the 50m list finishes its recursive sum
before its later drop fails for memory. On the K route, the
million-function chain completes with 1,000,001 explicit frames and about 6.71 GB
whole-command process-tree peak RSS. These observations describe those shapes;
larger depths and exhaustion on other shapes remain unmeasured.

Worker reservation derives from physical memory plus swap, subject to observed
cgroup and address-space constraints. The large Node probes reserve
168,023,928,832 bytes of virtual stack address space on this host. Reservation,
committed pages, resident memory and WebAssembly's logical memory extent are
different quantities. Simultaneous workers reserve separate ranges and compete
for the same physical resources; reservation fallback does not add a source quota.

The K machine remains compiled by rustc, and a native backend has not landed.
Test time/output envelopes cancel process groups and do not define accepted
programs. Missing constructs and semantic restrictions are recorded in language
and status rather than counted as size constraints here.
