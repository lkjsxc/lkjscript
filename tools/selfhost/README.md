# Semantic compiler bootstrap

The target is semantic self-hosting. The current component is an intermediate
backend compiler with Rust source admission and normalized export still required.
It validates bounded LKJCI1 profile 2 and emits standalone Linux x86_64 ELF.
The current input covers pure scalar function closures, branches and recursion.
It does not yet emit `.lkja` or compile itself. See the
[contract, schema and dependency inventory](../../docs/spec/selfhosting.md).

`project/` owns accepted meaning. `input.lkjc`, `flow.lkjc`, `machine.lkjc` and `elf.lkjc` are maintained
authoring proposals. Changes must pass ordinary graph admission before publication.
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
Full backend closure, `.lkja` emission, semantic admission, phase-local regions
and the C2/C3 fixed point remain required work.
