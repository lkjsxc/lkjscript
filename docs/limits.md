# Size, count, depth and time constraints

These are route constraints, not language quotas. Measurement sizes are not limits.

| Route / stage | What can stop a valid program |
| --- | --- |
| All current routes / allocation | Host address representation, available memory/swap and OS reservation failures. Indices/extents use the supported 64-bit host; arithmetic is checked. |
| Rust bootstrap / compile | Recursive passes use a memory-derived scoped-thread stack reservation in the public Rust adapter. A finite reservation can exhaust; complete stack independence is not established. Existing growable parser/type traversals remain. |
| WebAssembly / emit | Function/local indices, vector counts and section lengths are u32 even with memory64. Encoding overflow is diagnosed. Addresses obey memory64 representation. |
| Node v24.18.1 / engine load | 1,000,000 total functions per module. A million source functions plus helpers is refused here after successful compilation. The outside kit also measured a 7,654,321-byte function-body engine limit. |
| Node v24.18.1 / memory growth | The outside kit measured a 16 GiB memory64 growth ceiling. Physical memory availability does not remove it. |
| Node / execution | Source calls, copy helpers and recursive generated Clone/equality use the worker's finite memory-derived stack reservation. Structural drop traversal uses an explicit growing stack; recursive user destructor calls still use call frames. |
| Current allocation/layout / memory | Scalar elements currently occupy eight-byte slots; allocation reuses blocks without splitting/coalescing. Fragmentation and byte expansion can exhaust memory early. |

The worker reservation derives from physical memory plus swap, subject to observed
cgroup/address-space constraints. Reservation, committed memory and resident memory
are different; simultaneous workers reserve separate address ranges and compete
for the same physical resources. A large reservation does not prove unlimited depth.

No owned executor or native backend has landed. Node engine constraints are recorded
as properties of that route, not compiler failures. Test time/output envelopes stop
process groups only and do not define accepted programs. Missing language constructs
and semantic restrictions are tracked in language/status, not mixed into this table.
