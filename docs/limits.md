# Representation and engine constraints

- Extents and indices use 64-bit values on the supported 64-bit host. Values and
  allocation arithmetic must fit their representation; overflow is an error.
- Allocation and virtual-memory reservation depend on the host's address space,
  physical memory and swap. The Rust bootstrap grows its main stack on demand,
  preserving the host hard extent. This is a CLI accommodation: parsing and
  elaboration of nested generic types can still overflow a small embedded-thread
  stack. Depth 256 nested Box types reproduced both stage failures on a 2 MiB
  thread; explicit type-parser and elaborator work frames are the next repair.
- WebAssembly vector counts, section lengths, function indices and local indices
  are u32 even with memory64. Emission reports format overflow rather than
  truncating. Memory64 addresses obey their page/address-space encoding.
- Source/K locals are allocated by lifetime. A finite physical-local pool spills
  additional live values into a growing memory64 frame, so exhausting that pool
  does not reject a source program. Address-taken slots also use the frame.
  Parameter counts and remaining Wasm structural limits are separate constraints.
- Structural/custom cleanup walks an explicit growable stack. Its automatic walk
  no longer consumes one engine call frame per owned child. Copy/clone helpers and
  explicit recursive source calls still use the Node/Wasm call stack. A custom
  destructor that itself makes recursive source calls retains that limitation.
- Node/V8 has its own memory, module, function, parameter and call-stack limits.
  Node is the only generated-code engine currently implemented; an owned machine
  and native backend have not landed.
- The allocator reuses blocks without splitting or coalescing. Values commonly
  occupy eight-byte slots. Fragmentation and layout efficiency remain bootstrap
  implementation choices to improve from actual workloads.
- Generic definition/witness contracts and external view-mutation contracts remain
  incomplete. Those semantic restrictions are distinct from capacity constraints.
  Structural type interning avoids expanded-name growth; it does not remove all
  recursive compiler traversals or establish linear whole-compiler complexity.

Test-runner time, RSS and output envelopes stop test processes only. They do not
define accepted programs. No language quotas, step counters or depth thresholds
are implemented. Allocation failure, I/O failure, engine rejection and external
termination are distinct outcomes. Measurement sizes are not acceptance ceilings.
