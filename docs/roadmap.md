# Roadmap

## Now

1. **Remove remaining source-representation, HIR, ownership, and executable ceilings.** Flat
   lexer-token, children-per-form, top-level-form, source-nesting, per-file source-byte, aggregate
   source-byte, and source-unit quotas are removed. Parsing, source projection/identity/formatting,
   recursive source and HIR destruction, and the ordinary deep-expression analysis/lowering path
   are stack-safe through 8,192 nested forms on a 256 KiB native stack. Next widen or segment `u32`
   source positions, spans, and snapshot-local node indexes, then address
   product/HIR/ownership/memory-plan/SSA checks, remaining
   recursive type/trait/enum paths, and compact executable widths in dependency-closed slices.
   Completion requires just-beyond-old-boundary and substantially larger positive programs,
   checked growth, and successful execution through the retained generic path.
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
