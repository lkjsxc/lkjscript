# Semantic compiler bootstrap

The target is semantic self-hosting. The current component is an intermediate
backend compiler with Rust source admission and normalized export still required.
It validates bounded LKJCI1 profile 3 and emits standalone Linux x86_64 ELF.
The current input covers pure scalar/byte function closures, buffers, List<I64>,
structural records, branches and recursion. A focused test reproduces the complete
native backend from normalized input; broader acceptance and `.lkja` emission
remain unfinished. See the
[contract, schema and dependency inventory](../../docs/spec/selfhosting.md).

`project/` owns accepted meaning. The `.lkjc` modules in this directory are
maintained authoring proposals. Changes pass ordinary graph admission before publication.
Tests reconstruct the complete compiler with bounded public transactions: first
typed interfaces, then each function body. They also replay the maintained bodies
on a private copy of the accepted graph and compare the complete normalized
compiler closure. Publication identities may change during body reconstruction.
The Rust test helper exports admitted input programs; the VM, disjoint reference
interpreter and standalone native ELF execute the same lkjscript compiler. Tests validate the
generated ELF and execute it in clean child environments, comparing independent
expected results/traps as well as the existing Rust backend.

Run the development bootstrap harness from the repository root:

```sh
bash tools/selfhost/verify.sh
```

This focused harness is not full acceptance. Run the repository's official
changed, fresh full and release-target checks after updating status and all
maintained source. Build the native binary filter with
`lkjscript --project tools/selfhost/project build --native-elf COMPILER --target compile-elf`;
it reads one LKJCI1 input from stdin and emits complete ELF bytes on stdout, with
diagnostics on stderr. An external launcher owns atomic output publication.
Milestone D requires both native backend closure and `.lkja` emission. Semantic
admission, phase-local regions and C2/C3 identity from the complete source graph
remain required for Tier 2.
