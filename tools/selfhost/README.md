# Semantic compiler bootstrap

The target is semantic self-hosting. The current component is an intermediate
backend compiler with Rust source admission and normalized export still required.
It validates LKJCI1 mode 0/profile 4 for standalone Linux x86_64 ELF and
mode 1/profile 5 for current `.lkja` artifacts. Artifact mode has no default
aggregate input-byte quota; its retained unit, map and value bounds are explicit.
The current input covers pure scalar/byte function closures, buffers, List<I64>,
structural records, branches and recursion. Focused tests reproduce the complete
backend in both output formats, with C2/C3 ELF and A1/A2/A3 artifact byte equality.
The artifact path serializes canonical
unit fields and carries the admitted source closure; source semantic checking
remains with Rust. See the
[contract, schema and dependency inventory](../../docs/spec/selfhosting.md).

The `source-read`, `source-shape`, `source-fields`, `source-expr` and `source-owner`
modules begin semantic-compiler admission with bounded current-format record
decoding. They are separate from the normalized backend entrypoint. Tests compare
decoded values on VM/reference/native, inspect every current source owner and
type, and reject malformed envelopes and payloads with independently recomputed
checksums. These readers do not yet resolve the source graph, check types or
ownership, or normalize source expressions.

`source-map`, `source-state`, `source-history` and `source-object` add the next
source-identity layer: fixed-width logical map roots, package semantic-state and
revision digests, current dependency/retirement records, and exact object-to-map
bindings. The tests compare these with independent current codecs and logical
root oracles. Matching these commitments does not establish reference resolution,
type correctness, custody or execution permission.

The `source-table`, `source-sort`, `source-edges`, `source-containment`,
`source-members`, `source-forest` and `source-scope` modules add complete
owner membership, reverse parent agreement, unique syntax containment, cycle
rejection and exact value-local lexical scope for the existing decoded profile.
Native source-inspection tests include both Rust-produced and self-host-produced
ELF validators. This remains a restricted E1 milestone; complete semantic admission,
source normalization and native phase reclamation retain their separate obligations.
See the [source-input contract](../../docs/spec/selfhosting-source-input.md) and
[complete active-source inventory](../../docs/decisions/20261022-active-compiler-profile.md).
The latter is a reproducible observation of every supplied owner and type, including
intrinsic substitutions, lexical scopes and direct/mutual recursion; it is not E2
admission or an E3 custody certificate.

`project/` owns accepted meaning. The `.lkjc` modules in this directory are
maintained authoring proposals. Changes pass ordinary graph admission before publication.
Tests reconstruct the complete compiler with bounded public transactions: first
typed interfaces one module at a time, then bounded groups of function bodies.
They also replay final definitions in bounded groups on a private copy of the
accepted graph and compare every canonical compilation unit's tables and payload,
plus the native backend closure. Publication identities may change during body
reconstruction.
The Rust test helper exports admitted input programs; the VM, disjoint reference
interpreter and standalone native ELF execute the same lkjscript compiler. Tests validate the
generated ELF and execute it in clean child environments. Emitted artifacts pass
the unchanged strict loader and execute on both evaluators. Independent expected
results and traps supplement compiler-engine agreement.

Run the development bootstrap harness from the repository root:

```sh
bash tools/selfhost/verify.sh
```

This focused harness is not full acceptance. Run the repository's official
changed, fresh full and release-target checks after updating status and all
maintained source. Build the native binary filter with
`lkjscript --project tools/selfhost/project build --native-elf COMPILER --target compile-elf`;
it reads one LKJCI1 input from stdin and emits complete ELF or `.lkja` bytes on stdout, with
diagnostics on stderr. An external launcher owns atomic output publication.
Milestone D requires both native backend closure and `.lkja` emission. Semantic
admission, phase-local regions and C2/C3 identity from the complete source graph
remain required for Tier 2.
