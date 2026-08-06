# Roadmap

## Now

1. **Remove remaining source-shape and HIR/ownership ceilings.** Delete the nesting, child, token,
   top-level, field, source-unit, HIR-expression, and related profile checks dependency-closed.
   Completion requires just-beyond-old-boundary and substantially larger positive programs,
   stack-safe user-controlled traversals, checked growth, and successful execution through the
   retained generic path.
2. **Separate resource policy from language validity.** Replace positional compiler count ceilings
   with unrestricted trusted-local compilation and explicit coarse input, memory, output,
   deadline, cancellation, and parallelism policy at untrusted request boundaries. The same
   program must exhaust under low policy and succeed unchanged under higher or unrestricted
   policy.

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
