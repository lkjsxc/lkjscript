# Representation and engine constraints

- Extents and indices use 64-bit values on the supported 64-bit host. Values and
  allocation arithmetic must fit their representation; overflow is an error.
- Allocation and virtual-memory reservation depend on the host's address space,
  available physical memory and swap. The Rust bootstrap reserves its native
  stack from physical memory plus swap. The parser and elaborator still use that
  stack for some nested constructs; fully explicit work stacks remain work.
- WebAssembly encodes vector counts, section lengths, function indices and local
  indices as u32, including when memory64 is used. Emission reports a format
  overflow rather than truncating. Memory64 addresses are 64-bit, with the
  format's page-count/address-space constraints.
- Node/V8 may reject modules or executions at its own memory, local-count,
  function-count or call-stack boundaries. This is the only execution engine
  currently implemented. Our machine and native backend have not landed yet.
- Test-runner time and output envelopes stop test processes only. They do not
  define which programs the compiler accepts.

There are no language-level quotas, step counters or depth thresholds. Host
allocation failure, I/O errors and externally terminated processes are observed
failures. No claim of unlimited practical capacity is made for the current Node
or native-bootstrap paths.
