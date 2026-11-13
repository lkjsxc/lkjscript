# Representation and host constraints

Successful probe sizes are observations, not acceptance limits. The language
has no source-depth, execution-fuel or operation-count quota. Commands and seeds
belong in status; detailed output is disposable under `target/`.

| Route / stage | What can stop a valid program |
| --- | --- |
| All routes / host allocation | Address representation, available memory/swap, cgroups and OS reservation failures. Indices and extents use the supported 64-bit host; arithmetic is checked. |
| Rust bootstrap / compilation | Recursive paths still use a finite native thread stack reserved from observed host resources. Rejected thread reservations can be retried before compilation starts. This remaining bootstrap adapter is separate from emitted native execution. |
| Native / code and addressing | Linux x86-64 virtual-address and ELF/file representations, OS mappings and available memory. Calls and branches use absolute addresses; there is no Node function-count ceiling or accidental signed-32-bit code-distance ceiling. |
| Native / stack and heap | The host supplies reservation extents. Each prologue checks the stack end; memory operations and growth check the heap extent. Mapping/growth refusal or checked reservation exhaustion traps R0006 without retrying guest execution. Linux may still kill an overcommitted process during page faults under system-wide OOM pressure. |
| K machine / execution | Its typed heap and explicit frame/control/copy/drop stacks consume Rust process memory and retain some peak capacity. Some Rust collection allocation failures still abort rather than reporting R0006. Exhaustion remains unmeasured. |
| WebAssembly / encoding | Function/local indices, vector counts and section lengths are u32 even with memory64. Encoding overflow is diagnosed; addresses obey memory64 representation. |
| Node v24.18.1 / engine load and memory | The optional route retains engine ceilings: one million total module functions and 16 GiB linear memory. Historical probes reached those boundaries. The engine body-size observation from the outside kit has not been repeated here. |
| Node / call stack | The thin host uses Node's ordinary stack. The reservation worker and its retries were deleted after native capacity passed. Old deep-Node runs do not describe this host. |
| Shared allocation/layout | Scalars and collection elements use natural widths; Vec<u8> is contiguous bytes. Blocks round payload plus a 16-byte header to power-of-two size classes, with a 32-byte minimum. Released blocks are reused within their classes; unused classes and old growing-buffer capacities can retain memory. There is no coalescing between classes. Growth tries geometric capacity, then the exact required extent. |

The new native observations are a million-function chain, 100 million non-tail
recursive calls and a complete 50-million-node construct/sum/drop list. Peak
execution RSS was respectively 1.017, 15.999 and 26.189 GB. The list's destructor
walker uses explicit growing source-owned storage; its recursive sum exercises
the native call stack. These shapes do not establish every possible depth or
allocation pattern.

Earlier observations, before the source-runtime/native transition: the K
million-function chain completed with 1,000,001 explicit frames and about 6.71 GB
whole-command process-tree peak RSS. The former Node worker completed a
35-million-node list; its 50-million-node list finished recursive sum and then
trapped R0006 during destruction at the 16 GiB memory extent. These are historical
comparisons, not fresh tests of the current compiler or thin Node host.

Reservation, committed pages, resident memory and logical memory extents are
different quantities. Multiple processes compete for physical resources even
when virtual reservations succeed. The current native host derives extents from
physical memory/swap, cgroups and address-space limits; it does not turn a tested
size into a source acceptance bound.

Test time/output envelopes cancel process groups and never define accepted
programs. Missing language constructs and semantic restrictions belong in
language/status rather than being counted as size constraints.
