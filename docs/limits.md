# Representation and engine constraints

- Extents and indices use 64-bit values on the supported 64-bit host. Values and
  allocation arithmetic must fit their representation; overflow is an error.
- Allocation and virtual-memory reservation depend on the host's address space,
  physical memory and swap. The Rust bootstrap grows its main stack on demand,
  preserving the host hard extent. Type parsing and type resolution now use
  explicit work frames, including generic arguments, aliases and nominal fields.
  Other expression/pattern parsing, elaboration and inference paths still use
  native recursion; complete compiler stack independence is not established.
  Embedding tests use an explicit 2 MiB stack, separately from the CLI shim.
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
- Immutable text literals share a deduplicated static region before the heap.
  Their extents, addresses, initial pages and Wasm data-section sizes are checked
  against actual target representations. Byte elements still occupy eight-byte
  slots. Text projection creates no temporary descriptor; checked UTF-8 conversion
  is linear in the byte length and String::as_str currently repeats that check.
  Static constraints on individual source reference leaves remain unsupported.
- The allocator reuses blocks without splitting or coalescing. Values commonly
  occupy eight-byte slots. Fragmentation and layout efficiency remain bootstrap
  implementation choices to improve from actual workloads.
- Complete generic witness/lifetime contracts and external view-mutation contracts
  remain incomplete; generic bodies are checked under declared rigid assumptions. Those semantic restrictions are distinct from capacity constraints.
  Structural interning avoids expanded-name growth and reference containment is
  solved once over the type graph. These do not establish linear whole-compiler
  complexity or eliminate every recursive traversal.
- Integer-pattern coverage sorts and merges intervals in the supported integer
  domain. It uses neither value enumeration nor a value beyond u64::MAX. This
  bounds the coverage algorithm's work by the number of pattern intervals; it is
  not a whole-match lowering or engine-capacity measurement.

Test-runner time, RSS and output envelopes stop test processes only. They do not
define accepted programs. No language quotas, step counters or depth thresholds
are implemented. Allocation failure, I/O failure, engine rejection and external
termination are distinct outcomes. Measurement sizes are not acceptance ceilings.
