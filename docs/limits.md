# Representation and engine constraints

- Extents and indices use 64-bit values on the supported 64-bit host. Values and
  allocation arithmetic must fit their representation; overflow is an error.
- Allocation and virtual-memory reservation depend on the host's address space,
  physical memory and swap. The Rust bootstrap grows its main stack on demand,
  sizing its soft extent from physical memory plus swap while preserving the
  host hard stack extent. Some parser/elaborator paths still use that stack;
  fully explicit work stacks remain work.
- WebAssembly vector counts, section lengths, function indices and local indices
  are u32 even with memory64. Emission reports format overflow rather than
  truncating. Memory64 addresses are 64-bit and obey its page/address-space format.
- Node/V8 has its own memory, module, local-count, function-count and call-stack
  limits. Recursive generated copy/drop helpers also use that call stack. Node is
  the only generated-code engine currently implemented; our machine and native
  backend have not landed.

Test-runner time, RSS and output envelopes stop test processes only. They do not
define accepted programs. No language quotas, step counters or depth thresholds
are implemented. Allocation failure, I/O failure and externally terminated
processes are reported as observed.
