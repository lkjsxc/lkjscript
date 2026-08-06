# Roadmap

## Now

1. **Remove remaining source-representation, HIR, ownership, and executable ceilings.** Flat
   lexer-token, children-per-form, top-level-form, source-nesting, per-file source-byte, aggregate
   source-byte, and source-unit quotas are removed. Parsing, source projection/identity/formatting,
   recursive source and HIR destruction, and the ordinary deep-expression analysis/lowering path
   are stack-safe through 20,000 nested forms and 20,001 HIR expressions on a 256 KiB native stack.
   Ownership's aggregate expression pre-scan and memory planning's expression admission are
   removed; checked expression work remains telemetry. Next widen or segment `u32` source
   positions, spans, and snapshot-local node indexes, then address
   product/HIR/memory-plan-table/SSA checks and remaining recursive type/trait/enum paths.
   Trusted bytecode validation no longer has total encoded-byte, table-entry, metadata-byte,
   constant-data-byte, or cleanup-node/range count admission; a boundary-local limited validator
   checks only total artifact bytes and has no finite default. Byte-sized function arity, call
   argument, local, cleanup-local, and executable-place widths are
   removed: one fixed-`u64` index/pair format now executes 300 parameters, arguments, live lexical
   locals, and direct VM place 299, with checked decode and automatic VM fallback; a larger stress
   case executes 1,024 parameters, arguments, and lexical locals. Branch targets use the same fixed
   `u64` layout, and the separate 65,535-byte per-function validity limit is removed. Production
   coverage executes a 115,754-byte main with 1,024 owned arguments and a generated function whose
   branch target exceeds 65,535 bytes; automatic mode retains VM fallback when native expansion is
   ineligible. SSA ownership work and retained state-cell quotas are removed; generated coverage
   verifies 44,000 owned parameters and places,
   132,000 state cells per block, 264,000 cells under the former aggregate retained-state
   accounting across propagation, and exact cleanup. SSA and bytecode cleanup now use wide,
   backward-only hash-consed node arenas with segmented roots; a 300-owned-parameter/argument source
   publishes and runs with 315,450 logical actions represented by 1,200 physical nodes. Next remove
   retained `u16` constants, globals, product/enum/structural tables and descriptors, plus byte-sized
   product fields and enum substitutions. Bytecode links, call-witness offsets, cleanup range
   offsets, and cleanup roots are already `u64`, while physical cleanup-node/range counts remain
   unrestricted by the removed general bytecode table and metadata admission. Completion requires
   just-beyond-old-boundary and substantially larger positive programs, checked growth, and
   successful execution through the retained generic path.
2. **Complete resource-policy separation.** Trusted compiler profile/ledger and source-byte
   admission are removed. The untrusted Semantic Source boundary now supplies only an aggregate
   source-byte loader policy and applies it to staged transactions before publication. Continue
   replacing remaining validity-changing implementation ceilings with unrestricted local
   compilation and explicit coarse input, memory, output, deadline, cancellation, and parallelism
   policy at untrusted request boundaries. The same program must exhaust under low policy and
   succeed unchanged under higher or unrestricted policy.

## Next

1. Measure VM-only, automatic baseline-native, and optimizing configurations on scalar, call,
   aggregate, byte, cleanup, error, and host workloads. Select one production path and delete the
   losing product-tier surface.
2. Remove duplicated in-process witness and digest reconstruction while retaining artifact,
   process, capability, path, and executable-memory validation.
3. Implement the first semantic-program vertical described in [`source-model.md`](source-model.md),
   including direct compilation without text rendering and reparsing.
4. Add dependency-aware incremental name, type, effect, ownership, and lowering queries after
   measuring a small custom cache against a mature query framework.

## Later

1. Integrate immutable semantic snapshots, compilation caches, and warm runtime state with the
   daemon through narrow interfaces.
2. Profile representative applications and add native specialization, tiering, OSR, or
   deoptimization only where measured benefit exceeds implementation cost.
3. Expand package, service, database, network, GUI, web, and game capabilities through the semantic
   model, capability system, and selected production runtime.
